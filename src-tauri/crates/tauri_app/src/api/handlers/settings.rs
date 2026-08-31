use serde_json::{json, Value};

use crate::{
    app_runtime::AppState,
    core::{
        error::{wrap_error, AppResult},
        error_codes as codes,
    },
    providers::config::builtin_provider_config_descriptors,
};

#[tauri::command]
pub async fn get_settings(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let base_dir = &st.base_dir;
    let path = base_dir.join("config.json");
    let ctx = json!({
        "feature": "settings",
        "action": "get_settings",
    });

    // Read raw file (preserves exact user structure for upload/general, etc.)
    let mut config_val: Value = if path.exists() {
        match tokio::fs::read_to_string(&path).await {
            Ok(s) => serde_json::from_str(&s).unwrap_or(json!({})),
            Err(e) => {
                eprintln!("Warning: cannot read config.json ({}), using defaults", e);
                json!({})
            }
        }
    } else {
        json!({})
    };

    // Load validated config to get provider defaults merged
    let base_dir2 = st.base_dir.clone();
    let descriptors = builtin_provider_config_descriptors();
    let cfg = tokio::task::spawn_blocking(move || omega_drive_core::config::load_config(&base_dir2, &descriptors))
        .await
        .map_err(|e| {
            wrap_error(
                "settings",
                codes::E_UNKNOWN,
                "Error loading configuration.",
                ctx.clone(),
                e,
            )
        })?;

    // Build a RawConfig-shaped defaults map from the validated Config. This
    // shape matches what the UI reads (download.http_timeout_s, upload.general
    // .chunk_mb, providers.telegram.limits.file_limit_mb, logging.feature_
    // enabled.drive, ...) and what save_config_to_file writes to disk. The
    // Config struct is flat (http_timeout_s at top level) so we cannot merge
    // it directly into the UI paths; we have to round-trip through RawConfig.
    let raw_defaults = omega_drive_core::config::raw_from_config(&cfg);
    let raw_defaults_value = serde_json::to_value(&raw_defaults).map_err(|e| {
        wrap_error(
            "settings",
            codes::E_JSON,
            "Error serialising default config.",
            ctx.clone(),
            e,
        )
    })?;

    // Fill every missing key (recursively) in the raw on-disk config from
    // the RawConfig defaults. User-set values win at every level.
    merge_grouped_defaults(&mut config_val, &raw_defaults_value);

    // Provider defaults (transfer / retry / limits) are already covered by
    // merge_grouped_defaults above - the recursive walk handles the
    // 3-level nesting just like the other groups.

    Ok(json!({
        "config": config_val,
        "env": json!({})
    }))
}

/// Deep-merge RawConfig-shaped defaults into the on-disk config (only fills
/// missing keys). Recursive: handles arbitrary nesting (download.*,
/// upload.general.*, providers.telegram.limits.*, logging.feature_enabled.*).
/// User values win at every level via the missing-key check.
fn merge_grouped_defaults(raw: &mut Value, defaults: &Value) {
    if let (Value::Object(raw_obj), Value::Object(default_obj)) = (raw, defaults) {
        for (k, default_leaf) in default_obj {
            match raw_obj.get_mut(k) {
                None => {
                    raw_obj.insert(k.clone(), default_leaf.clone());
                }
                Some(existing) => merge_grouped_defaults(existing, default_leaf),
            }
        }
    }
    // Mismatched types (existing is a leaf, default is an object, or vice
    // versa): leave the user value alone.
}

#[tauri::command]
pub async fn save_settings(st: tauri::State<'_, AppState>, config: Value) -> AppResult<Value> {
    write_config_file(&st.base_dir, &config).await?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn apply_settings(st: tauri::State<'_, AppState>, config: Value) -> AppResult<Value> {
    let ctx = json!({
        "feature": "settings",
        "action": "apply_settings",
    });

    write_config_file(&st.base_dir, &config).await?;

    let base_dir = st.base_dir.clone();
    let descriptors = builtin_provider_config_descriptors();
    let new_cfg = tokio::task::spawn_blocking(move || omega_drive_core::config::load_config(&base_dir, &descriptors))
        .await
        .map_err(|e| {
            wrap_error(
                "settings",
                codes::E_UNKNOWN,
                "Error reloading configuration.",
                ctx.clone(),
                e,
            )
        })?;
    *st.cfg.write().expect("cfg RwLock write") = new_cfg;

    Ok(json!({ "success": true }))
}

async fn write_config_file(base_dir: &std::path::Path, config: &Value) -> AppResult<()> {
    let path = base_dir.join("config.json");
    let ctx = json!({
        "feature": "settings",
        "action": "write_config",
    });

    let content = serde_json::to_string_pretty(&config).map_err(|e| {
        wrap_error(
            "settings",
            codes::E_JSON,
            "Error formatting configuration.",
            ctx.clone(),
            e,
        )
    })?;

    tokio::fs::write(&path, content).await.map_err(|e| {
        wrap_error(
            "settings",
            codes::E_IO,
            "Cannot write config.json file.",
            ctx.clone(),
            e,
        )
    })?;

    Ok(())
}

#[tauri::command]
pub async fn get_gpu_adapters() -> Vec<String> {
    tokio::task::spawn_blocking(|| {
        omega_drive_player::hwdec::enumerate_gpu_adapters()
    })
    .await
    .unwrap_or_else(|_| vec!["Auto".to_string()])
}
