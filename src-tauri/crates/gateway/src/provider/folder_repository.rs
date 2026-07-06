use async_trait::async_trait;

use crate::core::data::FolderMetadata;
use crate::core::error::AppResult;

#[async_trait]
pub trait FolderRepository: Send + Sync {
    async fn insert_folder(&self, name: &str, parent_id: Option<i64>) -> AppResult<i64>;
    async fn get_folder_by_id(&self, id: i64) -> AppResult<Option<FolderMetadata>>;
    async fn get_all_folders(&self, drive_scope: Option<&str>) -> AppResult<Vec<FolderMetadata>>;
    async fn get_folders_by_parent(&self, parent_id: Option<i64>, drive_scope: Option<&str>) -> AppResult<Vec<FolderMetadata>>;
    async fn get_folder_by_name(&self, name: &str, parent_id: Option<i64>, drive_scope: Option<&str>) -> AppResult<Option<FolderMetadata>>;
    async fn update_folder_name(&self, id: i64, name: &str) -> AppResult<()>;
    async fn update_folder_parent(&self, id: i64, parent_id: Option<i64>) -> AppResult<()>;
    async fn delete_folder(&self, id: i64) -> AppResult<()>;
    async fn toggle_folder_star(&self, id: i64, starred: bool) -> AppResult<()>;
}
