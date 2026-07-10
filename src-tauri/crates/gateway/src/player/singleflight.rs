use async_trait::async_trait;
use bytes::Bytes;
use futures_util::future::BoxFuture;

pub type PartKey = (i64, u32, u64, u64);

#[async_trait]
pub trait PartSingleFlight: Send + Sync {
    async fn run(
        &self,
        key: PartKey,
        f: Box<dyn FnOnce() -> BoxFuture<'static, Result<Bytes, String>> + Send>,
    ) -> Result<Bytes, String>;
}
