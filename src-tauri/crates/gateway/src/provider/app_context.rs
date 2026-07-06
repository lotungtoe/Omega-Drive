use async_trait::async_trait;
use serde_json::Value;

pub trait AppContext: Send + Sync {
    fn emit_event(&self, event: &str, payload: Value);
}

#[async_trait]
pub trait SidecarProvider: Send + Sync {
    async fn sidecar_output(&self, name: &str, args: &[&str]) -> anyhow::Result<Vec<u8>>;
}


