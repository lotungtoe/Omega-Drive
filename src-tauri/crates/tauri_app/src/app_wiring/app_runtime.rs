use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock as StdRwLock;

use tokio::sync::RwLock;

use omega_drive_gateway::core::engine_context::EngineContext;
use omega_drive_gateway::provider::app_context::AppContext;
use omega_drive_core::ports::app_context::NoopAppContext;
use omega_drive_gateway::provider::app_context::SidecarProvider;
use omega_drive_core::provider_runtime::ProviderRuntime;
use omega_drive_player::{PlayerContext};
use omega_drive_player::runtime::PlayerRuntime;

use crate::db::repos::{
    DbDriveStatsCacheRepository, DbUploadProfileRepository,
    ProgressMapTransferProgress, StateFeatureLog,
};
use super::infrastructure::feature_log::FeatureLogState;
use crate::features::backup::BackupService;
use crate::features::download::DownloadContext;
use crate::features::drive::{DriveCommandContext, DriveQueryContext, DriveService};
use omega_drive_gateway::core::types::SenderMap;
use omega_drive_gateway::upload::upload_context::UploadContext;
use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::core::events::EventBus;
use omega_drive_core::services::{
    DefaultExtensionNormalizer, DefaultFileTypeClassifier,
    DefaultMediaParser, DefaultSystemProfileProvider,
};
use omega_drive_gateway::core::tenant::TenantDescriptor;
use omega_drive_download as download_crate;

pub use omega_drive_gateway::core::types::PlatformProgress;
pub use omega_drive_gateway::core::types::ProgressInfo;
pub use omega_drive_gateway::core::types::UiHeartbeatStatus;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<StdRwLock<Config>>,
    pub db_read: Arc<omega_drive_db::ReadDbPool>,
    pub db_write: Arc<omega_drive_db::DbWriteQueue>,
    pub drive_db_read: Arc<omega_drive_db::ReadDbPool>,
    pub provider_runtime: Arc<std::sync::RwLock<Arc<ProviderRuntime>>>,
    pub senders: SenderMap,
    pub progress_map: Arc<RwLock<HashMap<String, ProgressInfo>>>,
    pub base_dir: PathBuf,
    pub thumbnail_dir: PathBuf,
    pub feature_logs: Arc<FeatureLogState>,
    pub cdn_link_cache: Arc<RwLock<HashMap<String, (String, chrono::DateTime<chrono::Utc>)>>>,
    pub events: Arc<EventBus>,
    pub drive_service: Arc<DriveService>,
    pub bridge_port: u16,
    pub book_bridge_port: u16,
    pub active_tenant: Arc<std::sync::Mutex<TenantDescriptor>>,
    pub player_runtime: Arc<PlayerRuntime>,
    pub download_manager: Arc<download_crate::DownloadManager>,
    pub disk_semaphore: Arc<tokio::sync::Semaphore>,
    pub stream_spool_sem: Arc<tokio::sync::Semaphore>,
    pub stream_spool_bytes: Arc<AtomicU64>,
    pub stream_spool_limit_bytes: u64,
    pub ui_last_heartbeat: Arc<std::sync::atomic::AtomicU64>,
    pub app_ctx: Arc<Mutex<Option<Arc<dyn AppContext>>>>,
    pub sidecar: Arc<Mutex<Option<Arc<dyn SidecarProvider>>>>,
    pub ui_ping_count: Arc<std::sync::atomic::AtomicU64>,
    pub ui_heartbeats: Arc<std::sync::Mutex<HashMap<String, UiHeartbeatStatus>>>,
    pub engine: EngineContext,
    pub backup_service: Option<Arc<BackupService>>,
    pub player_ctx: Arc<PlayerContext>,
    pub file_repo: Arc<dyn omega_drive_gateway::provider::file_repository::FileRepository>,
    pub folder_repo: Arc<dyn omega_drive_gateway::provider::folder_repository::FolderRepository>,
    pub upload_job_repo: Arc<dyn omega_drive_gateway::provider::upload_job_repository::UploadJobRepository>,
    pub download_job_repo: Arc<dyn omega_drive_gateway::provider::download_job_repository::DownloadJobRepository>,
}

