use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::provider::{
    provider_types::{ProviderConnectionStatus, ProviderUploadConstraints},
    storage::{ProviderCapability, ProviderMetadata, ProviderQuota},
};

#[async_trait]
pub trait ProviderAdminGateway: Send + Sync {
    fn provider_id(&self) -> &str;
    fn metadata(&self) -> ProviderMetadata;
    fn fetch_capabilities(&self) -> Vec<ProviderCapability>;
    async fn get_quota(&self) -> Result<ProviderQuota>;
    async fn connection_status(&self) -> Result<ProviderConnectionStatus>;
    async fn fetch_upload_limits(&self) -> Result<ProviderUploadConstraints>;

    async fn check_health(&self) -> bool {
        true
    }
}

pub struct ProviderAdminRegistry {
    gateways: HashMap<String, Arc<dyn ProviderAdminGateway>>,
}

impl ProviderAdminRegistry {
    pub fn new() -> Self {
        Self { gateways: HashMap::new() }
    }

    pub fn register(&mut self, gateway: Arc<dyn ProviderAdminGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn ProviderAdminGateway>> {
        self.gateways.get(provider_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn ProviderAdminGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for ProviderAdminRegistry {
    fn default() -> Self { Self::new() }
}


