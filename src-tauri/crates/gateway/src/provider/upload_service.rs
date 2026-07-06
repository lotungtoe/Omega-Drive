use async_trait::async_trait;

use crate::core::error::AppResult;
use crate::core::scope::DriveScope;
use crate::upload::upload_plan::UploadPlan;

/// Interface for upload operations consumed by external_import feature.
/// Concrete impl is in omega_drive_upload crate.
#[async_trait]
pub trait UploadService: Send + Sync {
    async fn start_url_import(
        &self,
        url: String,
        folder_id: Option<i64>,
        drive_scope: DriveScope,
        session_id: String,
        profile_id: Option<i64>,
        upload_plan: Option<UploadPlan>,
    ) -> AppResult<()>;
}
