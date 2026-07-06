use std::path::{Path, PathBuf};

fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
}

fn dev_binaries_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Some(manifest_dir.join("binaries"))
}

pub fn mpv_runtime_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(exe_dir) = current_exe_dir() {
        candidates.push(exe_dir.join("mpv-1.dll"));
        candidates.push(exe_dir.join("mpv.dll"));
        candidates.push(exe_dir.join("binaries").join("mpv-1.dll"));
        candidates.push(exe_dir.join("binaries").join("mpv.dll"));
    }

    if let Some(dev_dir) = dev_binaries_dir() {
        candidates.push(dev_dir.join("mpv-1.dll"));
        candidates.push(dev_dir.join("mpv.dll"));
    }

    candidates
}

pub fn native_player_runtime_ready() -> bool {
    if !cfg!(target_os = "windows") {
        return true;
    }

    mpv_runtime_candidates()
        .into_iter()
        .any(|path| path.exists())
}

pub fn existing_path_strings(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| path.exists())
        .map(|path| path.display().to_string())
        .collect()
}

pub fn path_exists(path: &Path) -> bool {
    path.exists()
}


