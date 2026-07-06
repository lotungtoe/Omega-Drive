use serde_json::{json, Value};

use omega_drive_gateway::upload::upload_plan::UploadProfile;

use crate::context::UploadContext;
use crate::error::UploadError;
use crate::UploadResult;

pub async fn get_upload_profiles(ctx: &UploadContext) -> UploadResult<Vec<UploadProfile>> {
    ctx.upload_profile_repo
        .get_upload_profiles()
        .await
        .map_err(|e| UploadError::db("Failed to load upload profiles.", e))
}

pub async fn save_upload_profile(
    ctx: &UploadContext,
    profile: UploadProfile,
) -> UploadResult<UploadProfile> {
    ctx.upload_profile_repo
        .save_upload_profile(&profile)
        .await
        .map_err(|e| UploadError::db("Failed to save upload profile.", e))
}

pub async fn delete_upload_profile(ctx: &UploadContext, id: i64) -> UploadResult<Value> {
    ctx.upload_profile_repo
        .delete_upload_profile(id)
        .await
        .map_err(|e| UploadError::db("Failed to delete upload profile.", e))?;
    Ok(json!({ "success": true }))
}

pub async fn restore_default_profiles(ctx: &UploadContext) -> UploadResult<Vec<UploadProfile>> {
    ctx.upload_profile_repo
        .restore_default_profiles()
        .await
        .map_err(|e| UploadError::db("Failed to restore default profiles.", e))
}
