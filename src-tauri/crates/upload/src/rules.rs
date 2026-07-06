use serde_json::{json, Value};

use omega_drive_gateway::upload::upload_rules::UploadProfileRule;

use crate::context::UploadContext;
use crate::error::UploadError;
use crate::UploadResult;

pub async fn get_upload_profile_rules(
    ctx: &UploadContext,
    profile_id: Option<i64>,
) -> UploadResult<Vec<UploadProfileRule>> {
    ctx.upload_profile_repo
        .get_upload_profile_rules(profile_id)
        .await
        .map_err(|e| UploadError::db("Failed to load upload rules.", e))
}

pub async fn save_upload_profile_rule(
    ctx: &UploadContext,
    rule: UploadProfileRule,
) -> UploadResult<UploadProfileRule> {
    ctx.upload_profile_repo
        .save_upload_profile_rule(&rule)
        .await
        .map_err(|e| UploadError::db("Failed to save upload rule.", e))
}

pub async fn delete_upload_profile_rule(ctx: &UploadContext, id: i64) -> UploadResult<Value> {
    ctx.upload_profile_repo
        .delete_upload_profile_rule(id)
        .await
        .map_err(|e| UploadError::db("Failed to delete upload rule.", e))?;
    Ok(json!({ "success": true }))
}

pub async fn save_upload_profile_rules_bulk(
    ctx: &UploadContext,
    profile_id: i64,
    ordered_rule_ids: Vec<i64>,
) -> UploadResult<Vec<UploadProfileRule>> {
    ctx.upload_profile_repo
        .save_upload_profile_rules_bulk(profile_id, &ordered_rule_ids)
        .await
        .map_err(|e| UploadError::db("Failed to reorder upload rules.", e))
}
