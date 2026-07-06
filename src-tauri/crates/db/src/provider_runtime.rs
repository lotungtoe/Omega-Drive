use std::sync::Arc;
use crate::registries::{
    provider::ProviderRegistry,
    provider_admin::ProviderAdminRegistry,
    part_store::PartStoreRegistry,
    stream::StreamRegistry,
    remote_folder::RemoteFolderRegistry,
    remote_object::RemoteObjectRegistry,
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
