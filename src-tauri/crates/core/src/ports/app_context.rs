use async_trait::async_trait;
use std::sync::Arc;

use omega_drive_gateway::provider::app_context::{AppContext, SidecarProvider};

pub struct NoopAppContext;

impl AppContext for NoopAppContext {
    fn emit_event(&self, _event: &str, _payload: serde_json::Value) {}
}

#[async_trait]
impl SidecarProvider for NoopAppContext {
    async fn sidecar_output(&self, _name: &str, _args: &[&str]) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("sidecar not available in noop context")
    }
}
