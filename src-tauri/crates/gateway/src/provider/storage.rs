use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PartMetadata {
    pub id: i64,
    pub file_id: i64,
    pub platform: String,
    pub message_id: String,
    pub attachment_name: Option<String>,
    pub part_index: u32,
    pub size: i64,
    pub part_type: String,
    pub duration: Option<f64>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderQuota {
    pub total_bytes: Option<u64>,
    pub used_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderCapability {
    ResumableUpload,
    PublicLink,
    Streaming,
    Encryption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderMetadata {
    pub id: String,
    pub display_name: String,
    pub icon: String,
    pub description: String,
}

#[async_trait]
pub trait StorageProvider: Send + Sync {
    fn metadata(&self) -> ProviderMetadata;
    fn fetch_capabilities(&self) -> Vec<ProviderCapability>;
    async fn get_quota(&self) -> Result<ProviderQuota>;

    async fn check_health(&self) -> bool {
        true
    }

    async fn upload_part(&self, data: Vec<u8>, file_id: i64, part_idx: i32)
        -> Result<PartMetadata>;

    async fn download_part(&self, part: &PartMetadata) -> Result<Vec<u8>>;
    async fn delete_part(&self, part: &PartMetadata) -> Result<()>;
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn StorageProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }

    pub fn register(&mut self, provider: Arc<dyn StorageProvider>) {
        self.providers.insert(provider.metadata().id.clone(), provider);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn StorageProvider>> {
        self.providers.get(provider_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn StorageProvider>> {
        self.providers.values().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self { Self::new() }
}


