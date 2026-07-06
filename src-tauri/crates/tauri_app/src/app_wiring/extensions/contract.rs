use async_trait::async_trait;
use serde_json::Value;

use omega_drive_gateway::core::error::AppResult;

use super::{context::ExtensionContext, manifest::ExtensionManifest};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvocationMeta {
    pub window_label: Option<String>,
}

#[async_trait]
pub trait InternalExtension: Send + Sync {
    fn manifest(&self) -> &ExtensionManifest;

    async fn handle(
        &self,
        command_id: &str,
        payload: Value,
        ctx: &dyn ExtensionContext,
        meta: InvocationMeta,
    ) -> AppResult<Value>;
}
