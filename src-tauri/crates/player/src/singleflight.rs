use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use bytes::Bytes;
use futures_util::future::{BoxFuture, FutureExt, Shared};
use tokio::sync::Mutex;

pub(crate) type PartKey = (i64, u32, u64, u64);
pub type PlayerSingleFlight = SingleFlight<PartKey>;
type SharedBytesFuture = Shared<BoxFuture<'static, Result<Bytes, String>>>;

#[derive(Clone)]
pub struct SingleFlight<K> {
    inner: Arc<Mutex<HashMap<K, SharedBytesFuture>>>,
}

impl<K> Default for SingleFlight<K> {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

struct EntryGuard<K: Hash + Eq + Clone + Send + 'static> {
    key: K,
    inner: Arc<Mutex<HashMap<K, SharedBytesFuture>>>,
}

impl<K: Hash + Eq + Clone + Send + 'static> Drop for EntryGuard<K> {
    fn drop(&mut self) {
        let key = self.key.clone();
        let inner = Arc::clone(&self.inner);
        tokio::spawn(async move {
            let mut map = inner.lock().await;
            map.remove(&key);
        });
    }
}

impl<K: Hash + Eq + Clone + Send + 'static> SingleFlight<K> {
    pub(crate) async fn run<F, Fut>(&self, key: K, f: F) -> Result<Bytes, String>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<Bytes, String>> + Send + 'static,
    {
        let mut map = self.inner.lock().await;
        if let Some(shared) = map.get(&key) {
            let fut = shared.clone();
            drop(map);
            return fut.await;
        }

        let guard = EntryGuard {
            key: key.clone(),
            inner: Arc::clone(&self.inner),
        };

        let fut = async move {
            let _guard = guard;
            f().await
        }
        .boxed()
        .shared();

        map.insert(key, fut.clone());
        drop(map);
        fut.await
    }
}

use omega_drive_gateway::player::singleflight::PartSingleFlight;
use async_trait::async_trait;

#[async_trait]
impl PartSingleFlight for SingleFlight<PartKey> {
    async fn run(
        &self,
        key: PartKey,
        f: Box<dyn FnOnce() -> BoxFuture<'static, Result<Bytes, String>> + Send>,
    ) -> Result<Bytes, String> {
        SingleFlight::run(self, key, move || f()).await
    }
}


