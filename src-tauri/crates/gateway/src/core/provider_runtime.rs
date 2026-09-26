use std::sync::Arc;

use crate::provider::{
    part_store::PartStoreRegistry, provider_admin::ProviderAdminRegistry,
    remote_folder::RemoteFolderRegistry, remote_object::RemoteObjectRegistry,
    storage::ProviderRegistry, stream::StreamRegistry,
};

#[derive(Clone)]
pub struct ProviderRuntime {
    pub storage_registry: Arc<ProviderRegistry>,
    pub part_store_registry: Arc<PartStoreRegistry>,
    pub stream_registry: Arc<StreamRegistry>,
    pub provider_admin_registry: Arc<ProviderAdminRegistry>,
    pub remote_folder_registry: Arc<RemoteFolderRegistry>,
    pub remote_object_registry: Arc<RemoteObjectRegistry>,
}

impl ProviderRuntime {
    pub fn with_registries(
        storage_registry: Arc<ProviderRegistry>,
        part_store_registry: Arc<PartStoreRegistry>,
        stream_registry: Arc<StreamRegistry>,
        provider_admin_registry: Arc<ProviderAdminRegistry>,
        remote_folder_registry: Arc<RemoteFolderRegistry>,
        remote_object_registry: Arc<RemoteObjectRegistry>,
    ) -> Self {
        Self { storage_registry, part_store_registry, stream_registry, provider_admin_registry, remote_folder_registry, remote_object_registry }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::platform::PLATFORM_DISCORD;
    use crate::provider::provider_types::{UploadPartReceipt, UploadPartRequest};

    struct FakeStore;
    #[async_trait::async_trait]
    impl crate::provider::part_store::PartStoreGateway for FakeStore {
        fn provider_id(&self) -> &str { PLATFORM_DISCORD }
        async fn upload_part(&self, _req: UploadPartRequest) -> anyhow::Result<UploadPartReceipt> {
            anyhow::bail!("unused in test")
        }
        async fn download_part(&self, _part: &crate::provider::storage::PartMetadata) -> anyhow::Result<Vec<u8>> {
            anyhow::bail!("unused in test")
        }
        async fn delete_part(&self, _part: &crate::provider::storage::PartMetadata) -> anyhow::Result<()> {
            anyhow::bail!("unused in test")
        }
        async fn forward_part(&self, _part: &crate::provider::storage::PartMetadata, _target: &str) -> anyhow::Result<UploadPartReceipt> {
            anyhow::bail!("unused in test")
        }
    }

    fn runtime_with_store() -> ProviderRuntime {
        let mut parts = PartStoreRegistry::new();
        parts.register(Arc::new(FakeStore));
        ProviderRuntime::with_registries(
            Arc::new(ProviderRegistry::default()),
            Arc::new(parts),
            Arc::new(StreamRegistry::default()),
            Arc::new(ProviderAdminRegistry::default()),
            Arc::new(RemoteFolderRegistry::default()),
            Arc::new(RemoteObjectRegistry::default()),
        )
    }

    #[test]
    fn runtime_swap_keeps_inflight_snapshot_serving() {
        let old = runtime_with_store();
        let inflight = old.clone();
        let _new = runtime_with_store();
        // Clones share registries: the swap only affects new holders.
        assert!(Arc::ptr_eq(&inflight.stream_registry, &old.stream_registry));
        assert!(Arc::ptr_eq(&inflight.part_store_registry, &old.part_store_registry));
        // In-flight holder still resolves the gateway after the swap.
        assert!(inflight.part_store_registry.get(PLATFORM_DISCORD).is_some());
    }
}
