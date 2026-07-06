use serde_json::{json, Value};

use crate::{
    app_runtime::AppState,
    core::{
        error::{report, wrap_error, AppError, AppResult},
        error_codes as codes,
    },
    infrastructure::feature_log::FEATURE_KEYS,
};

#[tauri::command]
pub async fn create_feature_log_file(
    st: tauri::State<'_, AppState>,
    feature: String,
) -> AppResult<Value> {
    let ctx = json!({
        "feature": "diagnostics",
        "action": "create_feature_log_file",
        "target_feature": feature.clone(),
    });
    if !FEATURE_KEYS.contains(&feature.as_str()) {
        let err = AppError::new(codes::E_INVALID_INPUT, "Unknown feature").with_context(ctx);
        return Err(report("diagnostics", err));
    }
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
    let path = log_dir.join(format!("{feature}.log"));
    if !path.exists() {
        tokio::fs::write(&path, "").await.map_err(|e| {
            wrap_error(
                "diagnostics",
                codes::E_IO,
                "Khong the tao file log.",
                ctx.clone(),
                e,
            )
        })?;
    }
    Ok(json!({ "path": path.display().to_string() }))
}

#[tauri::command]
pub async fn get_log_status(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let log_dir = st.base_dir.join("logs");
    let mut status = serde_json::Map::new();
    for feature in FEATURE_KEYS {
        let exists = log_dir.join(format!("{feature}.log")).exists();
        status.insert(feature.to_string(), json!(exists));
    }
    Ok(Value::Object(status))
}

#[tauri::command]
pub async fn log_frontend_event(
    st: tauri::State<'_, AppState>,
    feature: String,
    level: String,
    message: String,
    context: Option<Value>,
) -> AppResult<Value> {
    if !st.feature_logs.frontend_enabled {
        return Ok(json!({ "ignored": true }));
    }
    if !st.feature_logs.is_enabled(&feature) {
        return Ok(json!({ "ignored": true }));
    }

    let ctx = context.unwrap_or(Value::Null);
    let lvl = level.to_lowercase();
    match feature.as_str() {
        "upload" => log_frontend_with_target("feature::upload", &lvl, &message, &ctx),
        "download" => log_frontend_with_target("feature::download", &lvl, &message, &ctx),
        "player" => log_frontend_with_target("feature::player", &lvl, &message, &ctx),
        "drive" => log_frontend_with_target("feature::drive", &lvl, &message, &ctx),
        "settings" => log_frontend_with_target("feature::settings", &lvl, &message, &ctx),
        "diagnostics" => log_frontend_with_target("feature::diagnostics", &lvl, &message, &ctx),
        _ => log_frontend_with_target("feature::unknown", &lvl, &message, &ctx),
    }

    Ok(json!({ "ok": true }))
}

fn log_frontend_with_target(target: &str, level: &str, message: &str, context: &Value) {
    let frontend_target = target;
    match level {
        "error" => tracing::error!(
            target: "frontend",
            frontend_target,
            message = message,
            context = ?context
        ),
        "warn" | "warning" => tracing::warn!(
            target: "frontend",
            frontend_target,
            message = message,
            context = ?context
        ),
        "debug" => tracing::debug!(
            target: "frontend",
            frontend_target,
            message = message,
            context = ?context
        ),
        _ => tracing::info!(
            target: "frontend",
            frontend_target,
            message = message,
            context = ?context
        ),
    }
}
