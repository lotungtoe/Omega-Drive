use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::{Child, Command},
    sync::Arc,
    time::Duration,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};

use crate::PlayerContext;
use tracing::{info, warn};


const GLOBAL_VIDEO_BRIDGE_PROCESS_KEY: &str = "__global_video_bridge__";
const BRIDGE_READY_ATTEMPTS: usize = 40;
const BRIDGE_READY_DELAY_MS: u64 = 100;

#[derive(Clone)]
pub struct PlayerRuntime {
    pub active_playback_windows: Arc<std::sync::Mutex<HashSet<String>>>,
    pub video_bridge_processes: Arc<std::sync::Mutex<HashMap<String, std::process::Child>>>,
}

impl PlayerRuntime {
    pub fn new() -> Self {
        Self {
            active_playback_windows: Arc::new(std::sync::Mutex::new(HashSet::new())),
            video_bridge_processes: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }
}

const VIDEO_BRIDGE_SPAWN_RETRIES: u32 = 3;

pub async fn ensure_video_bridge_child(
    base_dir: &Path,
    bridge_port: u16,
    processes: &Arc<std::sync::Mutex<HashMap<String, Child>>>,
) -> Result<u16, String> {
    {
        let mut guard = processes
            .lock()
            .map_err(|_| "Video bridge process lock poisoned".to_string())?;
        if let Some(child) = guard.get_mut(GLOBAL_VIDEO_BRIDGE_PROCESS_KEY) {
            match child.try_wait() {
                Ok(None) => return Ok(bridge_port),
                Ok(Some(_)) => {
                    guard.remove(GLOBAL_VIDEO_BRIDGE_PROCESS_KEY);
                }
                Err(err) => {
                    return Err(format!(
                        "Failed to inspect existing video bridge child process: {err}"
                    ));
                }
            }
        }
    }

    let mut last_err = String::new();
    for attempt in 0..VIDEO_BRIDGE_SPAWN_RETRIES {
        match spawn_video_bridge_child(base_dir, bridge_port) {
            Ok(mut child) => match wait_for_video_bridge_ready(&mut child, bridge_port).await {
                Ok(actual_port) => {
                    let mut guard = processes
                        .lock()
                        .map_err(|_| "Video bridge process lock poisoned".to_string())?;
                    guard.insert(GLOBAL_VIDEO_BRIDGE_PROCESS_KEY.to_string(), child);
                    if attempt > 0 {
                        info!("[bridge] spawned successfully on attempt {}/{}", attempt + 1, VIDEO_BRIDGE_SPAWN_RETRIES);
                    }
                    return Ok(actual_port);
                }
                Err(err) => {
                    last_err = err;
                    let _ = child.kill();
                    let _ = child.wait();
                }
            },
            Err(err) => {
                last_err = err;
            }
        }

        if attempt < VIDEO_BRIDGE_SPAWN_RETRIES - 1 {
            warn!(
                "[bridge] spawn attempt {}/{} failed: {}. Retrying in {}ms...",
                attempt + 1, VIDEO_BRIDGE_SPAWN_RETRIES, last_err,
                500 * (attempt + 1)
            );
            sleep(Duration::from_millis(500 * (attempt + 1) as u64)).await;
        }
    }

    Err(format!(
        "Failed to spawn video bridge after {VIDEO_BRIDGE_SPAWN_RETRIES} attempts. Last error: {last_err}"
    ))
}

pub async fn ensure_video_bridge_child_for_player(state: &PlayerContext) -> Result<u16, String> {
    let actual_port = ensure_video_bridge_child(
        &state.base_dir,
        state.bridge_port.load(std::sync::atomic::Ordering::Relaxed),
        &state.player_runtime.video_bridge_processes,
    )
    .await?;
    state.bridge_port.store(actual_port, std::sync::atomic::Ordering::Relaxed);
    Ok(actual_port)
}

fn spawn_video_bridge_child(base_dir: &Path, bridge_port: u16) -> Result<Child, String> {
    let current_exe = std::env::current_exe()
        .map_err(|err| format!("Failed to resolve current executable for video bridge: {err}"))?;
    let mut command = Command::new(current_exe);
    command
        .current_dir(base_dir)
        .arg("--video-bridge")
        .arg("--video-bridge-port")
        .arg(bridge_port.to_string())
        .arg("--parent-pid")
        .arg(std::process::id().to_string());

    omega_drive_gateway::suppress_console!(&mut command);

    let mut child = command
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to spawn video bridge child process: {err}"))?;

    // Forward child stderr to tracing for diagnostics
    if let Some(stderr) = child.stderr.take() {
        tokio::task::spawn_blocking(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(text) => info!("[video-bridge] {}", text),
                    Err(_) => break,
                }
            }
        });
    }

    Ok(child)
}

async fn probe_bridge_port(ip: std::net::Ipv4Addr, port: u16) -> bool {
    let addr = std::net::SocketAddr::from((ip, port));
    let mut stream = match timeout(Duration::from_millis(150), TcpStream::connect(&addr)).await {
        Ok(Ok(s)) => s,
        _ => return false,
    };
    let req = "GET /player/status HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    if timeout(Duration::from_millis(100), stream.write_all(req.as_bytes())).await.is_err() {
        return false;
    }
    let mut buf = [0u8; 32];
    match timeout(Duration::from_millis(100), stream.read_exact(&mut buf[..12])).await {
        Ok(Ok(_)) => buf.starts_with(b"HTTP/1.1 200") || buf.starts_with(b"HTTP/1.0 200"),
        _ => false,
    }
}

async fn wait_for_video_bridge_ready(child: &mut Child, bridge_port: u16) -> Result<u16, String> {
    let max_scan = 100u16;
    let probe_ip = crate::infrastructure::pick_working_ip();

    for _ in 0..BRIDGE_READY_ATTEMPTS {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!(
                    "Video bridge child exited before becoming ready (status: {status})"
                ));
            }
            Ok(None) => {}
            Err(err) => {
                return Err(format!(
                    "Failed while waiting for video bridge child process: {err}"
                ));
            }
        }

        // Fast path: try configured port first
        if probe_bridge_port(probe_ip, bridge_port).await {
            return Ok(bridge_port);
        }

        // Scan fallback ports if configured port didn't respond
        for offset in 1..max_scan {
            let port = bridge_port + offset;
            if probe_bridge_port(probe_ip, port).await {
                return Ok(port);
            }
        }

        sleep(Duration::from_millis(BRIDGE_READY_DELAY_MS)).await;
    }

    Err(format!(
        "Timed out scanning ports {}-{} (probe_ip={})",
        bridge_port,
        bridge_port + max_scan - 1,
        probe_ip,
    ))
}
