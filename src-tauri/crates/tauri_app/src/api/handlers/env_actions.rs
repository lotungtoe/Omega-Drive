use serde_json::json;

use crate::{
    app_runtime::AppState,
    core::{
        error::{wrap_error, AppResult},
        error_codes as codes,
    },
};

#[tauri::command]
pub async fn open_discord_auth() -> AppResult<()> {
    let ctx = json!({
        "feature": "diagnostics",
        "action": "open_discord_auth",
    });
    open::that("https://discord.com/developers/applications").map_err(|e| {
        wrap_error(
            "diagnostics",
            codes::E_IO,
            "Khong the mo Discord Developer Portal.",
            ctx,
            e,
        )
    })?;
    Ok(())
}

#[tauri::command]
pub async fn open_bot_env(st: tauri::State<'_, AppState>) -> AppResult<()> {
    let ctx = json!({
        "feature": "diagnostics",
        "action": "open_bot_env",
    });
    let path = st.base_dir.join("bot.env");
    if !path.exists() {
        tokio::fs::write(&path, "").await.map_err(|e| {
            wrap_error(
                "diagnostics",
                codes::E_IO,
                "Khong the tao bot.env.",
                ctx.clone(),
                e,
            )
        })?;
    }
    open::that(path)
        .map_err(|e| wrap_error("diagnostics", codes::E_IO, "Khong the mo bot.env.", ctx, e))?;
    Ok(())
}

#[tauri::command]
pub async fn open_logs_dir(st: tauri::State<'_, AppState>) -> AppResult<()> {
    let ctx = json!({
        "feature": "diagnostics",
        "action": "open_logs_dir",
    });
    let log_dir = st.base_dir.join("logs");
    tokio::fs::create_dir_all(&log_dir).await.map_err(|e| {
        wrap_error(
            "diagnostics",
            codes::E_IO,
            "Khong the tao thu muc log.",
            ctx.clone(),
            e,
        )
    })?;
    open::that(log_dir).map_err(|e| {
        wrap_error(
            "diagnostics",
            codes::E_IO,
            "Khong the mo thu muc log.",
            ctx,
            e,
        )
    })?;
    Ok(())
}

#[tauri::command]
pub async fn check_shared_drive_status(
    st: tauri::State<'_, AppState>,
) -> AppResult<crate::providers::discord_provider::SharedDriveStatus> {
    crate::providers::discord_provider::check_shared_drive_status_internal(st).await
}

#[tauri::command]
pub async fn setup_shared_drive(
    st: tauri::State<'_, AppState>,
    guild_id: String,
    tg_chat_id: String,
) -> AppResult<String> {
    crate::providers::discord_provider::setup_shared_drive_internal(st, guild_id, tg_chat_id).await
}

#[tauri::command]
pub async fn forward_file_to_shared(st: tauri::State<'_, AppState>, file_id: i64) -> AppResult<()> {
    crate::providers::discord_provider::forward_file_to_shared_internal(st, file_id).await
}
