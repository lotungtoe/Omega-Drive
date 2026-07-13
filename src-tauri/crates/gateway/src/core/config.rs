use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_DISCORD_PARTS_PER_MESSAGE: usize = 10;

#[derive(Deserialize, Debug, Clone, Copy, Serialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum UploadMode {
    #[default]
    Safe,
    Speed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupSettings {
    pub chunk_bytes: u64,
    pub parallel_sends: usize,
    pub zip_level: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Config {
    pub general: GroupSettings,
    pub upload_mode: UploadMode,
    pub providers: HashMap<String, ProviderConfig>,
    pub http_timeout_s: u64,
    pub download_retry: u32,
    pub download_retry_base_s: u64,
    pub part_delay_ms: u64,
    pub read_buffer_bytes: usize,
    pub large_file_threshold_mb: u64,
    pub prefetch_concurrency: usize,
    pub prefetch_chunks: u32,
    pub prefetch_debounce_ms: u64,
    pub mpv_cache_secs: u64,
    pub mpv_demuxer_max_mb: u64,
    pub mpv_readahead_secs: u64,
    pub prevent_sleep_enabled: bool,
    pub bandwidth_limit_kbps: u64,
    pub adaptive_soft_limit: bool,
    pub soft_limit_ratio: f64,
    pub soft_limit_when_player_active: bool,
    pub soft_limit_when_minimized: bool,
    pub disk_check_interval_parts: u32,
    pub auto_resume_on_startup: bool,
    pub purge_days: u32,
    pub cache_preview_max_bytes: u64,
    pub cache_player_max_bytes: u64,
    pub cache_video_max_bytes: u64,
    pub cache_audio_max_bytes: u64,
    pub session_ttl_s: u64,
    pub gc_interval_s: u64,
    pub trash_ttl_days: i64,
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub keep_alive_s: u64,
    pub max_concurrency: usize,
    pub auto_sync_interval_s: u64,
    pub history_file: String,
    pub folders_file: String,
    pub sessions_file: String,
    pub auto_sync_on_startup: bool,
    pub persistent_video_bridge: bool,
    pub logging: LoggingConfig,
    pub backup_enabled: bool,
    pub backup_snapshot_interval_days: u64,
    pub d3d11_adapter: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct LoggingConfig {
    #[serde(default)]
    pub feature_enabled: HashMap<String, bool>,
    pub frontend_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProviderTransferConfig {
    pub parallel_sends: usize,
    pub chunk_mb: Option<u64>,
    pub batch_size: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProviderRetryConfig {
    pub send_retries: u32,
    pub retry_base_delay_s: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProviderLimitConfig {
    pub hard_limit_bytes: u64,
    pub file_limit_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ProviderConfig {
    pub transfer: ProviderTransferConfig,
    pub retry: ProviderRetryConfig,
    pub limits: ProviderLimitConfig,
}


