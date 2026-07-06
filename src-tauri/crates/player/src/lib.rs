#[macro_use]
pub mod debug;
#[macro_use]
pub mod bridge;
pub mod stream;
pub mod sparse;
pub mod download;           // MĂ¡y chá»§ Axum â€” nháº­n request tá»« mpv, táº£i dá»¯ liá»‡u tá»« Discord/Telegram/DB
pub mod nativeplayer;     // Điều khiển mpv qua IPC — mở, play/pause, seek, volume, speed, fullscreen
pub mod playlistbuild;    // Kiểm tra trạng thái video trong DB đã "ready" chưa trước khi phát

pub mod url_cache;        // Discord CDN URL cache â€” lookup, resolve, persist
pub mod range_stream;     // XĂ¢y dá»±ng range plan â€” chia byte range thĂ nh danh sĂ¡ch máº£nh nhá»
pub mod runtime;          // Quáº£n lĂ½ toĂ n bá»™ runtime state â€” cache part, keyframe, seek targets, bridge child
pub mod segment_telemetry;// Thu tháº­p sá»‘ liá»‡u táº£i segment â€” TTFB, bytes, retries â€” Ä‘á» xuáº¥t Ä‘á»™ song song
pub mod segmentgen;       // Sinh segment tá»« cache/cloud cho streaming â€” slice byte range tá»« part gá»‘c
pub mod singleflight;     // Gá»™p request trĂ¹ng â€” nhiá»u luá»“ng cĂ¹ng táº£i 1 part/block thĂ¬ chá»‰ gá»i 1 láº§n
pub mod idx_cache;
pub mod video_indexer;    // DĂ² vĂ  cache index hint â€” loáº¡i container, part quan trá»ng â€” tá»‘i Æ°u seek
pub mod hwdec;            // Enumerate GPU adapter cho hardware decode
pub mod infrastructure;   // ÄÆ°á»ng dáº«n ffmpeg/mpv theo ná»n táº£ng, kiá»ƒm tra runtime mpv

// â”€â”€â”€ Public API â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU16, AtomicU64},
        Arc,
    },
};

use chrono::{DateTime, Utc};
use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::provider::{file_repository::FileRepository, stream::StreamRegistry};
use omega_drive_gateway::provider::debug_logger::DebugLogger;

// Re-exports for cross-module usage
pub use playlistbuild::{
    ensure_video_playback_ready,
};
pub use segmentgen::get_file_part_internal;
pub use segment_telemetry::SegmentTelemetry;
pub use crate::sparse::SparseCache;
pub use singleflight::PartSingleFlight;
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
}



