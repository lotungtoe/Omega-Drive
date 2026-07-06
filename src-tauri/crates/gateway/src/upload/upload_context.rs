use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use crate::core::config::Config;
use crate::provider::app_context::{AppContext, SidecarProvider};
use crate::provider::backup_service::BackupService;
use crate::provider::feature_log::FeatureLog;
use crate::provider::file_repository::FileRepository;
use crate::provider::transfer_progress::TransferProgress;
use crate::provider::upload_job_repository::UploadJobRepository;
use crate::provider::upload_orchestrator::UploadOrchestrator;
use crate::provider::upload_profile_repository::UploadProfileRepository;
use crate::core::services::{ExtensionNormalizer, FileTypeClassifier, MediaParser, SystemProfileProvider};
use crate::core::engine_context::EngineContext;
use crate::core::events::SharedEventBus;
use crate::core::provider_runtime::ProviderRuntime;
use crate::core::types::{UiHeartbeatStatus, SenderMap};

#[derive(Clone)]
pub struct UploadContext {
    pub cfg: Arc<std::sync::RwLock<Config>>,
    pub file_repo: Arc<dyn FileRepository>,
    pub upload_job_repo: Arc<dyn UploadJobRepository>,
    pub upload_profile_repo: Arc<dyn UploadProfileRepository>,
    pub senders: SenderMap,
    pub progress_reporter: Arc<dyn TransferProgress>,
    pub base_dir: PathBuf,
    pub thumbnail_dir: PathBuf,
    pub feature_log: Arc<dyn FeatureLog>,
    pub disk_semaphore: Arc<tokio::sync::Semaphore>,
    pub app_ctx: Arc<dyn AppContext>,
    pub sidecar: Option<Arc<dyn SidecarProvider>>,
    pub backup_service: Option<Arc<dyn BackupService>>,
    pub ui_ping_count: Arc<AtomicU64>,
    pub ui_heartbeats: Arc<Mutex<std::collections::HashMap<String, UiHeartbeatStatus>>>,
    pub file_classifier: Arc<dyn FileTypeClassifier>,
    pub ext_normalizer: Arc<dyn ExtensionNormalizer>,
    pub media_parser: Arc<dyn MediaParser>,
    pub profile_provider: Arc<dyn SystemProfileProvider>,
    pub orchestrator: Arc<dyn UploadOrchestrator>,
    pub engine: EngineContext,
    pub provider_runtime: Arc<ProviderRuntime>,
    pub events: SharedEventBus,
}
