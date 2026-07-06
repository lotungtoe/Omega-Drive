use serde_json::{json, Value};
use tracing::info;

use crate::{app_runtime::AppState, core::error::AppResult, db::files as db_files};

#[tauri::command]
pub async fn check_backend_health() -> AppResult<Value> {
    info!("đŸ“¡ [IPC Audit] Calling handler: check_backend_health");
    Ok(json!({ "ok": true }))
}

#[tauri::command]
pub async fn get_connection_status(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    info!("đŸ“¡ [IPC Audit] Calling handler: get_connection_status");
    let provider_runtime = st.provider_runtime();
    let discord_status = match provider_runtime.provider_admin_registry.get("discord") {
        Some(gateway) => gateway.connection_status().await.ok(),
        None => None,
    };
    let telegram_status = match provider_runtime.provider_admin_registry.get("telegram") {
        Some(gateway) => gateway.connection_status().await.ok(),
        None => None,
    };

    Ok(json!({
        "discord": {
            "connected": discord_status
                .as_ref()
                .map(|status| status.connected)
                .unwrap_or(false),
        },
        "telegram": {
            "connected": telegram_status
                .as_ref()
                .map(|status| status.connected)
                .unwrap_or(false),
            "authorized": telegram_status
                .as_ref()
                .map(|status| status.authorized)
                .unwrap_or(false),
        }
    }))
}

#[tauri::command]
pub async fn get_version(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let db = st.db_read.lock().await;
    let history_len = db_files::get_all_file_count(db.conn()).unwrap_or(0);
    Ok(json!({ "version": "0.1.0-omega", "history_len": history_len }))
}
