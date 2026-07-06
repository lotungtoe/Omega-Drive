use async_trait::async_trait;
use serde_json::Value;

use omega_drive_gateway::core::error::AppResult;

pub trait UiEventsPort: Send + Sync {}
pub trait UploadPort: Send + Sync {}
pub trait DownloadPort: Send + Sync {}
pub trait PlaybackPort: Send + Sync {}
pub trait SettingsPort: Send + Sync {}
pub trait PluginsPort: Send + Sync {}
pub trait FeatureLogsPort: Send + Sync {}

#[async_trait]
pub trait DiagnosticsPort: Send + Sync {
    async fn get_version(&self) -> AppResult<Value>;
    async fn get_connection_status(&self) -> AppResult<Value>;
    async fn get_bootstrap_status(&self) -> AppResult<Value>;
}
