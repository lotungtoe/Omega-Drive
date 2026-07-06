use omega_drive_gateway::provider::provider_types::RemoteUploadTarget;
use omega_drive_gateway::provider::upload_orchestrator::UploadOrchestrator;
use omega_drive_gateway::core::scope::DriveScope;
use omega_drive_gateway::upload::upload_error::UploadError;
use omega_drive_gateway::upload::upload_plan::{PreparedUploadPlan, ProviderType, UploadPlan, UploadSourceInfo, UploadStrategy};
use omega_drive_gateway::upload::upload_types::{UploadRecordContext, UploadedPart};
use tokio::sync::mpsc::UnboundedSender;

use crate::context::UploadContext;
use crate::persistence;
use crate::plan;
use crate::provider_dispatch;

pub struct UploadOrchestratorImpl;

#[async_trait::async_trait]
impl UploadOrchestrator for UploadOrchestratorImpl {
    async fn build_execution_plan(
        &self,
        ctx: &UploadContext,
        source: &UploadSourceInfo,
        plan: &UploadPlan,
    ) -> Result<PreparedUploadPlan, UploadError> {
        plan::build_execution_plan(ctx, source, plan).await
    }

    async fn ensure_upload_target(
        &self,
        ctx: &UploadContext,
        target_path: &str,
        filename: &str,
        size: u64,
        folder_id: Option<i64>,
        drive_scope: DriveScope,
    ) -> Result<UploadRecordContext, UploadError> {
        persistence::ensure_upload_target(ctx, target_path, filename, size, folder_id, drive_scope, None, None).await
    }

    async fn persist_part_results(
        &self,
        ctx: &UploadContext,
        file_id: i64,
        results: &[UploadedPart],
        part_type: &str,
    ) -> Result<(), UploadError> {
        persistence::persist_part_results(ctx, file_id, results, part_type).await
    }

    async fn mark_failure(
        &self,
        ctx: &UploadContext,
        file_id: i64,
        err: &UploadError,
    ) -> Result<(), UploadError> {
        persistence::mark_failure(ctx, file_id, err).await
    }

    async fn mark_status(
        &self,
        ctx: &UploadContext,
        file_id: i64,
        status: &str,
    ) -> Result<(), UploadError> {
        persistence::mark_status(ctx, file_id, status).await
    }

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
    ) -> Result<Vec<UploadedPart>, UploadError> {
        provider_dispatch::dispatch_original_part(
            ctx, upload_target, tg_authorized, strategy, providers,
            file_id, buffer, filename, part_num, total_parts,
            checksum, telegram_progress_tx,
        ).await
    }
}
