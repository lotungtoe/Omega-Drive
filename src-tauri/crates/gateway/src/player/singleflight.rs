use async_trait::async_trait;
use bytes::Bytes;
use std::future::Future;

pub type PartKey = (i64, u32);

#[async_trait]
pub trait PartSingleFlight: Send + Sync {
    async fn run<F, Fut>(&self, key: PartKey, f: F) -> Result<Bytes, String>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = Result<Bytes, String>> + Send + 'static;
}
