use crate::downloader;
use std::path::{Path, PathBuf};

pub struct ImportResult {
    pub metadata: downloader::Metadata,
    pub video_path: PathBuf,
    pub audio_path: Option<PathBuf>,
    pub total_bytes: u64,
}

pub async fn start_import_stream(
    url: &str,
    cookies_browser: Option<&str>,
    base_dir: &Path,
) -> Result<ImportResult, String> {
    let metadata = downloader::get_metadata(url, cookies_browser).await?;

    let temp_dir = base_dir.join("temp");
    tokio::fs::create_dir_all(&temp_dir).await
        .map_err(|e| format!("Failed to create temp dir: {}", e))?;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();

    let video_path = temp_dir.join(format!("{}_video.mp4", nonce));
    download_single(url, cookies_browser, "bestvideo", &video_path).await?;

    let audio_path = temp_dir.join(format!("{}_audio.m4a", nonce));
    let audio_path = match download_single(url, cookies_browser, "bestaudio", &audio_path).await {
        Ok(_) => Some(audio_path),
        Err(e) => {
            tracing::warn!("[import] No audio stream: {}", e);
            None
        }
    };

    let total_bytes = metadata.filesize_approx.unwrap_or(0);
    Ok(ImportResult { metadata, video_path, audio_path, total_bytes })
}

async fn download_single(
    url: &str,
    cookies_browser: Option<&str>,
    format: &str,
    output_path: &Path,
) -> Result<(), String> {
    let mut cmd = tokio::process::Command::new(omega_drive_gateway::updater::path::resolve_binary_path("yt-dlp"));
    omega_drive_gateway::suppress_console!(&mut cmd);
    if let Some(browser) = cookies_browser { cmd.arg("--cookies-from-browser").arg(browser); }
    cmd.arg("--js-runtimes").arg(format!("deno:{}", omega_drive_gateway::updater::path::deno_path().display()))
        .arg("-f").arg(format)
        .arg("-o").arg(output_path)
        .arg("--no-part")
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let status = cmd.status().await
        .map_err(|e| format!("Failed to spawn yt-dlp for {}: {}", format, e))?;
    if !status.success() {
        return Err(format!("yt-dlp {} download failed", format));
    }
    Ok(())
}
