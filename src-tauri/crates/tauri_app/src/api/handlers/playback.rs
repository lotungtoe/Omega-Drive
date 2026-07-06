use serde_json::{json, Value};
use tracing::info;

use crate::{
    app_runtime::AppState,
    core::{
        error::{wrap_error, AppError, AppResult},
        error_codes as codes,
    },
    db::files as db_files,
};

const PLAYBACK_RESUME_MIN_SECS: f64 = 10.0;

fn playback_completion_tail(duration_sec: f64) -> f64 {
    (duration_sec * 0.1).clamp(3.0, 20.0)
}

fn playback_resume_eligible(position_sec: f64, duration_sec: Option<f64>) -> bool {
    if !position_sec.is_finite() || position_sec < PLAYBACK_RESUME_MIN_SECS {
        return false;
    }

    if let Some(duration_sec) = duration_sec.filter(|value| value.is_finite() && *value > 0.0) {
        position_sec < duration_sec - playback_completion_tail(duration_sec)
    } else {
        true
    }
}

#[tauri::command]
pub async fn get_playback_position(
    st: tauri::State<'_, AppState>,
    file_id: i64,
) -> AppResult<Value> {
    let ctx = json!({
        "feature": "player",
        "action": "get_playback_position",
        "file_id": file_id,
    });
    let db = st.db_read.lock().await;
    let playback = db_files::get_effective_video_playback(db.conn(), file_id).map_err(|e| {
        wrap_error(
            "player",
            codes::E_DB,
            "Lá»—i DB khi láº¥y vá»‹ trĂ­ phĂ¡t láº¡i.",
            ctx.clone(),
            e,
        )
    })?;

    let Some(playback) = playback else {
        return Ok(Value::Null);
    };

    Ok(json!({
        "fileId": playback.file_id,
        "positionSec": playback.position_sec,
        "durationSec": playback.duration_sec,
        "resumePartIndex": playback.resume_part_index,
        "resumeEligible": playback_resume_eligible(playback.position_sec, playback.duration_sec),
    }))
}

