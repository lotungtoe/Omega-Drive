use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::provider::{provider_types::RemoteObjectRef, storage::PartMetadata};

#[async_trait]
pub trait RemoteObjectGateway: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn archive_object(&self, object: &RemoteObjectRef) -> Result<()>;

    async fn delete_object(&self, object: &RemoteObjectRef) -> Result<()>;

    async fn delete_parts(&self, parts: &[PartMetadata]) -> Result<()> {
        let _ = parts;
        Ok(())
    }

    async fn delete_file_artifacts(&self, file_id: i64, parts: &[PartMetadata]) -> Result<()> {
        let _ = file_id;
        self.delete_parts(parts).await
    }

    async fn post_note(&self, object: &RemoteObjectRef, content: &str) -> Result<()> {
        let _ = (object, content);
        Ok(())
    }

    async fn object_exists(&self, object: &RemoteObjectRef) -> Result<bool>;
}

pub struct RemoteObjectRegistry {
    gateways: HashMap<String, Arc<dyn RemoteObjectGateway>>,
}

impl RemoteObjectRegistry {
    pub fn new() -> Self {
        Self { gateways: HashMap::new() }
    }

    pub fn register(&mut self, gateway: Arc<dyn RemoteObjectGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn RemoteObjectGateway>> {
        self.gateways.get(provider_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn RemoteObjectGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for RemoteObjectRegistry {
    fn default() -> Self { Self::new() }
}


