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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::PLATFORM_DISCORD;
    use crate::provider::provider_types::{RemoteUploadTarget, UploadPartReceipt, UploadPartRequest};

    struct FakeStore;
    #[async_trait]
    impl PartStoreGateway for FakeStore {
        fn provider_id(&self) -> &str { PLATFORM_DISCORD }
        async fn upload_part(&self, req: UploadPartRequest) -> Result<UploadPartReceipt> {
            Ok(UploadPartReceipt {
                message_id: req.part_num as i64,
                platform: PLATFORM_DISCORD.to_string(),
                size: req.data.len() as u64,
                attachment_name: Some(req.file_name),
            })
        }
        async fn download_part(&self, _part: &PartMetadata) -> Result<Vec<u8>> {
            anyhow::bail!("unused in test")
        }
        async fn delete_part(&self, _part: &PartMetadata) -> Result<()> {
            anyhow::bail!("unused in test")
        }
        async fn forward_part(&self, _part: &PartMetadata, _target: &str) -> Result<UploadPartReceipt> {
            anyhow::bail!("unused in test")
        }
    }

    #[tokio::test]
    async fn one_chunk_batch_roundtrips_through_registry() {
        let mut registry = PartStoreRegistry::new();
        registry.register(Arc::new(FakeStore));
        let store = registry.get(PLATFORM_DISCORD).expect("registered store must resolve");
        let receipts = store.upload_parts_batch(vec![UploadPartRequest {
            target: RemoteUploadTarget::DiscordThread { thread_id: 1, archive_on_finalize: false },
            data: vec![7u8; 1024],
            file_name: "part_0.bin".to_string(),
            caption: String::new(),
            part_num: 0,
            telegram_progress_tx: None,
        }]).await.expect("batch upload must succeed");
        assert_eq!(receipts.len(), 1);
        assert_eq!(receipts[0].size, 1024);
        assert_eq!(receipts[0].attachment_name.as_deref(), Some("part_0.bin"));
    }
}


