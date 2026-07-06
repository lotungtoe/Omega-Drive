use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::provider::provider_types::{RemoteFolderRef, RemoteUploadTarget};

#[async_trait]
pub trait RemoteFolderGateway: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn create_folder(
        &self,
        name: &str,
        parent: Option<&RemoteFolderRef>,
    ) -> Result<RemoteFolderRef>;

    async fn rename_folder(&self, folder: &RemoteFolderRef, new_name: &str) -> Result<()>;

    async fn delete_folder(&self, folder: &RemoteFolderRef) -> Result<()>;

    async fn ensure_upload_target(
        &self,
        file_name: &str,
        folder: Option<&RemoteFolderRef>,
    ) -> Result<RemoteUploadTarget>;
}

pub struct RemoteFolderRegistry {
    gateways: HashMap<String, Arc<dyn RemoteFolderGateway>>,
}

impl RemoteFolderRegistry {
    pub fn new() -> Self {
        Self { gateways: HashMap::new() }
    }

    pub fn register(&mut self, gateway: Arc<dyn RemoteFolderGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn RemoteFolderGateway>> {
        self.gateways.get(provider_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn RemoteFolderGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for RemoteFolderRegistry {
    fn default() -> Self { Self::new() }
}


