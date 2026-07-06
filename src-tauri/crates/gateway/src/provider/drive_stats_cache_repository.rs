use async_trait::async_trait;

use crate::core::data::DriveStats;
use crate::core::error::AppResult;

#[async_trait]
pub trait DriveStatsCacheRepository: Send + Sync {
    async fn refresh_drive_stats_cache(&self) -> AppResult<()>;
    async fn get_drive_stats_cache(&self, drive_scope: &str) -> AppResult<Option<DriveStats>>;
}
