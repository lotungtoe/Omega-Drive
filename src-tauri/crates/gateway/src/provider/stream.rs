use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::stream::Stream;
use std::{
    path::{Path, PathBuf},
    pin::Pin,
};

use crate::provider::{
    provider_types::{ByteRange, MediaSource},
    storage::PartMetadata,
};

#[derive(Debug, Clone)]
pub enum StreamDownload {
    InMemory(Vec<u8>),
    OnDisk(PathBuf),
}

pub type ProviderByteStream = Pin<Box<dyn Stream<Item = Result<Bytes>> + Send>>;

#[async_trait]
pub trait StreamGateway: Send + Sync {
    fn provider_id(&self) -> &str;

    async fn download_part_bytes(&self, part: &PartMetadata) -> Result<Vec<u8>>;

    async fn download_part_range(
        &self,
        part: &PartMetadata,
        range: Option<ByteRange>,
    ) -> Result<Vec<u8>> {
        let data = self.download_part_bytes(part).await?;
        let Some(range) = range else {
            return Ok(data);
        };
        let start = range.start.min(data.len() as u64) as usize;
        let end = range.start.saturating_add(range.len).min(data.len() as u64) as usize;
        Ok(data[start..end].to_vec())
    }

    async fn download_part_range_stream(
        &self,
        part: &PartMetadata,
        range: Option<ByteRange>,
    ) -> Result<ProviderByteStream> {
        let data = self.download_part_range(part, range).await?;
        Ok(Box::pin(futures_util::stream::once(
            async move { Ok(Bytes::from(data)) },
        )))
    }

    async fn prepare_parts_for_playback(&self, _parts: &[PartMetadata]) -> Result<()> {
        Ok(())
    }

    async fn resolve_media_source(&self, _part: &PartMetadata) -> Result<MediaSource> {
        Ok(MediaSource::ProviderOwned)
    }

    async fn resolve_message_attachments(
        &self,
        _part: &PartMetadata,
    ) -> Result<Vec<(String, String)>> {
        Err(anyhow::anyhow!(
            "resolve_message_attachments not implemented by this provider"
        ))
    }

    async fn download_part_to_temp_or_bytes(
        &self,
        part: &PartMetadata,
        _threshold_bytes: usize,
        _temp_dir: &Path,
    ) -> Result<StreamDownload> {
        Ok(StreamDownload::InMemory(
            self.download_part_bytes(part).await?,
        ))
    }
}

pub struct StreamRegistry {
    gateways: HashMap<String, Arc<dyn StreamGateway>>,
}

impl StreamRegistry {
    pub fn new() -> Self {
        Self { gateways: HashMap::new() }
    }

    pub fn register(&mut self, gateway: Arc<dyn StreamGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn StreamGateway>> {
        self.gateways.get(provider_id).cloned()
    }

    pub fn list(&self) -> Vec<Arc<dyn StreamGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for StreamRegistry {
    fn default() -> Self { Self::new() }
}


