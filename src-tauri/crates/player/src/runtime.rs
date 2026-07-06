use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::{Child, Command},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::net::TcpStream;
use tokio::sync::{Mutex as AsyncMutex, RwLock};
use tokio::time::{sleep, timeout};

use crate::PlayerContext;
use tracing::{info, warn};
use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::provider::storage::PartMetadata;
use crate::{
    PartSingleFlight, SegmentTelemetry,
    VideoIndexer, SparseCache,
};

const GLOBAL_VIDEO_BRIDGE_PROCESS_KEY: &str = "__global_video_bridge__";
const BRIDGE_READY_ATTEMPTS: usize = 40;
const BRIDGE_READY_DELAY_MS: u64 = 100;

#[derive(Clone, Copy, Debug)]
struct RecentSeekTarget {
    pts_ms: u64,
    recorded_at: Instant,
    source: SeekSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeekSource {
    Heuristic, // Bridge byte-range detection (fast but imprecise)
    Confirmed, // Monitor position jump or UI command (accurate)
}

#[derive(Clone)]
pub struct PlayerRuntime {
    pub sparse_cache: Arc<SparseCache>,
    pub part_singleflight: Arc<PartSingleFlight>,

    pub segment_telemetry: Arc<SegmentTelemetry>,
    pub video_indexer: Arc<VideoIndexer>,
    pub original_part_index: Arc<RwLock<HashMap<i64, Arc<HashMap<u32, PartMetadata>>>>>,
    recent_seek_targets: Arc<RwLock<HashMap<i64, RecentSeekTarget>>>,
    pub active_playback_windows: Arc<std::sync::Mutex<HashSet<String>>>,
    pub video_bridge_processes: Arc<std::sync::Mutex<HashMap<String, std::process::Child>>>,
    pub download_semaphore: Arc<tokio::sync::Semaphore>,
    pub prefetch_ahead: u32,
    pub in_flight_coordinator: Arc<AsyncMutex<HashSet<(i64, u32)>>>,
}

impl PlayerRuntime {
    pub fn new(cfg: &Config) -> Self {
        Self {
            sparse_cache: Arc::new(SparseCache::new(cfg.playback_buffer_ram_bytes as usize)),
            part_singleflight: Arc::new(PartSingleFlight::default()),
            segment_telemetry: Arc::new(SegmentTelemetry::default()),
            video_indexer: Arc::new(VideoIndexer::default()),
            original_part_index: Arc::new(RwLock::new(HashMap::new())),
            recent_seek_targets: Arc::new(RwLock::new(HashMap::new())),
            active_playback_windows: Arc::new(std::sync::Mutex::new(HashSet::new())),
            video_bridge_processes: Arc::new(std::sync::Mutex::new(HashMap::new())),
            download_semaphore: Arc::new(tokio::sync::Semaphore::new(3)),
            prefetch_ahead: cfg.prefetch_chunks,
            in_flight_coordinator: Arc::new(AsyncMutex::new(HashSet::new())),
        }
    }

    pub(crate) async fn clear_playback_cache(&self) {
        self.sparse_cache.clear();
        self.original_part_index.write().await.clear();
        self.recent_seek_targets.write().await.clear();
    }

    pub fn start_idle_gc(self: &Arc<Self>) {
        let this = self.clone();
        tokio::spawn(async move {
            let check_interval = Duration::from_secs(60);
            let idle_timeout = Duration::from_secs(300);
            loop {
                tokio::time::sleep(check_interval).await;
                let empty = this.active_playback_windows.lock().expect("Mutex poisoned").is_empty();
                if empty {
                    tokio::time::sleep(idle_timeout).await;
                    let still_empty = this.active_playback_windows.lock().expect("Mutex poisoned").is_empty();
                    if still_empty {
                        this.clear_playback_cache().await;
                    }
                }
            }
        });
    }

    pub async fn cache_original_parts(&self, file_id: i64, parts: Vec<PartMetadata>) {
        let mut cache = self.original_part_index.write().await;
        let existing = cache
            .get(&file_id)
            .map(|parts| parts.as_ref().clone())
            .unwrap_or_default();
        let mut by_part = HashMap::with_capacity(existing.len() + parts.len());
        by_part.extend(existing);
        for part in parts {
            by_part.insert(part.part_index, part);
        }
        cache.insert(file_id, Arc::new(by_part));
    }

