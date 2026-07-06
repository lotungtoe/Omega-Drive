use async_trait::async_trait;

/// Minimal attachment info needed by backup operations.
#[derive(Debug, Clone)]
pub struct BackupAttachment {
    pub message_id: u64,
    pub filename: String,
    pub url: String,
    pub size: u64,
}

/// Discord operations needed by the backup feature.
/// Concrete impl wraps discord_provider in the app layer.
#[async_trait]
pub trait DiscordBackupBackend: Send + Sync {
    async fn list_backup_messages(&self, thread_id: u64, limit: u32) -> Result<Vec<BackupAttachment>, String>;
    async fn download_backup_attachment(&self, url: &str) -> Result<Vec<u8>, String>;
    async fn create_backup_thread(&self, name: &str) -> Result<u64, String>;
    async fn upload_backup_file(&self, thread_id: u64, data: Vec<u8>, filename: &str) -> Result<(), String>;
    async fn delete_backup_thread(&self, thread_id: u64) -> Result<(), String>;
}
