use async_trait::async_trait;

use crate::core::data::DownloadJob;
use crate::core::error::AppResult;

#[async_trait]
pub trait DownloadJobRepository: Send + Sync {
    async fn create_job(&self, file_id: i64, target_path: &str, total_parts: i64) -> AppResult<i64>;
    async fn update_progress(&self, id: i64, done_parts: i64) -> AppResult<()>;
    async fn update_state(&self, id: i64, state: &str, error: Option<&str>, error_code: Option<&str>) -> AppResult<()>;
    async fn get_job(&self, id: i64) -> AppResult<Option<DownloadJob>>;
    async fn list_jobs_by_state(&self, states: &[&str]) -> AppResult<Vec<DownloadJob>>;
    async fn get_next_queued(&self) -> AppResult<Option<DownloadJob>>;
    async fn exists_active_job_for_file(&self, file_id: i64) -> AppResult<bool>;
    async fn delete_job(&self, id: i64) -> AppResult<()>;
    async fn pause_all_active_jobs(&self, error_code: &str) -> AppResult<()>;
    async fn resume_shutdown_jobs(&self) -> AppResult<()>;
    async fn purge_old_jobs(&self, days: i64, states: &[&str]) -> AppResult<usize>;
}
