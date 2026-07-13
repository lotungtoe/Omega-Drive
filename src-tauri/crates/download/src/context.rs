use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock as TokioRwLock;

use crate::parts_cache::PartsCacheInner;
use crate::partitioned_mem_cache::PartitionedMemCache;
use omega_drive_gateway::core::config::Config;
use omega_drive_gateway::core::engine_context::EngineContext;
use omega_drive_gateway::core::provider_runtime::ProviderRuntime;
use omega_drive_gateway::core::types::UiHeartbeatStatus;
use omega_drive_gateway::provider::app_context::AppContext;
use omega_drive_gateway::provider::download_job_repository::DownloadJobRepository;
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::provider::stream::StreamRegistry;

#[derive(Clone)]
pub struct DownloadContext {
    pub cfg: Arc<RwLock<Config>>,
    pub file_repo: Arc<dyn FileRepository>,
    pub download_job_repo: Arc<dyn DownloadJobRepository>,
    pub provider_runtime: Arc<ProviderRuntime>,
    pub app_ctx: Arc<dyn AppContext>,
    pub ui_heartbeats: Arc<Mutex<HashMap<String, UiHeartbeatStatus>>>,
    pub engine: EngineContext,
    pub cdn_link_cache: Arc<TokioRwLock<HashMap<String, (String, DateTime<Utc>)>>>,
    pub base_dir: PathBuf,
    pub stream_registry: Arc<StreamRegistry>,
    pub mem_cache: Arc<PartitionedMemCache>,
    pub parts_cache: Arc<Mutex<PartsCacheInner>>,
}
