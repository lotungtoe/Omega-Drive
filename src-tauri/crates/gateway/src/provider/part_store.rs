use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::provider::{
    provider_types::{UploadPartReceipt, UploadPartRequest},
    storage::PartMetadata,
};

#[async_trait]
pub trait PartStoreGateway: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn upload_part(&self, request: UploadPartRequest) -> Result<UploadPartReceipt>;

    async fn upload_parts_batch(
        &self,
        requests: Vec<UploadPartRequest>,
    ) -> Result<Vec<UploadPartReceipt>> {
        // ponytail: sequential loop, upgrade to concurrent with retry if throughput matters
        let mut receipts = Vec::with_capacity(requests.len());
        for req in requests {
            receipts.push(self.upload_part(req).await?);
        }
        Ok(receipts)
    }

    async fn download_part(&self, part: &PartMetadata) -> Result<Vec<u8>>;
    async fn delete_part(&self, part: &PartMetadata) -> Result<()>;
    async fn forward_part(
        &self,
        part: &PartMetadata,
        target_container_id: &str,
    ) -> Result<UploadPartReceipt>;
}

pub struct PartStoreRegistry {
    gateways: HashMap<String, Arc<dyn PartStoreGateway>>,
}

impl PartStoreRegistry {
    pub fn new() -> Self {
        Self { gateways: HashMap::new() }
    }

    pub fn register(&mut self, gateway: Arc<dyn PartStoreGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn PartStoreGateway>> {
        self.gateways.get(provider_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn PartStoreGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for PartStoreRegistry {
    fn default() -> Self { Self::new() }
}


