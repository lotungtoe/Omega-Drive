use std::path::PathBuf;
use std::sync::atomic::AtomicU16;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::time::sleep;
use tracing::{error, info};

use omega_drive_core::services::DefaultDebugLogger;
use omega_drive_discord::DiscordBackupGateway;
use omega_drive_gateway::player::cache::ByteCache;
use omega_drive_gateway::player::singleflight::PartSingleFlight;
use omega_drive_player::{IdxCache, PlayerContext};

use crate::app::event_emitter::TauriEventEmitter;
use omega_drive_player::runtime::PlayerRuntime;
use crate::app_wiring::app_runtime::AppState;
use crate::db::repos::{
    DbDownloadJobRepository, DbFileRepository, DbFolderRepository, DbUploadJobRepository,
};
use crate::features::backup::BackupService;
use crate::features::drive::DriveService;
use omega_drive_db::db_executor::DbExecutor;
use omega_drive_gateway::provider::backup_service::BackupService as BackupServiceTrait;
use crate::providers::install::{
    build_provider_runtime, install_builtin_providers, prepare_builtin_provider_state,
    render_builtin_bot_env_template, ProviderInstallContext,
};
use crate::providers::runtime::ProviderRuntime;
use omega_drive_core::ports::app_context::NoopAppContext;
use omega_drive_engine::integrity::EngineIntegrityService;
use omega_drive_engine::zip_utils::EngineZipService;
use omega_drive_gateway::core::backup::Op;
use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::core::engine_context::EngineContext;
use omega_drive_db::{Db, DbWriteQueue, ReadDbPool};
use omega_drive_gateway::core::types::new_sender_map;

use super::bridge::{ensure_video_bridge_child, run_video_bridge_process};
use super::paths::{resolve_base_dir, resolve_startup_tenant, resolve_tenant_db_path};
use super::tauri_app;

/// Parse command-line arguments to determine whether to run in Video Bridge mode.
/// This mode runs a separate child process dedicated to video streaming.
struct VideoBridgeProcessArgs {
    port: u16,
    parent_pid: Option<u32>,
}

/// Probe a free port starting from `start`, used as the bridge listening port.
fn pick_bridge_port(start: u16) -> u16 {
    for offset in 0..100u16 {
        let port = start + offset;
        if std::net::TcpListener::bind(
            std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        )
        .is_ok()
        {
            return port;
        }
    }
    start
}

fn parse_video_bridge_args() -> Option<VideoBridgeProcessArgs> {
    let mut args = std::env::args().skip(1);
    let mut bridge_mode = false;
    let mut port = None;
    let mut parent_pid = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--video-bridge" => bridge_mode = true,
            "--video-bridge-port" => port = args.next().and_then(|value| value.parse::<u16>().ok()),
            "--parent-pid" => {
                parent_pid = args.next().and_then(|value| value.parse::<u32>().ok());
            }
            _ => {}
        }
    }

    if bridge_mode {
        port.map(|port| VideoBridgeProcessArgs { port, parent_pid })
    } else {
        None
    }
}

