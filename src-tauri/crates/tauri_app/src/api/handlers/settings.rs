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

    let validated_config_value = serde_json::to_value(&cfg).map_err(|e| {
        wrap_error(
            "settings",
            codes::E_JSON,
            "Error serializing configuration.",
            ctx.clone(),
            e,
        )
    })?;

    // Merge validated providers into raw config (fills missing fields like batch_size)
    if let Some(validated_providers) = validated_config_value.get("providers") {
        merge_providers(&mut config_val, validated_providers);
    }

    Ok(json!({
        "config": config_val,
        "env": json!({})
    }))
}

/// Deep-merge validated provider defaults into raw config (only fills missing keys).
fn merge_providers(raw: &mut Value, validated: &Value) {
    let Some(validated_providers) = validated.as_object() else { return };
    if !raw.is_object() {
        return;
    }
    let raw_providers = raw
        .as_object_mut()
        .expect("just checked is_object()")
        .entry("providers")
        .or_insert_with(|| json!({}));
    let Some(raw_providers) = raw_providers.as_object_mut() else { return };

    for (provider_id, validated_provider_value) in validated_providers {
        let Some(validated_provider) = validated_provider_value.as_object() else { continue };
        let raw_provider = raw_providers
            .entry(provider_id.clone())
            .or_insert_with(|| json!({}));
        let Some(raw_provider) = raw_provider.as_object_mut() else { continue };

        for (section_name, validated_section_value) in validated_provider {
            if section_name != "transfer" && section_name != "retry" && section_name != "limits" {
                continue;
            }
            let Some(validated_section) = validated_section_value.as_object() else { continue };
            let raw_section = raw_provider
                .entry(section_name.clone())
                .or_insert_with(|| json!({}));
            let Some(raw_section) = raw_section.as_object_mut() else { continue };
            for (field_name, field_value) in validated_section {
                raw_section.entry(field_name.clone()).or_insert_with(|| field_value.clone());
            }
        }
    }
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