#[tauri::command]
pub async fn save_playback_position(
    st: tauri::State<'_, AppState>,
    file_id: i64,
    position_sec: f64,
    duration_sec: Option<f64>,
    completed: Option<bool>,
) -> AppResult<Value> {
    let ctx = json!({
        "feature": "player",
        "action": "save_playback_position",
        "file_id": file_id,
    });
    let db = st.db_write.lock().await;
    db_files::save_playback_history(
        db.conn(),
        file_id,
        position_sec,
        duration_sec,
        completed.unwrap_or(false),
    )
    .map_err(|e| {
        wrap_error(
            "player",
            codes::E_DB,
            "Lá»—i DB khi lÆ°u playback history.",
            ctx.clone(),
            e,
        )
    })?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn clear_playback_position(
    st: tauri::State<'_, AppState>,
    file_id: i64,
) -> AppResult<Value> {
    let ctx = json!({
        "feature": "player",
        "action": "clear_playback_position",
        "file_id": file_id,
    });
    let db = st.db_write.lock().await;
    db_files::clear_playback_history(db.conn(), file_id).map_err(|e| {
        wrap_error(
            "player",
            codes::E_DB,
            "Lá»—i DB khi xĂ³a playback history.",
            ctx.clone(),
            e,
        )
    })?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn get_video_player_config(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let cfg = st.cfg.read().expect("cfg RwLock");
    let providers = cfg
        .providers
        .iter()
        .map(|(provider_id, provider)| {
            (
                provider_id.clone(),
                json!({
                    "transfer": {
                        "parallel_sends": provider.transfer.parallel_sends,
                    },
                    "retry": {
                        "send_retries": provider.retry.send_retries,
                        "retry_base_delay_s": provider.retry.retry_base_delay_s,
                    },
                    "limits": {
                        "hard_limit_mb": provider.limits.hard_limit_bytes / 1024 / 1024,
                        "file_limit_mb": provider.limits.file_limit_bytes / 1024 / 1024,
                    }
                }),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    Ok(json!({
        "providers": providers,
        "stream_buffer_kb":      cfg.read_buffer_bytes / 1024,
        "low_latency_mode":      true,
        "back_buffer_length":    90,
    }))
}

#[tauri::command]
pub async fn update_video_player_config(
    st: tauri::State<'_, AppState>,
    data: Value,
) -> AppResult<()> {
    let ctx = json!({
        "feature": "player",
        "action": "update_video_player_config",
    });
    let mut new_cfg = st.cfg.read().expect("cfg RwLock").clone();

    if let Some(providers) = data.get("providers").and_then(|value| value.as_object()) {
        for (provider_id, provider_value) in providers {
            let Some(provider) = new_cfg.providers.get_mut(provider_id) else { continue };

            if let Some(val) = provider_value
                .get("transfer")
                .and_then(|value| value.get("parallel_sends"))
                .and_then(|value| value.as_u64())
            {
                provider.transfer.parallel_sends = val as usize;
            }

            if let Some(val) = provider_value
                .get("retry")
                .and_then(|value| value.get("send_retries"))
                .and_then(|value| value.as_u64())
            {
                provider.retry.send_retries = val as u32;
            }

            if let Some(val) = provider_value
                .get("retry")
                .and_then(|value| value.get("retry_base_delay_s"))
                .and_then(|value| value.as_u64())
            {
                provider.retry.retry_base_delay_s = val;
            }

            if let Some(val) = provider_value
                .get("limits")
                .and_then(|value| value.get("hard_limit_mb"))
                .and_then(|value| value.as_u64())
            {
                provider.limits.hard_limit_bytes = val * 1024 * 1024;
            }

            if let Some(val) = provider_value
                .get("limits")
                .and_then(|value| value.get("file_limit_mb"))
                .and_then(|value| value.as_u64())
            {
                provider.limits.file_limit_bytes = val * 1024 * 1024;
            }
        }
    }

    if let Some(val) = data.get("discord_hard_limit_mb").and_then(|v| v.as_u64()) {
        let limit_bytes = val * 1024 * 1024;
        if let Some(provider) = new_cfg.providers.get_mut("discord") {
            provider.limits.hard_limit_bytes = limit_bytes;
            provider.limits.file_limit_bytes = limit_bytes;
        }
    }
    if let Some(val) = data.get("stream_buffer_kb").and_then(|v| v.as_u64()) {
        new_cfg.read_buffer_bytes = (val * 1024) as usize;
    }

    omega_drive_core::config::save_config_to_file(&new_cfg, &st.base_dir).map_err(|e| {
        wrap_error(
            "player",
            codes::E_IO,
            "KhĂ´ng thá»ƒ lÆ°u cáº¥u hĂ¬nh player.",
            ctx.clone(),
            e,
        )
    })?;

    info!("ÄĂ£ cáº­p nháº­t vĂ  lÆ°u cáº¥u hĂ¬nh Video Player má»›i.");
    Ok(())
}

#[tauri::command]
pub async fn get_bridge_port(st: tauri::State<'_, AppState>) -> AppResult<u16> {
    Ok(st.bridge_port)
}

#[tauri::command]
pub async fn prepare_audio_bridge(st: tauri::State<'_, AppState>) -> AppResult<()> {
    let bridge_res: Result<(), String> =
        omega_drive_player::runtime::ensure_video_bridge_child_for_player(
            st.player_ctx.as_ref(),
        )
        .await
        .map(|_| ());
    bridge_res.map_err(|e| {
        AppError::new(
            codes::E_PLAYER_INIT_FAILED,
            "KhĂ´ng thá»ƒ khá»Ÿi Ä‘á»™ng audio bridge.",
        )
        .with_source(e)
    })?;
    Ok(())
}
