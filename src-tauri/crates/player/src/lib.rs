#[macro_use]
pub mod debug;
#[macro_use]
pub mod bridge;
pub mod stream;
pub mod nativeplayer;     // Controls mpv via IPC — open, play/pause, seek, volume, speed, fullscreen
pub mod playlistbuild;    // Checks if video in DB is "ready" before playback

pub mod url_cache;        // Discord CDN URL cache — lookup, resolve, persist
pub mod range_stream;     // Builds range plan — splits byte range into small part lists
pub mod runtime;          // Manages all runtime state — caches parts, keyframes, seek targets, bridge child
pub mod segment_telemetry;// Collects segment load telemetry — TTFB, bytes, retries — suggests parallelism
pub mod segmentgen;       // Generates segments from cache/cloud for streaming — slices byte range from original part
pub mod singleflight;     // Deduplicates concurrent requests — multiple threads loading same part/block call API once
pub mod idx_cache;
pub mod video_indexer;    // Scans and caches index hints — container type, critical parts — optimizes seek
pub mod hwdec;            // Enumerate GPU adapter cho hardware decode
pub mod infrastructure;   // Platform-specific ffmpeg/mpv paths, checks mpv runtime

// ─── Public API ────────────────────────────────────────────

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU16, AtomicU64},
        Arc,
    },
};

use chrono::{DateTime, Utc};
use omega_drive_download::DownloadContext;
use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::download::ByteStreamProvider;
use omega_drive_gateway::provider::{file_repository::FileRepository, stream::StreamRegistry};
use omega_drive_gateway::provider::debug_logger::DebugLogger;

// Re-exports for cross-module usage
pub use playlistbuild::{
    ensure_video_playback_ready,
};
pub use segmentgen::get_file_part_internal;
pub use segment_telemetry::SegmentTelemetry;
pub use singleflight::PlayerSingleFlight;
pub use idx_cache::IdxCache;
pub use video_indexer::VideoIndexer;

pub trait AppEventEmitter: Send + Sync {
    fn emit(&self, event: &str, payload: serde_json::Value);
}

#[derive(Clone)]
pub struct PlayerContext {
    pub player_runtime: Arc<runtime::PlayerRuntime>,
    pub bridge_port: Arc<AtomicU16>,
    pub file_repo: Arc<dyn FileRepository>,
    pub cfg: Arc<Config>,
    pub cdn_link_cache: Arc<tokio::sync::RwLock<HashMap<String, (String, DateTime<Utc>)>>>,
    pub base_dir: PathBuf,
    pub disk_semaphore: Arc<tokio::sync::Semaphore>,
    pub stream_registry: Arc<StreamRegistry>,
    pub event_emitter: Arc<dyn AppEventEmitter>,
    pub debug_logger: Arc<dyn DebugLogger>,
    pub ui_last_heartbeat: Arc<AtomicU64>,
    pub idx_cache: IdxCache,
    pub download_ctx: DownloadContext,
    pub byte_stream_provider: Arc<dyn ByteStreamProvider>,
}



