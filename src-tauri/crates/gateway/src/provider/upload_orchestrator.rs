use crate::upload::upload_context::UploadContext;
use crate::upload::upload_error::UploadError;
use crate::upload::upload_plan::{PreparedUploadPlan, UploadPlan, UploadSourceInfo};
use crate::upload::upload_types::{UploadRecordContext, UploadedPart};
use tokio::sync::mpsc::UnboundedSender;

use crate::provider::provider_types::RemoteUploadTarget;
use crate::core::scope::DriveScope;
use crate::upload::upload_plan::{ProviderType, UploadStrategy};

#[async_trait::async_trait]
pub trait UploadOrchestrator: Send + Sync {
    async fn build_execution_plan(
        &self,
        ctx: &UploadContext,
        source: &UploadSourceInfo,
        plan: &UploadPlan,
    ) -> Result<PreparedUploadPlan, UploadError>;

    async fn ensure_upload_target(
        &self,
        ctx: &UploadContext,
        target_path: &str,
        filename: &str,
        size: u64,
        folder_id: Option<i64>,
        drive_scope: DriveScope,
    ) -> Result<UploadRecordContext, UploadError>;

    async fn persist_part_results(
        &self,
        ctx: &UploadContext,
        file_id: i64,
        results: &[UploadedPart],
        part_type: &str,
    ) -> Result<(), UploadError>;

    async fn mark_failure(
        &self,
        ctx: &UploadContext,
        file_id: i64,
        err: &UploadError,
    ) -> Result<(), UploadError>;

    async fn mark_status(
        &self,
        ctx: &UploadContext,
        file_id: i64,
        status: &str,
    ) -> Result<(), UploadError>;

    async fn dispatch_original_part(
        &self,
        ctx: &UploadContext,
        upload_target: &RemoteUploadTarget,
        tg_authorized: bool,
        strategy: UploadStrategy,
        providers: &[ProviderType],
        file_id: i64,
        buffer: Vec<u8>,
        filename: &str,
        part_num: u32,
        total_parts: usize,
        checksum: Option<String>,
        telegram_progress_tx: Option<UnboundedSender<usize>>,
    ) -> Result<Vec<UploadedPart>, UploadError>;
}
