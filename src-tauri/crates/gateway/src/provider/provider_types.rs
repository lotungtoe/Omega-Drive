use chrono::{DateTime, Utc};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, Default)]
pub struct ProviderConnectionStatus {
    pub configured: bool,
    pub connected: bool,
    pub authorized: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderUploadConstraints {
    pub max_part_bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ByteRange {
    pub start: u64,
    pub len: u64,
}

#[derive(Debug, Clone)]
pub enum MediaSource {
    ResolvedUrl {
        url: String,
        expiry: Option<DateTime<Utc>>,
    },
    ProviderOwned,
}

#[derive(Debug, Clone)]
pub struct RemoteFolderRef {
    pub provider_id: String,
    pub remote_id: String,
}

#[derive(Debug, Clone)]
pub enum RemoteUploadTarget {
    DiscordThread {
        thread_id: u64,
        archive_on_finalize: bool,
    },
}

#[derive(Debug, Clone)]
pub enum RemoteObjectRef {
    DiscordThread { thread_id: u64 },
    DiscordChannel { thread_id: u64 },
    DiscordMessage { thread_id: u64, message_id: u64 },
    TelegramMessages { message_ids: Vec<i64> },
}

#[derive(Debug)]
pub struct UploadPartRequest {
    pub target: RemoteUploadTarget,
    pub data: Vec<u8>,
    pub file_name: String,
    pub caption: String,
    pub part_num: u32,
    pub telegram_progress_tx: Option<UnboundedSender<usize>>,
}

#[derive(Debug, Clone)]
pub struct UploadPartReceipt {
    pub message_id: i64,
    pub platform: String,
    pub size: u64,
    pub attachment_name: Option<String>,
}
