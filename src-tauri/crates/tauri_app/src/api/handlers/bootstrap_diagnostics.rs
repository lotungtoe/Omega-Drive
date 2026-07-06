use serde_json::Value;

use crate::{
    app_runtime::{AppState, UiHeartbeatStatus},
    core::error::AppResult,
    infrastructure::diagnostics::helpers::collect_bootstrap_status,
};

#[tauri::command]
pub async fn get_bootstrap_status(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let snapshot = collect_bootstrap_status(st.inner()).await;
    Ok(serde_json::to_value(snapshot)?)
}

/// ÄÆ°á»£c gá»i tá»« frontend CHá»ˆ khi tráº¡ng thĂ¡i visibility/focus thay Ä‘á»•i.
/// KhĂ´ng spam log â€” chá»‰ ghi nhá»› Ä‘á»ƒ quyáº¿t Ä‘á»‹nh cĂ³ hiá»ƒn thá»‹ system notification khĂ´ng.
#[tauri::command]
pub async fn report_ui_visibility(
    state: tauri::State<'_, AppState>,
    window_label: String,
    visible: bool,
    focused: bool,
) -> AppResult<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    tracing::debug!(
        "[UI Visibility] window={} visible={} focused={}",
        window_label,
        visible,
        focused
    );

    state
        .ui_last_heartbeat
        .store(now, std::sync::atomic::Ordering::Relaxed);

    if let Ok(mut heartbeats) = state.ui_heartbeats.lock() {
        heartbeats.insert(
            window_label,
            UiHeartbeatStatus {
                last_seen_epoch_secs: now,
                visible,
                focused,
                context: String::new(),
            },
        );
    }
    Ok(())
}
