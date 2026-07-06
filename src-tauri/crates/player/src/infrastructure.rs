use std::path::PathBuf;

fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
}

fn dev_binaries_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Some(manifest_dir.join("binaries"))
}

pub fn native_player_runtime_ready() -> bool {
    if !cfg!(target_os = "windows") {
        return true;
    }
    let candidates = mpv_runtime_candidates();
    candidates.into_iter().any(|path| path.exists())
}

fn mpv_runtime_candidates() -> Vec<PathBuf> {
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

use std::net::{Ipv4Addr, SocketAddr, TcpListener};

/// Detect an IP that accepts TCP connections on this machine.
/// Prefers loopback (127.0.0.1). Falls back to LAN IP if loopback TCP is broken
/// (e.g. by VPN WFP callout drivers).
///
/// Test: bind a listener to 127.0.0.1, then check the actual `local_addr`.
/// On a healthy system the listener is on 127.0.0.1.
/// On a system where WFP redirects loopback (VPN), it lands on the LAN IP.
pub fn pick_working_ip() -> Ipv4Addr {
    if let Ok(listener) = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))) {
        if let Ok(local) = listener.local_addr() {
            if local.ip().is_loopback() {
                return Ipv4Addr::new(127, 0, 0, 1);
            }
            // Socket ended up on a different IP (broken loopback) — use that IP
            if let SocketAddr::V4(v4) = local {
                return *v4.ip();
            }
        }
    }
    // Last resort: should never reach here on any real Windows/Linux/macOS
    Ipv4Addr::new(127, 0, 0, 1)
}