pub async fn run() {
    let base_dir = resolve_base_dir();
    info!(" Data directory: {}", base_dir.display());

    if let Err(e) = std::fs::create_dir_all(&base_dir) {
        eprintln!(
            "Error: Cannot create data directory '{}': {e}",
            base_dir.display()
        );
        std::process::exit(1);
    }
    // Provider-specific temp cleanup from previous crashes (best-effort)
    tokio::task::spawn_blocking(|| {
        crate::providers::cleanup_provider_temp_files(std::time::Duration::from_secs(
            24 * 60 * 60,
        ));
    });

    if let Some(bridge_args) = parse_video_bridge_args() {
        if let Err(e) =
            run_video_bridge_process(base_dir, bridge_args.port, bridge_args.parent_pid).await
        {
            eprintln!("Error: Video bridge child process failed: {e}");
            std::process::exit(1);
        }
        return;
    }

    let env_path = base_dir.join("bot.env");

    // Create sample provider config file if not exists
    if !env_path.exists() {
        let _ = std::fs::write(&env_path, render_builtin_bot_env_template());
    }

    // Load environment variables from bot.env
    dotenvy::from_path(&env_path).ok();

    // If DEBUG is set, enable backtrace for easier panic debugging
    if std::env::var("DEBUG").is_ok() {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    let cfg = Arc::new(RwLock::new(omega_drive_core::config::load_config(
        &base_dir,
        &crate::providers::config::builtin_provider_config_descriptors(),
    )));
    let feature_logs = Arc::new(crate::app_wiring::infrastructure::feature_log::init_tracing(
        &cfg.read().expect("cfg RwLock"), &base_dir,
    ));
    info!(" Data directory: {}", base_dir.display());

    // Check Deno runtime on startup if DEBUG is enabled
    if std::env::var("DEBUG").is_ok() {
        let deno = omega_drive_gateway::updater::path::deno_path();
        if deno.exists() {
            info!("[Startup] Deno runtime detected at: {}", deno.display());
        } else {
            error!(
                "[Startup] Deno runtime NOT found! Path: {}",
                deno.display()
            );
        }
    }

    let thumbnail_dir = base_dir.join("thumbnails_cache");
    let _ = std::fs::create_dir_all(&thumbnail_dir);
    let default_tenant = resolve_startup_tenant(&base_dir);
    let _ =
        omega_drive_core::tenant_registry::persist_active_tenant(&base_dir, &default_tenant);

    let prepared_providers = match prepare_builtin_provider_state(&base_dir, &default_tenant).await
    {
        Ok(prepared) => prepared,
        Err(err) => {
            eprintln!("Error: Provider setup failed: {err}");
            std::process::exit(1);
        }
    };

    // --- Initialize SQLite — File structure & metadata storage ---
    let db_path = resolve_tenant_db_path(&base_dir, &default_tenant);

    // Open 2 separate connections per user request: "separate DB into one for reading and one for writing"
    let db_write_conn = match Db::open(&db_path) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Error: Cannot open SQLite database (Write): {e}");
            std::process::exit(1);
        }
    };
    if let Err(err) =
        omega_drive_db::tenant_meta::upsert_tenant_meta(db_write_conn.conn(), &default_tenant)
    {
        eprintln!("Could not persist tenant metadata into SQLite: {err}");
        std::process::exit(1);
    }
    let drive_db_read = match ReadDbPool::open(&db_path, ReadDbPool::recommended_size()) {
        Ok(pool) => Arc::new(pool),
        Err(e) => {
            eprintln!("Could not open SQLite drive read pool: {e}");
            std::process::exit(1);
        }
    };

    let db_write = Arc::new(DbWriteQueue::new(db_write_conn));
    let db_read = Arc::clone(&drive_db_read);

    omega_drive_core::services::init_file_classifier();
    omega_drive_db::services::init(
        omega_drive_db::services::DbServices::new(
            Box::new(omega_drive_core::services::DefaultFileTypeClassifier),
            Box::new(omega_drive_core::services::DefaultExtensionNormalizer),
            Box::new(omega_drive_core::services::DefaultSystemProfileProvider),
            Box::new(omega_drive_core::services::DefaultMediaParser),
        ),
    );

    // --- Cloud Backup Service ---
    let backup_service = Arc::new(BackupService::new(base_dir.clone()));

    // --- Initialize Event Bus (Internal Event Bus) ---
    let event_bus = Arc::new(omega_drive_gateway::core::events::EventBus::new());

    let install_ctx = ProviderInstallContext {
        base_dir: base_dir.clone(),
        db_read: Arc::clone(&db_read),
        db_write: Arc::clone(&db_write),
        prepared: prepared_providers,
        event_bus: Arc::clone(&event_bus),
        tenant: default_tenant.clone(),
    };
    let install_results = match install_builtin_providers(install_ctx).await {
        Ok(results) => results,
        Err(err) => {
            eprintln!("Error: Provider initialization failed: {err}");
            std::process::exit(1);
        }
    };

    omega_drive_core::config::print_config_summary(&cfg.read().expect("cfg RwLock"));

    // --- DATABASE WATCHDOG + BACKUP HOOK ---
    {
        let eb = Arc::clone(&event_bus);
        let db_watch = Arc::clone(&db_write);
        let backup = Arc::clone(&backup_service);
        tokio::spawn(async move {
            let db = db_watch.lock().await;
            db.conn().update_hook(Some(
                move |action: rusqlite::hooks::Action, _: &str, table: &str, row_id: i64| {
                    if table == "files" {
                        eb.emit(omega_drive_gateway::core::events::OmegaEvent::FilesTableChanged);
                    }
                    // Backup P1/P2 mutations
                    let priority = match table {
                        "folders" | "tenant_meta" => Some(1u8),
                        "upload_profiles"
                        | "upload_profile_rules"
                        | "upload_jobs"
                        | "download_jobs" => Some(2),
                        _ => None,
                    };
                    if let Some(pri) = priority {
                        let action_str = match action {
                            rusqlite::hooks::Action::SQLITE_INSERT => "insert",
                            rusqlite::hooks::Action::SQLITE_UPDATE => "update",
                            rusqlite::hooks::Action::SQLITE_DELETE => "delete",
                            _ => "unknown",
                        };
                        let seq = backup.next_seq();
                        backup.push_op(Op::Mutation {
                            seq,
                            priority: pri,
                            table: table.to_string(),
                            action: action_str.to_string(),
                            row_id,
                        });
                    }
                },
            ));
            tracing::info!("[DB Hook] Registered for files + backup tables.");
        });
    }

    // --- Initialize AppState: Shared memory for the whole app ---
    // (This version doesn't have bridge_port yet because bridge hasn't started)
    let download_manager = Arc::new(omega_drive_download::DownloadManager::new());
    let provider_runtime_raw = build_provider_runtime(install_results);
    let provider_runtime = Arc::new(std::sync::RwLock::new(Arc::clone(&provider_runtime_raw)));

    let backup_enabled = cfg.read().expect("cfg RwLock").backup_enabled;
    let backup_snapshot_interval_days = cfg.read().expect("cfg RwLock").backup_snapshot_interval_days;

    omega_drive_player::debug::debug_init(&base_dir);
    let player_cfg = Arc::new(cfg.read().expect("cfg RwLock").clone());

    {
        let gc_cfg = Arc::clone(&cfg);
        let db_write2 = Arc::clone(&db_write);
        let provider_runtime_ref = Arc::clone(&provider_runtime);
        let backup_gc = Arc::clone(&backup_service);
        tokio::spawn(async move {
            gc_task(gc_cfg, db_write2, provider_runtime_ref, backup_gc).await;
        });
    }
    // --- EngineContext ---
    let engine_ctx = EngineContext {
        integrity: Arc::new(EngineIntegrityService),
        zip: Arc::new(EngineZipService),
    };

    // --- Khoi tao DriveService ---
    let drive_service = Arc::new(DriveService::new(
        Arc::new(DbFileRepository::new(Arc::clone(&db_read), Arc::clone(&db_write))),
        Arc::new(DbFolderRepository::new(Arc::clone(&db_write))),
        Arc::clone(&provider_runtime_raw),
        Arc::clone(&event_bus),
        engine_ctx.clone(),
    ));
    let player_runtime = Arc::new(PlayerRuntime::new(&player_cfg));
    player_runtime.start_idle_gc();
    let app_handle_state = Arc::new(std::sync::Mutex::new(None));
    let bridge_port = pick_bridge_port(13370);
    let file_repo: Arc<dyn omega_drive_gateway::provider::file_repository::FileRepository> = Arc::new(DbFileRepository::new(Arc::clone(&db_read), Arc::clone(&db_write)));
    let shared_cdn_link_cache = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let download_ctx = omega_drive_download::DownloadContext {
        cfg: Arc::clone(&cfg),
        file_repo: Arc::clone(&file_repo),
        download_job_repo: Arc::new(DbDownloadJobRepository::new(Arc::clone(&db_write))),
        provider_runtime: Arc::clone(&provider_runtime_raw),
        app_ctx: Arc::new(NoopAppContext),
        ui_heartbeats: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        engine: engine_ctx.clone(),
        cdn_link_cache: Arc::clone(&shared_cdn_link_cache),
        base_dir: base_dir.clone(),
        stream_registry: provider_runtime_raw.stream_registry.clone(),
        mem_cache: Arc::new(omega_drive_download::PartitionedMemCache::new(
            50 * 1024 * 1024,
            std::collections::HashMap::new(),
        )),
    };
    let player_ctx = Arc::new(PlayerContext {
        player_runtime: Arc::clone(&player_runtime),
        bridge_port: Arc::new(AtomicU16::new(bridge_port)),
        file_repo: Arc::clone(&file_repo),
        cfg: Arc::clone(&player_cfg),
        cdn_link_cache: Arc::clone(&shared_cdn_link_cache),
        base_dir: base_dir.clone(),
        disk_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
        stream_registry: {
            let guard = match provider_runtime.read() {
                Ok(g) => Arc::clone(&g),
                Err(poisoned) => Arc::clone(&poisoned.into_inner()),
            };
            Arc::clone(&guard.stream_registry)
        },
        event_emitter: Arc::new(TauriEventEmitter(Arc::clone(&app_handle_state))),
        debug_logger: Arc::new(DefaultDebugLogger),
        ui_last_heartbeat: Arc::new(std::sync::atomic::AtomicU64::new(
            match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(dur) => dur.as_secs(),
                Err(e) => {
                    tracing::warn!("SystemTime error in bootstrap: {}", e);
                    0
                }
            },
        )),
        idx_cache: IdxCache::new(base_dir.join("idx_cache")),
        download_ctx,
    });
    omega_drive_telegram::services::init(Box::new(DefaultDebugLogger));
    let mut app_state_init = AppState {
        cfg: Arc::clone(&cfg),
        db_read: Arc::clone(&db_read),
        db_write: Arc::clone(&db_write),
        drive_db_read: Arc::clone(&drive_db_read),
        provider_runtime,
        senders: new_sender_map(),
        progress_map: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        base_dir: base_dir.clone(),
        thumbnail_dir: thumbnail_dir.clone(),
        feature_logs: Arc::clone(&feature_logs),
        cdn_link_cache: Arc::clone(&shared_cdn_link_cache),
        events: Arc::clone(&event_bus),
        drive_service: Arc::clone(&drive_service),
        bridge_port,
        book_bridge_port: 0,
        file_repo: Arc::clone(&file_repo),
        active_tenant: Arc::new(std::sync::Mutex::new(default_tenant)),
        player_runtime: player_runtime.clone(),
        download_manager: Arc::clone(&download_manager),
        disk_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
        stream_spool_sem: Arc::new(tokio::sync::Semaphore::new(3)),
        stream_spool_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        stream_spool_limit_bytes: 2 * 1024 * 1024 * 1024,
        ui_last_heartbeat: Arc::new(std::sync::atomic::AtomicU64::new(
            match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(dur) => dur.as_secs(),
                Err(e) => {
                    tracing::warn!("SystemTime error in bootstrap: {}", e);
                    0
                }
            },
        )),
        app_ctx: Arc::new(std::sync::Mutex::new(None)),
        sidecar: Arc::new(std::sync::Mutex::new(None)),
        ui_ping_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        ui_heartbeats: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        backup_service: Some(Arc::clone(&backup_service)),
        engine: engine_ctx,
        player_ctx: Arc::clone(&player_ctx),
        folder_repo: Arc::new(DbFolderRepository::new(Arc::clone(&db_write))),
        upload_job_repo: Arc::new(DbUploadJobRepository::new(Arc::clone(&db_write))),
        download_job_repo: Arc::new(DbDownloadJobRepository::new(Arc::clone(&db_write))),
        mem_cache: Arc::new(omega_drive_download::PartitionedMemCache::new(
            50 * 1024 * 1024,
            std::collections::HashMap::new(),
        )),
    };

    // Startup: flush any pending ops from crash recovery
    if backup_service.has_pending() {
        info!("Backup: flushing pending ops from crash recovery");
        backup_service.flush_queues();
    }

    // Spawn P1 timer (5 min) and P2 timer (30 min)
    {
        let bs1 = Arc::clone(&backup_service);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(300)).await;
                bs1.flush_queues();
            }
        });
    }
    {
        let bs2 = Arc::clone(&backup_service);
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(1800)).await;
                bs2.flush_queues();
            }
        });
    }

    // --- Snapshot scheduler ---
    if backup_enabled {
        let interval_secs = backup_snapshot_interval_days * 86400;
        let bs_snap = Arc::clone(&backup_service);
        let db_snap = Arc::clone(&db_write);
        let base_snap = base_dir.clone();
        tokio::spawn(async move {
            let dc = match crate::providers::discord_provider::discord_backup_gateway() {
                Some(dc) => dc,
                None => {
                    error!("Backup: Discord not initialized — snapshot scheduler disabled");
                    return;
                }
            };
            async fn run(
                dc: &DiscordBackupGateway,
                db: &dyn DbExecutor,
                dir: &PathBuf,
                bs: &Arc<BackupService>,
            ) {
                let chunks = {
                    let mut result = None;
                    db.read(&mut |c| {
                        match omega_drive_db::backup::create_snapshot(c, dir) {
                            Ok(chunks) => result = Some(chunks),
                            Err(e) => error!("Backup: create_snapshot failed: {e}"),
                        }
                    });
                    match result {
                        Some(chunks) => chunks,
                        None => return,
                    }
                };
                if let Err(e) = crate::features::backup::run_snapshot_and_upload(dc, chunks, bs).await {
                    error!("Backup: snapshot failed: {e}");
                }
            }
            run(&dc, db_snap.as_ref(), &base_snap, &bs_snap).await;
            loop {
                sleep(Duration::from_secs(interval_secs)).await;
                info!("Backup: starting scheduled snapshot");
            run(&dc, db_snap.as_ref(), &base_snap, &bs_snap).await;
            }
        });
    }

    // --- Start Book Bridge (port 13480) ---
    let book_bridge_manager = Arc::new(omega_drive_book_bridge::BookManager::new());
    let book_bridge_port = {
        let sr = match app_state_init.provider_runtime.read() {
            Ok(g) => Arc::clone(&g),
            Err(p) => Arc::clone(&p.into_inner()),
        };
        let cfg = omega_drive_book_bridge::BookBridgeConfig {
            base_dir: base_dir.clone(),
            file_repo: Arc::clone(&file_repo),
            stream_registry: Arc::clone(&sr.stream_registry),
            engine: app_state_init.engine.clone(),
            port: pick_bridge_port(13480),
            byte_cache: player_runtime.sparse_cache.clone() as Arc<dyn ByteCache>,
            singleflight: player_runtime.part_singleflight.clone() as Arc<dyn PartSingleFlight>,
        };
        match omega_drive_book_bridge::start_book_bridge(cfg, Arc::clone(&book_bridge_manager)).await {
            Ok(port) => {
                tracing::info!("Book bridge started at port {}", port);
                port
            }
            Err(e) => {
                eprintln!("Failed to start book bridge: {e}");
                std::process::exit(1);
            }
        }
    };
    app_state_init.book_bridge_port = book_bridge_port;

    // --- Start HTTP Bridge (dynamic port) ---
    #[cfg(feature = "player")]
    {
        match ensure_video_bridge_child(
            &base_dir,
            app_state_init.bridge_port,
            &app_state_init.player_runtime.video_bridge_processes,
        )
        .await
        {
            Ok(actual_port) => {
                if actual_port != app_state_init.bridge_port {
                    tracing::info!("Bridge port adjusted {} → {}", app_state_init.bridge_port, actual_port);
                    app_state_init.bridge_port = actual_port;
                }
                // always sync player_ctx so nativeplayer.rs uses the actual port
                player_ctx.bridge_port.store(actual_port, std::sync::atomic::Ordering::Relaxed);
            }
            Err(err) => {
                eprintln!("Failed to start video bridge child process: {err}");
                std::process::exit(1);
            }
        }
    }

    let app_state = app_state_init;
    let download_manager = app_state.download_manager.clone();
    download_manager.start(app_state.download_context());

    // --- Start Tauri ---
    tauri_app::run_tauri(app_state);
}