impl AppState {
    pub fn provider_runtime(&self) -> Arc<ProviderRuntime> {
        match self.provider_runtime.read() {
            Ok(guard) => Arc::clone(&guard),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    pub fn replace_provider_runtime(&self, runtime: Arc<ProviderRuntime>) {
        match self.provider_runtime.write() {
            Ok(mut guard) => *guard = runtime,
            Err(poisoned) => {
                let mut guard = poisoned.into_inner();
                *guard = runtime;
            }
        }
    }

    pub fn drive_query_context(&self) -> DriveQueryContext {
        DriveQueryContext {
            file_repo: Arc::clone(&self.file_repo),
            folder_repo: Arc::clone(&self.folder_repo),
            stats_cache_repo: Arc::new(DbDriveStatsCacheRepository::new(Arc::clone(&self.db_write))),
            cfg: Arc::clone(&self.cfg),
            thumbnail_dir: self.thumbnail_dir.clone(),
            file_classifier: Arc::new(DefaultFileTypeClassifier),
            engine: self.engine.clone(),
        }
    }

    pub fn drive_command_context(&self) -> DriveCommandContext {
        DriveCommandContext {
            file_repo: Arc::clone(&self.file_repo),
            folder_repo: Arc::clone(&self.folder_repo),
            service: Arc::clone(&self.drive_service),
            events: Arc::clone(&self.events),
        }
    }

    pub fn upload_context(&self) -> UploadContext {
        UploadContext {
            cfg: Arc::clone(&self.cfg),
            file_repo: Arc::clone(&self.file_repo),
            upload_job_repo: Arc::clone(&self.upload_job_repo),
            upload_profile_repo: Arc::new(DbUploadProfileRepository::new(Arc::clone(&self.db_write))),
            provider_runtime: self.provider_runtime(),
            senders: Arc::clone(&self.senders),
            progress_reporter: Arc::new(ProgressMapTransferProgress::new(Arc::clone(&self.progress_map))),
            base_dir: self.base_dir.clone(),
            thumbnail_dir: self.thumbnail_dir.clone(),
            feature_log: Arc::new(StateFeatureLog),
            disk_semaphore: Arc::clone(&self.disk_semaphore),
            app_ctx: self.app_ctx_emit().unwrap_or_else(|| Arc::new(NoopAppContext)),
            sidecar: self.sidecar.lock().ok().and_then(|g| g.clone()),
            ui_ping_count: Arc::clone(&self.ui_ping_count),
            ui_heartbeats: Arc::clone(&self.ui_heartbeats),
            events: Arc::clone(&self.events),
            backup_service: self.backup_service.as_ref().map(|s| Arc::clone(s) as Arc<dyn omega_drive_gateway::provider::backup_service::BackupService>),
            file_classifier: Arc::new(DefaultFileTypeClassifier),
            ext_normalizer: Arc::new(DefaultExtensionNormalizer),
            media_parser: Arc::new(DefaultMediaParser),
            profile_provider: Arc::new(DefaultSystemProfileProvider),
            orchestrator: Arc::new(omega_drive_upload::UploadOrchestratorImpl),
            engine: self.engine.clone(),
        }
    }

    pub fn download_context(&self) -> DownloadContext {
        DownloadContext {
            cfg: Arc::clone(&self.cfg),
            file_repo: Arc::clone(&self.file_repo),
            download_job_repo: Arc::clone(&self.download_job_repo),
            provider_runtime: self.provider_runtime(),
            app_ctx: self.app_ctx_emit().unwrap_or_else(|| Arc::new(NoopAppContext)),
            ui_heartbeats: Arc::clone(&self.ui_heartbeats),
            engine: self.engine.clone(),
        }
    }

    pub fn app_ctx_emit(&self) -> Option<Arc<dyn AppContext>> {
        self.app_ctx.lock().ok().and_then(|g| g.clone())
    }
}
