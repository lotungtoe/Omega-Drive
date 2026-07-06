use std::path::PathBuf;

fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn dev_binaries_dir() -> Option<PathBuf> {
    Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"))
}

fn sidecar_filename(name: &str) -> String {
    let ext = std::env::consts::EXE_SUFFIX;
    let target = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        "aarch64-pc-windows-msvc"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else {
        ""
    };
    if target.is_empty() {
        format!("{name}{ext}")
    } else {
        format!("{name}-{target}{ext}")
    }
}

fn plain_binary_filename(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

pub fn resolve_binary_path(name: &str) -> PathBuf {
    let sidecar_name = sidecar_filename(name);
    let plain_name = plain_binary_filename(name);
    let mut candidates = Vec::new();
    if let Some(exe_dir) = current_exe_dir() {
        candidates.push(exe_dir.join(&sidecar_name));
        candidates.push(exe_dir.join(&plain_name));
        candidates.push(exe_dir.join("binaries").join(&sidecar_name));
        candidates.push(exe_dir.join("binaries").join(&plain_name));
    }
    if let Some(dev_dir) = dev_binaries_dir() {
        candidates.push(dev_dir.join(&sidecar_name));
        candidates.push(dev_dir.join(&plain_name));
    }
    let resolved = candidates
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from(name));
    if std::env::var("DEBUG").is_ok() {
        eprintln!("[updater::path] Resolved binary '{name}' to: {}", resolved.display());
    }
    resolved
}

pub fn ffmpeg_path() -> PathBuf {
    resolve_binary_path("ffmpeg")
}

pub fn ffprobe_path() -> PathBuf {
    resolve_binary_path("ffprobe")
}

pub fn deno_path() -> PathBuf {
    resolve_binary_path("deno")
}

#[macro_export]
macro_rules! suppress_console {
    ($cmd:expr) => {
        #[cfg(target_os = "windows")]
        {
            #[allow(unused_imports)]
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            $cmd.creation_flags(CREATE_NO_WINDOW);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = $cmd;
        }
    };
}
