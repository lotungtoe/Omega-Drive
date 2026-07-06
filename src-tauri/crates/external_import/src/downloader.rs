use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};

use omega_drive_gateway::updater::path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub id: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub ext: Option<String>,
    pub filesize_approx: Option<u64>,
    pub webpage_url: String,
}

pub async fn get_metadata(url: &str, cookies_browser: Option<&str>) -> Result<Metadata, String> {
    let mut cmd = tokio::process::Command::new(path::resolve_binary_path("yt-dlp"));
    omega_drive_gateway::suppress_console!(&mut cmd);
    let deno = path::deno_path();
    if std::env::var("DEBUG").is_ok() { cmd.arg("-v"); }
    if let Some(browser) = cookies_browser { cmd.arg("--cookies-from-browser").arg(browser); }
    let output = cmd
        .arg("--js-runtimes").arg(format!("deno:{}", deno.display()))
        .arg("-J").arg("--flat-playlist").arg(url)
        .stdout(Stdio::piped()).stderr(Stdio::piped())
        .output().await
        .map_err(|e| format!("Failed to run downloader: {}", e))?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    tracing::debug!("[downloader] Full stderr output:\n{}", stderr);
    if stderr.contains("deno") || stderr.contains("Deno") {
        tracing::debug!("[downloader] Deno runtime detected and loaded successfully!");
    } else {
        tracing::debug!("[downloader] Deno runtime NOT detected in stderr check.");
    }
    if !output.status.success() {
        let err_line = stderr.lines()
            .find(|l| l.contains("ERROR:"))
            .map(|l| l.trim())
            .unwrap_or("unknown error");
        return Err(format!("Downloader error: {}", err_line));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse downloader JSON metadata: {}", e))
}

pub async fn download_video<F>(url: &str, output_path: &Path, on_progress: F, cookies_browser: Option<&str>) -> Result<(), String>
where F: Fn(f64, String) + Send + 'static,
{
    let mut cmd = tokio::process::Command::new(path::resolve_binary_path("yt-dlp"));
    omega_drive_gateway::suppress_console!(&mut cmd);
    let ffmpeg_dir = path::ffmpeg_path().parent()
        .ok_or_else(|| "FFmpeg path has no parent directory".to_string())?.to_path_buf();
    if let Some(browser) = cookies_browser { cmd.arg("--cookies-from-browser").arg(browser); }
    cmd.arg("--js-runtimes").arg(format!("deno:{}", path::deno_path().display()))
        .arg("--ffmpeg-location").arg(ffmpeg_dir)
        .arg("-o").arg(output_path)
        .arg("--progress-template").arg("download:%(progress._percent_str)s")
        .arg(url)
        .stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn downloader: {}", e))?;
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        if line.starts_with("download:") {
            let percent_str = line.replace("download:", "").replace('%', "").trim().to_string();
            if let Ok(percent) = percent_str.parse::<f64>() {
                on_progress(percent, "Downloading...".to_string());
            }
        }
    }
    let status = child.wait().await.map_err(|e| format!("Failed to wait for downloader: {}", e))?;
    if !status.success() { Err("Download failed".to_string()) } else { Ok(()) }
}

pub struct DownloadStream {
    pub reader: tokio::process::ChildStdout,
    pub child: tokio::process::Child,
    pub output_ext: String,
}

const MERGE_OUTPUT_FORMAT: &str = "mp4";

pub async fn download_stream(url: &str, cookies_browser: Option<&str>) -> Result<DownloadStream, String> {
    let mut cmd = tokio::process::Command::new(path::resolve_binary_path("yt-dlp"));
    omega_drive_gateway::suppress_console!(&mut cmd);
    let deno = path::deno_path();
    tracing::debug!("[downloader] Streaming: {} -o - --no-part {}", path::resolve_binary_path("yt-dlp").display(), url);
    let ffmpeg_dir = path::ffmpeg_path().parent()
        .ok_or_else(|| "FFmpeg path has no parent directory".to_string())?.to_path_buf();
    if let Some(browser) = cookies_browser { cmd.arg("--cookies-from-browser").arg(browser); }
    let mut child = cmd
        .arg("--js-runtimes").arg(format!("deno:{}", deno.display()))
        .arg("--ffmpeg-location").arg(ffmpeg_dir)
        .arg("-f").arg("bestvideo+bestaudio/best")
        .arg("--merge-output-format").arg(MERGE_OUTPUT_FORMAT)
        .arg("-o").arg("-").arg("--no-part")
        .arg(url)
        .stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Failed to spawn yt-dlp: {}", e))?;
    let reader = child.stdout.take().ok_or("Failed to capture stdout")?;
    Ok(DownloadStream { reader, child, output_ext: MERGE_OUTPUT_FORMAT.into() })
}

