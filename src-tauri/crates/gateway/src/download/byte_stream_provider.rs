use bytes::Bytes;
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct StreamChunk {
    pub file_id: i64,
    pub file_offset: u64,
    pub data: Bytes,
}

#[async_trait::async_trait]
pub trait ByteStreamProvider: Send + Sync {
    /// Stream bytes [offset, offset+len) từ file_id.
    /// namespace = "video" | "book" | "document" | "download"
    async fn stream_range(
        &self,
        file_id: i64,
        offset: u64,
        len: u64,
        namespace: &str,
    ) -> Result<mpsc::Receiver<Result<StreamChunk, String>>, String>;
}
