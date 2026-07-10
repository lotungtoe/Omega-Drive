use std::{
    path::{Path, PathBuf},
    process::Child,
    sync::{Arc, RwLock},
};

use tracing::info;

use omega_drive_player::bridge as player_bridge;
use omega_drive_player::runtime::{
    ensure_video_bridge_child as ensure_video_bridge_child_shared,
    ensure_video_bridge_child_for_player as ensure_video_bridge_child_for_player_shared,
    PlayerRuntime,
};
use omega_drive_player::{IdxCache, PlayerContext};
use crate::app_wiring::app_runtime::AppState;
use crate::db::repos::{
    DbDownloadJobRepository, DbFileRepository, DbFolderRepository, DbUploadJobRepository,
};
use crate::features::drive::DriveService;
use crate::providers::install::{
    build_provider_runtime, install_builtin_providers, prepare_builtin_provider_state,
    ProviderInstallContext,
};

use omega_drive_core::ports::app_context::NoopAppContext;
use omega_drive_engine::integrity::EngineIntegrityService;
use omega_drive_engine::zip_utils::EngineZipService;
use omega_drive_gateway::core::engine_context::EngineContext;
use omega_drive_gateway::download::ByteStreamProvider;
use omega_drive_core::services::DefaultDebugLogger;
use omega_drive_gateway::core::tenant::TenantDescriptor;
use omega_drive_db::{Db, DbWriteQueue, ReadDbPool};
use omega_drive_gateway::core::types::new_sender_map;

use super::paths::{resolve_startup_tenant, resolve_tenant_db_path};