    pub async fn get_original_part(&self, file_id: i64, part_num: u32) -> Option<PartMetadata> {
        let cache = self.original_part_index.read().await;
        cache
            .get(&file_id)
            .and_then(|parts| parts.get(&part_num))
            .cloned()
    }

    pub async fn get_all_original_parts(&self, file_id: i64) -> Option<Arc<HashMap<u32, PartMetadata>>> {
        self.original_part_index.read().await.get(&file_id).cloned()
    }

    pub async fn get_first_original_part(&self, file_id: i64) -> Option<PartMetadata> {
        let cache = self.original_part_index.read().await;
        cache
            .get(&file_id)
            .and_then(|parts| {
                parts
                    .iter()
                    .min_by_key(|(part_num, _)| *part_num)
                    .map(|(_, part)| part)
            })
            .cloned()
    }

    pub async fn record_recent_seek_target(&self, file_id: i64, position_sec: f64) {
        self.record_seek_internal(file_id, position_sec, SeekSource::Confirmed).await;
    }

    pub async fn record_recent_seek_target_heuristic(&self, file_id: i64, position_sec: f64) {
        self.record_seek_internal(file_id, position_sec, SeekSource::Heuristic).await;
    }

    async fn record_seek_internal(&self, file_id: i64, position_sec: f64, source: SeekSource) {
        if !position_sec.is_finite() || position_sec < 0.0 {
            if std::env::var("DEBUG").is_ok() {
                info!("[seek] record: file={} skipped (invalid pos={})", file_id, position_sec);
            }
            return;
        }

        let pts_ms = (position_sec * 1000.0).round().max(0.0) as u64;

        let mut targets = self.recent_seek_targets.write().await;

        // Only overwrite if: no existing target, OR new source is Confirmed, OR existing is also Heuristic
        let should_record = match targets.get(&file_id) {
            None => true,
            Some(existing) => {
                source == SeekSource::Confirmed || existing.source == SeekSource::Heuristic
            }
        };

        if should_record {
            if std::env::var("DEBUG").is_ok() {
                info!("[seek] record: file={} pts_ms={} (pos={}s) source={:?}", file_id, pts_ms, position_sec, source);
            }
            targets.insert(
                file_id,
                RecentSeekTarget {
                    pts_ms,
                    recorded_at: Instant::now(),
                    source,
                },
            );
        } else if std::env::var("DEBUG").is_ok() {
            info!("[seek] record: file={} skipped (existing Confirmed target)", file_id);
        }
    }

    pub async fn peek_recent_seek_target(&self, file_id: i64, max_age: Duration) -> Option<u64> {
        let targets = self.recent_seek_targets.read().await;
        let target = targets.get(&file_id).copied()?;
        if target.recorded_at.elapsed() > max_age {
            return None;
        }
        Some(target.pts_ms)
    }

    pub async fn take_recent_seek_target(&self, file_id: i64, max_age: Duration) -> Option<u64> {
        let mut targets = self.recent_seek_targets.write().await;
        let target = targets.remove(&file_id)?;
        if target.recorded_at.elapsed() > max_age {
            if std::env::var("DEBUG").is_ok() {
                info!("[seek] take: file={} expired (age={:?} > max={:?})", file_id, target.recorded_at.elapsed(), max_age);
            }
            return None;
        }
        if std::env::var("DEBUG").is_ok() {
            info!("[seek] take: file={} → Some({})", file_id, target.pts_ms);
        }
        Some(target.pts_ms)
    }

    pub async fn clear_hot_file_state(&self, file_id: i64) {
        self.original_part_index.write().await.remove(&file_id);
        self.recent_seek_targets.write().await.remove(&file_id);
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
        let addr = std::net::SocketAddr::from((probe_ip, bridge_port));
        if timeout(Duration::from_millis(150), TcpStream::connect(&addr)).await.is_ok() {
            return Ok(bridge_port);
        }

        // Scan fallback ports if configured port didn't respond
        for offset in 1..max_scan {
            let port = bridge_port + offset;
            let addr = std::net::SocketAddr::from((probe_ip, port));
            if timeout(Duration::from_millis(20), TcpStream::connect(&addr)).await.is_ok() {
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
