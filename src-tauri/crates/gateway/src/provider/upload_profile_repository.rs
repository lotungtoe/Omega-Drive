use async_trait::async_trait;

use crate::core::error::AppResult;
use crate::upload::upload_plan::UploadProfile;
use crate::upload::upload_rules::UploadProfileRule;

#[async_trait]
pub trait UploadProfileRepository: Send + Sync {
    async fn get_upload_profiles(&self) -> AppResult<Vec<UploadProfile>>;
    async fn get_profile_by_id(&self, id: i64) -> AppResult<Option<UploadProfile>>;
    async fn save_upload_profile(&self, profile: &UploadProfile) -> AppResult<UploadProfile>;
    async fn delete_upload_profile(&self, id: i64) -> AppResult<()>;
    async fn restore_default_profiles(&self) -> AppResult<Vec<UploadProfile>>;
    async fn get_upload_profile_rules(&self, profile_id: Option<i64>) -> AppResult<Vec<UploadProfileRule>>;
    async fn get_rule_by_id(&self, id: i64) -> AppResult<Option<UploadProfileRule>>;
    async fn save_upload_profile_rule(&self, rule: &UploadProfileRule) -> AppResult<UploadProfileRule>;
    async fn delete_upload_profile_rule(&self, id: i64) -> AppResult<()>;
    async fn save_upload_profile_rules_bulk(&self, profile_id: i64, ordered_rule_ids: &[i64]) -> AppResult<Vec<UploadProfileRule>>;
}
