use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachableDestination {
    pub id: String,
    pub name: String,
}

#[derive(Clone, serde::Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlatformProgress {
    pub name: String,
    pub done: u64,
    pub total: u64,
}

#[derive(Clone, serde::Serialize, Default)]
pub struct ProgressInfo {
    pub phase: String,
    pub done_parts: usize,
    pub total_parts: usize,
    pub detail: String,
    pub platforms: Vec<PlatformProgress>,
}

#[derive(Clone, Debug, Default)]
pub struct UiHeartbeatStatus {
    pub last_seen_epoch_secs: u64,
    pub visible: bool,
    pub focused: bool,
    pub context: String,
}

pub enum TransferType {
    Upload { file_id: Option<i64> },
    Download { path: PathBuf },
}

pub struct SenderEntry {
    pub handle: tokio::task::JoinHandle<()>,
    pub transfer_type: TransferType,
    pub cancel_token: tokio_util::sync::CancellationToken,
}

pub type SenderMap = Arc<tokio::sync::Mutex<std::collections::HashMap<String, SenderEntry>>>;

pub fn new_sender_map() -> SenderMap {
    Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()))
}