#[cfg(target_os = "windows")]
fn enable_bridge_native_dpi_awareness() {
    use windows_sys::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };

    // SAFETY: This only asks Windows to switch the current process to the
    // documented DPI-awareness mode before any bridge-owned window is created.
    unsafe {
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0 {
            tracing::info!(
                target: "feature::player::bridge",
                "Video bridge process set DPI awareness to PerMonitorV2 before mpv window creation"
            );
        } else {
            tracing::warn!(
                target: "feature::player::bridge",
                "Video bridge process could not set DPI awareness to PerMonitorV2; the executable manifest may already have fixed the mode"
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn enable_bridge_native_dpi_awareness() {}

pub(super) async fn ensure_video_bridge_child(
    base_dir: &Path,
    bridge_port: u16,
    processes: &Arc<std::sync::Mutex<std::collections::HashMap<String, Child>>>,
) -> Result<u16, String> {
    ensure_video_bridge_child_shared(base_dir, bridge_port, processes).await
}

pub(super) async fn ensure_video_bridge_child_for_player(state: &AppState) -> Result<(), String> {
    ensure_video_bridge_child_for_player_shared(state.player_ctx.as_ref()).await.map(|_| ())
}

pub(super) async fn run_video_bridge_process(
    base_dir: PathBuf,
    bridge_port: u16,
    parent_pid: Option<u32>,
) -> Result<(), String> {
    if let Some(parent_pid) = parent_pid {
        spawn_parent_watchdog(parent_pid)?;
    }

    let env_path = base_dir.join("bot.env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    }

    let cfg = Arc::new(RwLock::new(omega_drive_core::config::load_config(
        &base_dir,
        &crate::providers::config::builtin_provider_config_descriptors(),
    )));
    let player_cfg = Arc::new(cfg.read().expect("cfg RwLock").clone());
    let feature_logs = Arc::new(crate::app_wiring::infrastructure::feature_log::init_tracing(
        &cfg.read().expect("cfg RwLock"), &base_dir,
    ));
    enable_bridge_native_dpi_awareness();
    let thumbnail_dir = base_dir.join("thumbnails_cache");
    let _ = std::fs::create_dir_all(&thumbnail_dir);

    let default_tenant = resolve_startup_tenant(&base_dir);
    let _ =
        omega_drive_core::tenant_registry::persist_active_tenant(&base_dir, &default_tenant);
    let prepared_providers = prepare_builtin_provider_state(&base_dir, &default_tenant)
        .await
        .map_err(|e| e.to_string())?;
    let db_path = resolve_tenant_db_path(&base_dir, &default_tenant);
    let db_write_conn = Db::open(&db_path).map_err(|e| e.to_string())?;
    omega_drive_db::tenant_meta::upsert_tenant_meta(db_write_conn.conn(), &default_tenant)
        .map_err(|e| e.to_string())?;
    let drive_db_read = Arc::new(
        ReadDbPool::open(&db_path, ReadDbPool::recommended_size())
            .map_err(|e| format!("Could not open SQLite drive read pool: {e}"))?,
    );
    let db_write = Arc::new(DbWriteQueue::new(db_write_conn));
    let db_read = Arc::clone(&drive_db_read);

    let event_bus = Arc::new(omega_drive_gateway::core::events::EventBus::new());
    let install_ctx = ProviderInstallContext {
        base_dir: base_dir.clone(),
        db_read: Arc::clone(&db_read),
        db_write: Arc::clone(&db_write),
        prepared: prepared_providers,
        event_bus: Arc::clone(&event_bus),
        tenant: default_tenant.clone(),
    };
    let install_results = install_builtin_providers(install_ctx)
        .await
        .map_err(|e| e.to_string())?;
    let download_manager = Arc::new(omega_drive_download::DownloadManager::new());
    let provider_runtime_raw = build_provider_runtime(
        install_results,
    );
    let provider_runtime = Arc::new(std::sync::RwLock::new(Arc::clone(&provider_runtime_raw)));
    let engine_ctx = EngineContext {
        integrity: Arc::new(EngineIntegrityService),
        zip: Arc::new(EngineZipService),
    };
    let drive_service = Arc::new(DriveService::new(
        Arc::new(DbFileRepository::new(Arc::clone(&db_read), Arc::clone(&db_write))),
        Arc::new(DbFolderRepository::new(Arc::clone(&db_write))),
        Arc::clone(&provider_runtime_raw),
        Arc::clone(&event_bus),
        engine_ctx.clone(),
    ));
    let player_runtime = Arc::new(PlayerRuntime::new(&player_cfg));
    player_runtime.active_playback_windows.lock().expect("Mutex poisoned").insert("video_bridge".to_string());
    player_runtime.start_idle_gc();
        let file_repo: Arc<dyn omega_drive_gateway::provider::file_repository::FileRepository> = Arc::new(DbFileRepository::new(Arc::clone(&db_read), Arc::clone(&db_write)));
    let shared_cdn_link_cache = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));
    let download_ctx = Arc::new(omega_drive_download::DownloadContext {
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
    });
    let byte_stream_provider: Arc<dyn ByteStreamProvider> = Arc::new(
        omega_drive_download::DownloadByteStreamProvider::new(Arc::clone(&download_ctx)),
    );
    let player_ctx = Arc::new(PlayerContext {
        player_runtime: Arc::clone(&player_runtime),
        bridge_port: Arc::new(std::sync::atomic::AtomicU16::new(bridge_port)),
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
        event_emitter: Arc::new(crate::app::event_emitter::TauriEventEmitter(Arc::new(std::sync::Mutex::new(None)))),
        debug_logger: Arc::new(DefaultDebugLogger),
        ui_last_heartbeat: Arc::new(std::sync::atomic::AtomicU64::new(
            match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(dur) => dur.as_secs(),
                Err(e) => {
                    tracing::warn!("SystemTime error in video bridge: {}", e);
                    0
                }
            },
        )),
        idx_cache: IdxCache::new(base_dir.join("idx_cache")),
        download_ctx: (*download_ctx).clone(),
        byte_stream_provider: Arc::clone(&byte_stream_provider),
    });

    let state = AppState {
        cfg: Arc::clone(&cfg),
        db_read: db_read.clone(),
        db_write: db_write.clone(),
        drive_db_read,
        provider_runtime,
        senders: new_sender_map(),
        progress_map: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        base_dir: base_dir.clone(),
        thumbnail_dir,
        feature_logs: Arc::clone(&feature_logs),
        cdn_link_cache: Arc::clone(&shared_cdn_link_cache),
        events: event_bus,
        drive_service,
        bridge_port,
        book_bridge_port: 0,
        file_repo: Arc::clone(&file_repo),
        active_tenant: Arc::new(std::sync::Mutex::new(TenantDescriptor::new(
            default_tenant.scope,
            default_tenant.discord_guild_id,
            default_tenant.telegram_group_id,
        ))),
        player_runtime,
        download_manager,
        disk_semaphore: Arc::new(tokio::sync::Semaphore::new(2)),
        stream_spool_sem: Arc::new(tokio::sync::Semaphore::new(3)),
        stream_spool_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        stream_spool_limit_bytes: 2 * 1024 * 1024 * 1024,
        ui_last_heartbeat: Arc::new(std::sync::atomic::AtomicU64::new(
            match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
                Ok(dur) => dur.as_secs(),
                Err(e) => {
                    tracing::warn!("SystemTime error in video bridge: {}", e);
                    0
                }
            },
        )),
        app_ctx: Arc::new(std::sync::Mutex::new(None)),
        sidecar: Arc::new(std::sync::Mutex::new(None)),
        ui_ping_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        ui_heartbeats: Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
        backup_service: None,
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

    let actual_port = player_bridge::start_bridge((*state.player_ctx).clone()).await?;
    info!("Video bridge process is ready at http://127.0.0.1:{actual_port}");

    std::future::pending::<()>().await;
    #[allow(unreachable_code)]
    Ok(())
}

#[cfg(target_os = "windows")]
fn spawn_parent_watchdog(parent_pid: u32) -> Result<(), String> {
    std::thread::Builder::new()
        .name("video-bridge-parent-watchdog".to_string())
        .spawn(move || {
            use windows_sys::Win32::Foundation::{CloseHandle, WAIT_FAILED};
            use windows_sys::Win32::System::Threading::{
                OpenProcess, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION,
            };
            const SYNCHRONIZE_ACCESS: u32 = 0x0010_0000;

            // SAFETY: The watchdog only calls Win32 process-handle APIs with
            // the parent PID supplied by the current process and closes the
            // handle before terminating the child process.
            unsafe {
                let handle = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE_ACCESS,
                    0,
                    parent_pid,
                );
                if handle.is_null() {
                    tracing::warn!(
                        "Video bridge watchdog could not open parent process {}; exiting child",
                        parent_pid
                    );
                    std::process::exit(0);
                }

                let wait_result = WaitForSingleObject(handle, u32::MAX);
                CloseHandle(handle);

                if wait_result == WAIT_FAILED {
                    tracing::warn!(
                        "Video bridge watchdog wait failed for parent process {}; exiting child",
                        parent_pid
                    );
                }
            }

            std::process::exit(0);
        })
        .map(|_| ())
        .map_err(|err| format!("Failed to start video bridge parent watchdog: {err}"))
}

#[cfg(not(target_os = "windows"))]
fn spawn_parent_watchdog(_parent_pid: u32) -> Result<(), String> {
    Ok(())
}

// playback_active and open_video_window were removed.
// The real implementations live in src/api/plugins/window_bridge.rs.




