use async_trait::async_trait;

use crate::core::types::ProgressInfo;

/// Interface for reporting transfer (upload/download) progress.
/// Used by both upload and download features so they don't depend on each other.
#[async_trait]
pub trait TransferProgress: Send + Sync {
    async fn report_progress(&self, transfer_id: &str, progress: ProgressInfo);
    async fn report_completion(&self, transfer_id: &str, file_id: i64);
}
