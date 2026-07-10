#[macro_use]
pub mod debug;
#[macro_use]
pub mod bridge;
pub mod stream;
pub mod nativeplayer;
pub mod playlistbuild;

pub mod runtime;
pub mod segmentgen;
pub mod hwdec;
pub mod infrastructure;

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
pub use playlistbuild::ensure_video_playback_ready;
pub use segmentgen::get_file_part_internal;

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
    pub download_ctx: DownloadContext,
    pub byte_stream_provider: Arc<dyn ByteStreamProvider>,
}



