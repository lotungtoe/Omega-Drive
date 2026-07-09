use async_trait::async_trait;
use bytes::Bytes;

#[async_trait]
pub trait ByteCache: Send + Sync {
    async fn write(&self, file_id: i64, offset: u64, data: Bytes);
    async fn is_range_filled(&self, file_id: i64, offset: u64, len: u64) -> bool;
    async fn wait_range(&self, file_id: i64, offset: u64, len: u64) -> Result<Bytes, String>;
    async fn set_pin_window(&self, file_id: i64, center: u64, half: u64, max: u64);
    async fn clear(&self);
}
