use async_trait::async_trait;

use crate::core::data::UploadJob;
use crate::core::error::AppResult;

#[async_trait]
pub trait UploadJobRepository: Send + Sync {
    async fn upsert_job(&self, file_id: i64, source_path: &str, state: &str, total_parts: i64) -> AppResult<i64>;
    async fn get_active_job_by_source_path(&self, source_path: &str) -> AppResult<Option<UploadJob>>;
    async fn get_job_by_file_id(&self, file_id: i64) -> AppResult<Option<UploadJob>>;
    async fn update_source_path(&self, file_id: i64, source_path: &str) -> AppResult<()>;
    async fn update_progress(&self, file_id: i64, done_parts: i64, total_parts: i64) -> AppResult<()>;
    async fn update_state(&self, file_id: i64, state: &str, error: Option<&str>, error_code: Option<&str>) -> AppResult<()>;
    async fn delete_job_by_file_id(&self, file_id: i64) -> AppResult<()>;
}
