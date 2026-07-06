use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::core::engine_context::EngineContext;
use omega_drive_gateway::core::events::EventBus;
use omega_drive_gateway::provider::drive_stats_cache_repository::DriveStatsCacheRepository;
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::provider::folder_repository::FolderRepository;
use omega_drive_gateway::core::services::FileTypeClassifier;

use crate::service::DriveService;

#[derive(Clone)]
pub struct DriveQueryContext {
    pub file_repo: Arc<dyn FileRepository>,
    pub folder_repo: Arc<dyn FolderRepository>,
    pub stats_cache_repo: Arc<dyn DriveStatsCacheRepository>,
    pub cfg: Arc<RwLock<Config>>,
    pub thumbnail_dir: PathBuf,
    pub file_classifier: Arc<dyn FileTypeClassifier>,
    pub engine: EngineContext,
}

#[derive(Clone)]
pub struct DriveCommandContext {
    pub file_repo: Arc<dyn FileRepository>,
    pub folder_repo: Arc<dyn FolderRepository>,
    pub service: Arc<DriveService>,
    pub events: Arc<EventBus>,
}
