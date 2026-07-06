use crate::downloader;
use std::path::{Path, PathBuf};

pub struct ImportResult {
    pub metadata: downloader::Metadata,
    pub video_path: PathBuf,
    pub audio_path: Option<PathBuf>,
    pub total_bytes: u64,
}

fn sanitize_filename(s: &str) -> String {
    let invalid = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];
    let safe: String = s.chars()
        .map(|c| if invalid.contains(&c) { ' ' } else { c })
        .collect();
    let joined = safe.split_whitespace().collect::<Vec<_>>().join(" ");
    let truncated: String = joined.chars().take(100).collect();
    if truncated.is_empty() { "untitled".to_string() } else { truncated }
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

    let safe_title = sanitize_filename(&metadata.title);
    let video_ext = metadata.ext.as_deref().unwrap_or("mp4");
    let video_path = temp_dir.join(format!("{}.{}", safe_title, video_ext));
    download_single(url, cookies_browser, "bestvideo", &video_path).await?;

    let audio_ext = if video_ext == "webm" { "webm" } else { "m4a" };
    let audio_path = temp_dir.join(format!("{}.{}", safe_title, audio_ext));
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