/// Background task to clean up old files.
async fn gc_task(
    cfg: Arc<RwLock<Config>>,
    db: Arc<DbWriteQueue>,
    provider_runtime: Arc<std::sync::RwLock<Arc<ProviderRuntime>>>,
    backup_service: Arc<BackupService>,
) {
    use omega_drive_db::files as db_files;

    let trash_ttl_secs: i64 = cfg.read().expect("cfg RwLock").trash_ttl_days * 24 * 3600;

    loop {
        // Sleep according to configured interval
        let gc_interval_s = cfg.read().expect("cfg RwLock").gc_interval_s;
        sleep(Duration::from_secs(gc_interval_s)).await;

        // Thread 1: Find and permanently delete files in Trash past TTL
        let to_purge = {
            let db_lock = db.lock().await;
            let mut stmt = match db_lock.conn().prepare(
                "SELECT id, filename FROM files 
                 WHERE status = 'trashed' 
                 AND strftime('%s', 'now') - strftime('%s', deleted_at) > ?",
            ) {
                Ok(s) => s,
                Err(e) => {
                    error!("GC: Prepare query error: {}", e);
                    continue; // Skip this iteration
                }
            };

            let rows = stmt.query_map([trash_ttl_secs], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            });

            match rows {
                Ok(mapped_rows) => mapped_rows.filter_map(|r| r.ok()).collect::<Vec<_>>(),
                Err(e) => {
                    error!("GC: Query old files error: {}", e);
                    Vec::new()
                }
            }
        };

        for (id, filename) in to_purge {
            // Claim first to avoid race with restore from UI.
            let claimed =
                {
                    let db_lock = db.lock().await;
                    db_lock.conn().execute(
                    "UPDATE files SET status = 'purging' WHERE id = ? AND status = 'trashed'",
                    [id]
                ).unwrap_or(0)
                };
            if claimed == 0 {
                continue;
            }

            info!("GC: Purging file '{}'", filename);
            let mut remote_failed = false;

            let provider_parts = {
                let db_lock = db.lock().await;
                db_files::get_parts_for_file(db_lock.conn(), id).unwrap_or_default()
            };
            let mut parts_by_platform = std::collections::HashMap::<String, Vec<_>>::new();
            for part in provider_parts {
                parts_by_platform
                    .entry(part.platform.clone())
                    .or_default()
                    .push(part);
            }
            for (platform, parts) in parts_by_platform {
                let runtime = match provider_runtime.read() {
                    Ok(guard) => Arc::clone(&guard),
                    Err(poisoned) => Arc::clone(&poisoned.into_inner()),
                };
                let Some(gateway) = runtime.remote_object_registry.get(&platform) else {
                    error!(
                        "GC: Remote-object gateway '{}' not found for file {}",
                        platform, id
                    );
                    remote_failed = true;
                    continue;
                };
                if let Err(e) = gateway.delete_file_artifacts(id, &parts).await {
                    error!(
                        "GC: Cannot delete artifacts on provider '{}' for file {}: {}",
                        platform, id, e
                    );
                    remote_failed = true;
                }
            }

            if remote_failed {
                // Revert to trashed status so next GC can retry.
                let db_lock = db.lock().await;
                let _ = db_lock.conn().execute(
                    "UPDATE files SET status = 'trashed' WHERE id = ? AND status = 'purging'",
                    [id],
                );
                continue;
            }

            // Backup snapshot before delete
            {
                let db_lock = db.lock().await;
                match omega_drive_db::backup::capture_file_state(db_lock.conn(), id) {
                    Ok(payload) => {
                        let seq = backup_service.next_seq();
                        let op = crate::features::backup::Op::FileSnapshot {
                            seq,
                            file_id: id,
                            action: "delete".to_string(),
                            payload,
                        };
                        backup_service.push_op(op);
                        backup_service.flush_queues();
                    }
                    Err(e) => {
                        error!("Backup: failed to capture file state before purge: {e}");
                    }
                }
            }

            // Delete local DB record if it's still the claimed record.
            let db_lock = db.lock().await;
            let _ = db_lock.conn().execute(
                "DELETE FROM files WHERE id = ? AND status = 'purging'",
                [id],
            );
        }

    }
}





