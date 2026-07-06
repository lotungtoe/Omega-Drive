use std::{path::Path, process::Stdio};

use serde::Serialize;
use tokio::process::Command;

use crate::app_wiring::app_runtime::AppState;
use omega_drive_gateway::updater::path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatusSnapshot {
    pub bot_env_path: String,
    pub bot_env_exists: bool,
    pub discord_configured: bool,
    pub telegram_configured: bool,
    pub ffmpeg_ready: bool,
    pub ffprobe_ready: bool,
    pub native_player_ready: bool,
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub mpv_paths: Vec<String>,
}

async fn probe_binary(path: &Path) -> bool {
    let mut cmd = Command::new(path);
    omega_drive_gateway::suppress_console!(&mut cmd);

    cmd.arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

pub async fn collect_bootstrap_status(state: &AppState) -> BootstrapStatusSnapshot {
    let bot_env_path = state.base_dir.join("bot.env");
    let ffmpeg_path = path::ffmpeg_path();
    let ffprobe_path = path::ffprobe_path();
    let mpv_candidates = crate::app_wiring::infrastructure::runtime::mpv_runtime_candidates();
    let ffmpeg_ready = probe_binary(&ffmpeg_path).await;
    let ffprobe_ready = probe_binary(&ffprobe_path).await;

    let discord_configured = !std::env::var("DISCORD_TOKEN")
        .unwrap_or_default()
        .is_empty();
    let telegram_api_id = std::env::var("TELEGRAM_API_ID")
        .unwrap_or_default()
        .parse::<i32>()
        .unwrap_or(0);
    let telegram_configured = !std::env::var("TELEGRAM_PHONE")
        .unwrap_or_default()
        .is_empty()
        && telegram_api_id > 0
        && !std::env::var("TELEGRAM_API_HASH")
            .unwrap_or_default()
            .is_empty();

    BootstrapStatusSnapshot {
        bot_env_path: bot_env_path.display().to_string(),
        bot_env_exists: bot_env_path.exists(),
        discord_configured,
        telegram_configured,
        ffmpeg_ready,
        ffprobe_ready,
        native_player_ready: crate::app_wiring::infrastructure::runtime::native_player_runtime_ready(),
        ffmpeg_path: ffmpeg_path.display().to_string(),
        ffprobe_path: ffprobe_path.display().to_string(),
        mpv_paths: crate::app_wiring::infrastructure::runtime::existing_path_strings(&mpv_candidates),
    }
}