/// Probe which browsers have cookies available for yt-dlp on this machine.
/// Checks the platform-specific cookie database paths for each browser.
/// Returns sorted list of browser names (matches yt-dlp's --cookies-from-browser names).
pub fn probe_installed_browsers() -> Vec<String> {
    let mut found = Vec::new();

    #[cfg(target_os = "windows")]
    {
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);

        if let Some(ref base) = local {
            check_cookies(base, r"Google\Chrome\User Data\Default\Network\Cookies", "chrome", &mut found);
            check_cookies(base, r"Chromium\User Data\Default\Network\Cookies", "chromium", &mut found);
            check_cookies(base, r"Microsoft\Edge\User Data\Default\Network\Cookies", "edge", &mut found);
            check_cookies(base, r"BraveSoftware\Brave-Browser\User Data\Default\Network\Cookies", "brave", &mut found);
            check_cookies(base, r"Vivaldi\User Data\Default\Network\Cookies", "vivaldi", &mut found);
            check_cookies(base, r"Naver\NAVER Whale\User Data\Default\Network\Cookies", "whale", &mut found);
        }
        if let Some(ref base) = appdata {
            check_cookies(base, r"Opera Software\Opera Stable\Network\Cookies", "opera", &mut found);
            find_firefox_profiles(&base.join(r"Mozilla\Firefox\Profiles"), &mut found);
            find_firefox_profiles(
                &base.join(r"..\Local\Packages\Mozilla.Firefox_n80bbvh6b1yt2\LocalCache\Roaming\Mozilla\Firefox\Profiles"),
                &mut found,
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            let support = home.join("Library/Application Support");
            check_cookies(&support, "Google/Chrome/Default/Network/Cookies", "chrome", &mut found);
            check_cookies(&support, "Chromium/Default/Network/Cookies", "chromium", &mut found);
            check_cookies(&support, "Microsoft Edge/Default/Network/Cookies", "edge", &mut found);
            check_cookies(&support.join("BraveSoftware"), "Brave-Browser/Default/Network/Cookies", "brave", &mut found);
            check_cookies(&support, "Vivaldi/Default/Network/Cookies", "vivaldi", &mut found);
            check_cookies(&support.join("Naver"), "Whale/Default/Network/Cookies", "whale", &mut found);
            check_cookies(&support.join("com.operasoftware.Opera"), "Network/Cookies", "opera", &mut found);
            find_firefox_profiles(&support.join("Firefox/Profiles"), &mut found);
            find_firefox_profiles(
                &support.join("Mozilla/firefox/Profiles"),
                &mut found,
            );
            // Safari
            let safari_paths = [
                home.join("Library/Cookies/Cookies.binarycookies"),
                home.join("Library/Containers/com.apple.Safari/Data/Library/Cookies/Cookies.binarycookies"),
            ];
            if safari_paths.iter().any(|p| p.exists()) {
                found.push("safari".to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = home_dir() {
            let config = home.join(".config");
            check_cookies(&config, "google-chrome/Default/Network/Cookies", "chrome", &mut found);
            check_cookies(&config, "chromium/Default/Network/Cookies", "chromium", &mut found);
            check_cookies(&config, "microsoft-edge/Default/Network/Cookies", "edge", &mut found);
            check_cookies(&config.join("BraveSoftware"), "Brave-Browser/Default/Network/Cookies", "brave", &mut found);
            check_cookies(&config, "vivaldi/Default/Network/Cookies", "vivaldi", &mut found);
            check_cookies(&config, "naver-whale/Default/Network/Cookies", "whale", &mut found);
            check_cookies(&config, "opera/Network/Cookies", "opera", &mut found);
            find_firefox_profiles(&home.join(".mozilla/firefox"), &mut found);
            if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
                find_firefox_profiles(&PathBuf::from(xdg).join("mozilla/firefox"), &mut found);
            }
        }
    }

    found.sort();
    found.dedup();
    found
}

fn check_cookies(base: &Path, relative: &str, browser_name: &str, found: &mut Vec<String>) {
    if base.join(relative).exists() {
        found.push(browser_name.to_string());
    }
}

fn find_firefox_profiles(profiles_dir: &Path, found: &mut Vec<String>) {
    if !profiles_dir.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.contains("default"))
                && path.join("cookies.sqlite").exists()
            {
                found.push("firefox".to_string());
                return;
            }
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
