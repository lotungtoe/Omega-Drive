use std::collections::HashMap;
use std::sync::Arc;
use omega_drive_gateway::provider::stream::StreamGateway;

pub struct StreamRegistry {
    gateways: HashMap<String, Arc<dyn StreamGateway>>,
}

impl StreamRegistry {
    pub fn new() -> Self { Self { gateways: HashMap::new() } }
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

#[cfg(test)]
mod tests {
    use super::*;
    use omega_drive_gateway::provider::storage::PartMetadata;
    use omega_drive_gateway::provider::provider_types::ByteRange;
    use async_trait::async_trait;
    use futures_util::StreamExt;

    struct DummyStreamGateway;

    #[async_trait]
    impl StreamGateway for DummyStreamGateway {
        fn provider_id(&self) -> &str {
            "dummy"
        }

        async fn download_part_bytes(&self, _part: &PartMetadata) -> anyhow::Result<Vec<u8>> {
            Ok(b"0123456789".to_vec())
        }
    }

    fn test_part() -> PartMetadata {
        PartMetadata {
            id: 1,
            file_id: 1,
            platform: "dummy".to_string(),
            message_id: "1".to_string(),
            attachment_name: None,
            part_index: 1,
            size: 10,
            part_type: "chunk".to_string(),
            duration: None,
            checksum: None,
        }
    }

    #[tokio::test]
    async fn default_range_stream_slices_legacy_vec_download() {
        let gateway = DummyStreamGateway;
        let mut stream = gateway
            .download_part_range_stream(&test_part(), Some(ByteRange { start: 2, len: 4 }))
            .await
            .expect("stream");

        let first = stream.next().await.expect("first item").expect("bytes");
        assert_eq!(&first[..], b"2345");
        assert!(stream.next().await.is_none());
    }
}
