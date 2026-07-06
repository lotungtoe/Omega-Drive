use async_trait::async_trait;
use omega_drive_gateway::provider::app_context::{AppContext, SidecarProvider};
use tauri_plugin_shell::ShellExt;

use super::bridge::ensure_video_bridge_child_for_player;
use crate::app_wiring::app_runtime::AppState;
use omega_drive_player::nativeplayer::{open_in_native_player, MpvSessionType};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Listener, Manager,
};
use tracing::{error, info, warn};

#[cfg(debug_assertions)]
fn install_emergency_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[CRASH] PANIC: {info}");
        tracing::error!("{msg}");
        let _ = std::fs::write(std::env::temp_dir().join("omega_drive_panic.log"), &msg);
        prev(info);
    }));
}

pub(super) fn run_tauri(app_state: AppState) {
    #[cfg(debug_assertions)]
    install_emergency_panic_hook();

    let last_window_event = Arc::new(AtomicU64::new(0));
    let lwe_ev = last_window_event.clone();

    tauri::Builder::default()
        .manage(app_state.clone())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tracing::info!("Single instance triggered, argv: {:?}, cwd: {}", argv, cwd);
            let window = app.get_webview_window("main");
            
            let mut show_ui = true;
            if let Some(url) = argv.into_iter().find(|a| a.starts_with("omegadrive://")) {
                tracing::info!("Found omegadrive URL in second instance: {}", url);
                if url.starts_with("omegadrive://play/") {
                    show_ui = false;
                    if let Ok(file_id) = url.replace("omegadrive://play/", "").parse::<i64>() {
                       let cloned_app = app.clone();
                       tauri::async_runtime::spawn(async move {
                           let state = cloned_app.state::<AppState>();
                           #[cfg(feature = "player")]
                           {
                            if let Err(err) = ensure_video_bridge_child_for_player(state.inner()).await {
                                warn!("Failed to ensure video bridge child before deep-link playback: {}", err);
                                return;
                            }
                            let _ = open_in_native_player(
                                state.player_ctx.as_ref(),
                                file_id,
                                "Xem file - Omega Drive".to_string(),
                                None,
                                Some(MpvSessionType::Video)
                            ).await;
                           }
                       });
                    }
                } else {
                    if let Some(window) = &window {
                        let _ = window.emit("omegadrive-deep-link", url);
                    }
                }
            }
            
            if show_ui {
                if let Some(window) = window {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .on_window_event(move |window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let _ = window.hide();
                    api.prevent_close();
                }
                tauri::WindowEvent::Focused(focused) => {
                    tracing::debug!("[WINDOW] focused={}", focused);
                    lwe_ev.store(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        Ordering::Relaxed,
                    );
                }
                _ => {}
            }
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            let state_in_tauri = app.state::<AppState>();
            
            tracing::info!("🔒 Setup: Attempting AppHandle LOCK...");
            let ctx: Arc<dyn AppContext> = Arc::new(TauriAppContext(handle.clone()));
            if let Ok(mut lock) = state_in_tauri.app_ctx.lock() {
                *lock = Some(ctx);
                tracing::info!("🔒 Setup: AppContext set");
            }
            if let Ok(mut lock) = state_in_tauri.sidecar.lock() {
                *lock = Some(Arc::new(TauriSidecarProvider(handle.clone())));
            }
            
            let args: Vec<String> = std::env::args().collect();
            let mut show_ui = true;
            
            if args.contains(&"--minimized".to_string()) {
                show_ui = false;
            }

            let mut run_play = None;
            if let Some(url) = args.iter().find(|a| a.starts_with("omegadrive://")) {
                if url.starts_with("omegadrive://play/") {
                    show_ui = false;
                    if let Ok(file_id) = url.replace("omegadrive://play/", "").parse::<i64>() {
                        run_play = Some(file_id);
                    }
                }
            }

            if show_ui {
                if let Some(window) = app.get_webview_window("main") {
                    let win = window.clone();
                    let _ = window.once("frontend-ready", move |_| {
                        if let Err(e) = win.show() {
                            error!("frontend-ready: failed to show window: {e}");
                        }
                        if let Err(e) = win.set_focus() {
                            error!("frontend-ready: failed to set focus: {e}");
                        }
                    });
                }
            }
            
            let cloned_app = app.handle().clone();
            let emit_url = args.into_iter().find(|a| a.starts_with("omegadrive://") && !a.starts_with("omegadrive://play/"));
            
            let is_persistent = state_in_tauri.cfg.read().expect("cfg RwLock").persistent_video_bridge;
            tokio::spawn(async move {
                if let Some(file_id) = run_play {
                    let state = cloned_app.state::<AppState>();
                    #[cfg(feature = "player")]
                    {
                        if let Err(err) = ensure_video_bridge_child_for_player(state.inner()).await {
                            warn!("Failed to ensure video bridge child during startup playback: {}", err);
                            return;
                        }
                        let _ = open_in_native_player(
                            state.player_ctx.as_ref(),
                            file_id,
                            "Xem file - Omega Drive".to_string(),
                            None,
                            Some(MpvSessionType::Video)
                        ).await;
                    }
                } else if is_persistent {
                    let state = cloned_app.state::<AppState>();
                    info!("Persistent video bridge enabled; spawning early...");
                    if let Err(err) = ensure_video_bridge_child_for_player(state.inner()).await {
                         warn!("Failed to spawn persistent video bridge on startup: {}", err);
                    }
                }

                if let Some(url) = emit_url {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    let _ = cloned_app.emit("omegadrive-deep-link", url);
                }
            });

            // --- OMEGA EVENT BRIDGE (Internal Bus -> Tauri UI) ---
            let internal_event_bus = Arc::clone(&state_in_tauri.events);
            let app_handle_for_events = handle.clone();
            tokio::spawn(async move {
                let mut rx = internal_event_bus.subscribe();
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if matches!(event, omega_drive_gateway::core::events::OmegaEvent::FilesTableChanged) {
                                tracing::info!("🚀 [Bridge] Forwarding FilesTableChanged to UI");
                            }
                            let _ = app_handle_for_events.emit("omega-event", event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!("🚀 [Bridge] Event loop lagged by {} items, some UI updates might be skipped", count);
                            continue;
                        }
                    }
                }
            });

            let quit_i = MenuItem::with_id(app, "quit", "Thoát ứng dụng", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Mở giao diện", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            } else {
                warn!("No default window icon available for tray icon");
            }

            let _tray = tray_builder
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Hide window if starting with --minimized
            if std::env::args().any(|arg| arg == "--minimized") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            
            // --- UI WATCHDOG (Forensic Monitoring) ---
            let ping_count = state_in_tauri.ui_ping_count.clone();
            let heartbeat_registry = state_in_tauri.ui_heartbeats.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    if ping_count.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                        continue;
                    }

                    let now = match std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                    {
                        Ok(dur) => dur.as_secs(),
                        Err(e) => {
                            warn!("SystemTime error in UI watchdog: {}", e);
                            0
                        }
                    };

                    let overdue_windows = {
                        let mut collected = Vec::new();
                        if let Ok(mut registry) = heartbeat_registry.lock() {
                            registry.retain(|_, status| {
                                now.saturating_sub(status.last_seen_epoch_secs) <= 300
                            });

                            for (label, status) in registry.iter() {
                                let lag_secs = now.saturating_sub(status.last_seen_epoch_secs);
                                let should_monitor = status.visible || status.focused;
                                if should_monitor && lag_secs > 15 {
                                    collected.push(format!(
                                        "{} ({}; visible={}, focused={}, {}s)",
                                        label,
                                        status.context,
                                        status.visible,
                                        status.focused,
                                        lag_secs
                                    ));
                                }
                            }
                        }
                        collected
                    };

                    if !overdue_windows.is_empty() {
                        warn!(
                            "🚨 [CRITICAL] UI WATCHDOG: visible renderer heartbeat missing: {}",
                            overdue_windows.join(", ")
                        );
                        warn!(
                            "👉 This now indicates the active window stopped sending heartbeats, not merely a background timer throttle."
                        );
                    }
                }
            });

            // --- KIá»‚M TRA CĂC TĂC Vá»¤ UPLOAD Dá» DANG ---
            let st = state_in_tauri.inner().clone();
            tokio::spawn(async move {
                use omega_drive_db::files as db_files;

                // Nghá»‰ ngÆ¡i má»™t chĂºt Ä‘á»ƒ há»‡ thá»‘ng khá»Ÿi Ä‘á»™ng hoĂ n táº¥t
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                let files = {
                    let db = st.db_read.lock().await;
                    db_files::get_all_files(db.conn()).unwrap_or_default()
                };

                let stalled: Vec<_> = files
                    .into_iter()
                    .filter(|f| {
                        (f.status == "uploading" || f.status == "processing")
                            && f.local_path.is_some()
                    })
                    .collect();

                if !stalled.is_empty() {
                    info!(
                        "PhĂ¡t hiá»‡n {} tĂ¡c vá»¥ upload chÆ°a hoĂ n táº¥t. NgÆ°á»i dĂ¹ng cĂ³ thá»ƒ tiáº¿p tá»¥c thá»§ cĂ´ng tá»« danh sĂ¡ch file.",
                        stalled.len()
                    );
                }
            });

            // --- WV MONITOR: phát hiện WebView2 ngừng gửi window event ---
            let lwe_mon = last_window_event.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    let last = lwe_mon.load(Ordering::Relaxed);
                    if last == 0 {
                        continue;
                    }
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    if now.saturating_sub(last) > 15 {
                        warn!(
                            "🚨 [WV MONITOR] WebView2 không gửi window event >15s (last={last}) — webview đã chết"
                        );
                        let _ = std::fs::write(
                            std::env::temp_dir().join("omega_drive_wv_dead.txt"),
                            format!("WebView2 silent at {now}"),
                        );
                        lwe_mon.store(0, Ordering::Relaxed); // chỉ warn 1 lần
                    }
                }
            });

            // --- CHILD PROCESS MONITOR (Windows) ---
            #[cfg(windows)]
            tokio::spawn(async move {
                use std::collections::HashMap;
                use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
                use windows_sys::Win32::System::Threading::*;
                use windows_sys::Win32::Foundation::*;

                unsafe fn send_handle(h: HANDLE) -> usize { h as usize }
                unsafe fn recv_handle(h: usize) -> HANDLE { h as HANDLE }

                let ppid = std::process::id();
                let mut children: HashMap<u32, usize> = HashMap::new();
                let exit_log = std::env::temp_dir().join("omega_drive_child_exit.txt");

                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    unsafe {
                        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                        if snap == INVALID_HANDLE_VALUE {
                            continue;
                        }
                        let mut pe = std::mem::zeroed::<PROCESSENTRY32W>();
                        pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

                        let mut alive: std::collections::HashSet<u32> = std::collections::HashSet::new();
                        if Process32FirstW(snap, &mut pe) != 0 {
                            loop {
                                if pe.th32ParentProcessID == ppid {
                                    alive.insert(pe.th32ProcessID);
                                }
                                if Process32NextW(snap, &mut pe) == 0 {
                                    break;
                                }
                            }
                        }
                        CloseHandle(snap);

                        // Report dead children
                        let dead: Vec<u32> = children.keys().copied()
                            .filter(|pid| !alive.contains(pid))
                            .collect();
                        for pid in dead {
                            if let Some(&handle_usize) = children.get(&pid) {
                                let handle = recv_handle(handle_usize);
                                let mut code: u32 = 0;
                                let result = if GetExitCodeProcess(handle, &mut code) != 0 {
                                    format!("Child PID {} exited with code {:#010x}\n", pid, code)
                                } else {
                                    format!("Child PID {} died, GetExitCodeProcess failed\n", pid)
                                };
                                let _ = std::fs::OpenOptions::new()
                                    .append(true).create(true).open(&exit_log)
                                    .and_then(|mut f| std::io::Write::write_all(&mut f, result.as_bytes()));
                                CloseHandle(handle);
                                children.remove(&pid);
                            }
                        }

                        // Discover new children
                        for pid in &alive {
                            if !children.contains_key(pid) {
                                let handle = OpenProcess(PROCESS_QUERY_INFORMATION, 0, *pid);
                                if !handle.is_null() {
                                    children.insert(*pid, send_handle(handle));
                                }
                            }
                        }
                    }
                }
            });

            #[cfg(debug_assertions)]
            if let Some(dev_win) = app.get_webview_window("main") {
                dev_win.open_devtools();
            }

            Ok(())
        })
        .invoke_handler(crate::tauri_feature_handler!())
        .run(tauri::generate_context!())
        .unwrap_or_else(|err| panic!("Lỗi khi khởi chạy giao diện Tauri: {err}"));
}

struct TauriAppContext(tauri::AppHandle);

impl AppContext for TauriAppContext {
    fn emit_event(&self, event: &str, payload: serde_json::Value) {
        let _ = self.0.emit(event, payload);
    }
}

struct TauriSidecarProvider(tauri::AppHandle);

#[async_trait]
impl SidecarProvider for TauriSidecarProvider {
    async fn sidecar_output(&self, name: &str, args: &[&str]) -> anyhow::Result<Vec<u8>> {
        let sidecar = self.0.shell().sidecar(name)?;
        let output = sidecar.args(args).output().await?;
        Ok(output.stdout)
    }
}



