use serde::Serialize;
use omega_drive_updater::manifest::UpdaterManifest;
use omega_drive_updater::downloader::download_file;
use omega_drive_updater::verifier::verify_blake3;
use omega_drive_updater::extractor::extract_archive;
use std::path::PathBuf;

// ponytail: hardcoded URL, make configurable when multi-env support needed
const MANIFEST_URL: &str = "https://raw.githubusercontent.com/lotungtoe/Omega-Drive/main/updater.json";

#[derive(Debug, Clone, Serialize)]
pub struct BinaryStatus {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub exists: bool,
    pub update_available: bool,
}

fn binaries_dir() -> PathBuf {
    let mut dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    dir.push("binaries");
    dir
}

fn get_binary_version(name: &str) -> Option<String> {
    let path = omega_drive_gateway::updater::path::resolve_binary_path(name);
    if !path.exists() {
        return None;
    }
    let output = std::process::Command::new(&path).arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ponytail: simple version extraction per binary, add semver comparison later
    match name {
        "ffmpeg" | "ffprobe" => {
            let first = stdout.lines().next()?;
            let ver = first.split_whitespace().nth(2)?;
            Some(ver.trim_start_matches('n').to_string())
        }
        "yt-dlp" => Some(stdout.trim().to_string()),
        "deno" => {
            let first = stdout.lines().next()?;
            Some(first.split_whitespace().nth(1)?.to_string())
        }
        _ => None,
    }
}

#[tauri::command]
pub fn check_app_update(app: tauri::AppHandle) -> Result<Option<serde_json::Value>, String> {
    let version = app.package_info().version.to_string();
    Ok(Some(serde_json::json!({ "current_version": version })))
}

#[tauri::command]
pub fn get_binary_status() -> Result<Vec<BinaryStatus>, String> {
    let binary_names = ["ffmpeg", "ffprobe", "yt-dlp", "deno"];
    let statuses = binary_names.map(|name| {
        let path = omega_drive_gateway::updater::path::resolve_binary_path(name);
        let exists = path.exists();
        BinaryStatus {
            name: name.to_string(),
            version: if exists { get_binary_version(name) } else { None },
            path: if exists { Some(path.display().to_string()) } else { None },
            exists,
            update_available: false,
        }
    });
    Ok(statuses.to_vec())
}

#[tauri::command]
pub async fn check_binary_updates(app: tauri::AppHandle) -> Result<Vec<BinaryStatus>, String> {
    let manifest = UpdaterManifest::fetch(MANIFEST_URL).await?;
    let current_version = app.package_info().version.to_string();
    let platform_info = manifest
        .for_current_platform()
        .ok_or_else(|| format!("No entry for platform {}", UpdaterManifest::current_platform()))?;
    let new_version_available = platform_info.version != current_version;

    let binary_names = ["ffmpeg", "ffprobe", "yt-dlp", "deno"];
    let statuses = binary_names.map(|name| {
        let path = omega_drive_gateway::updater::path::resolve_binary_path(name);
        let exists = path.exists();
        BinaryStatus {
            name: name.to_string(),
            version: if exists { get_binary_version(name) } else { None },
            path: if exists { Some(path.display().to_string()) } else { None },
            exists,
            update_available: new_version_available,
        }
    });
    Ok(statuses.to_vec())
}

#[tauri::command]
pub async fn download_binary_update(name: String) -> Result<String, String> {
    let manifest = UpdaterManifest::fetch(MANIFEST_URL).await?;
    let platform_info = manifest
        .for_current_platform()
        .ok_or_else(|| format!("No entry for platform {}", UpdaterManifest::current_platform()))?;
    let entry = platform_info
        .binaries
        .get(&name)
        .ok_or_else(|| format!("No binary '{name}' in manifest"))?;

    let bin_dir = binaries_dir();
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Cannot create binaries dir: {e}"))?;

    let ext = entry.url.rsplit('.').next().unwrap_or("tmp");
    let tmp_path = bin_dir.join(format!(".{name}.download.{ext}"));

    download_file(&entry.url, &tmp_path, |_, _| {})
        .await
        .map_err(|e| format!("Download failed for {name}: {e}"))?;

    verify_blake3(&tmp_path, &entry.checksum)
        .map_err(|e| format!("Checksum failed for {name}: {e}"))?;

    extract_archive(&tmp_path, &bin_dir)
        .map_err(|e| format!("Extract failed for {name}: {e}"))?;

    let _ = std::fs::remove_file(&tmp_path);
    Ok(format!("{name} updated to version {}", platform_info.version))
}
