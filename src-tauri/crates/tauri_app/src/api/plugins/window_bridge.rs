use tauri::State;
use tracing::info;

use crate::app_runtime::AppState;
use omega_drive_player::{nativeplayer, runtime::ensure_video_bridge_child_for_player};

#[tauri::command]
pub fn playback_active(
    state: State<'_, AppState>,
    active: bool,
    window_label: String,
) -> Result<(), String> {
    info!(
        "[IPC Audit] playback_active (active={}, label={})",
        active, window_label
    );

    let any_active = {
        let mut windows = state
            .player_runtime
            .active_playback_windows
            .lock()
            .map_err(|_| "Lock playback state (poisoned)".to_string())?;

        if active {
            windows.insert(window_label);
        } else {
            windows.remove(&window_label);
        }

        !windows.is_empty()
    };

    if let Some(ctx) = state.app_ctx_emit() {
        ctx.emit_event("playback-state-changed", serde_json::json!(any_active));
        Ok(())
    } else {
        let msg = "AppHandle not ready to emit playback-state-changed";
        tracing::error!("{}", msg);
        Err(msg.to_string())
    }
}

#[tauri::command]
pub async fn open_video_window(
    _app: tauri::AppHandle,
    state: State<'_, AppState>,
    file_id: i64,
    title: String,
    start_position_sec: Option<f64>,
) -> Result<(), String> {
    info!(
        "open_video_window -> native player (file_id={}, title={}, start={:?})",
        file_id, title, start_position_sec
    );
    #[cfg(feature = "player")]
    {
        ensure_video_bridge_child_for_player(state.player_ctx.as_ref()).await.map(|_| ())?;
        nativeplayer::open_in_native_player(
            state.player_ctx.as_ref(),
            file_id,
            title,
            start_position_sec,
            Some(nativeplayer::MpvSessionType::Video),
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err(crate::core::error::AppError::feature_disabled("player").to_string())
    }
}
