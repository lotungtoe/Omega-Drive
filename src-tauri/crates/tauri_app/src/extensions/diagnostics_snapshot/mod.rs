use std::sync::{Arc, LazyLock};

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    core::{
        error::{AppError, AppResult},
        error_codes as codes,
    },
    extensions::{
        context::ExtensionContext,
        contract::{InternalExtension, InvocationMeta},
        manifest::ExtensionManifest,
    },
};

static MANIFEST: LazyLock<ExtensionManifest> = LazyLock::new(|| {
    ExtensionManifest::parse_toml(include_str!("extension.toml"))
        .expect("diagnostics_snapshot manifest must stay valid")
});

#[derive(Default)]
struct DiagnosticsSnapshotExtension;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ReadPayload {}

pub(crate) fn build_extension() -> Arc<dyn InternalExtension> {
    Arc::new(DiagnosticsSnapshotExtension)
}

#[async_trait]
impl InternalExtension for DiagnosticsSnapshotExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &MANIFEST
    }

    async fn handle(
        &self,
        command_id: &str,
        payload: Value,
        ctx: &dyn ExtensionContext,
        _meta: InvocationMeta,
    ) -> AppResult<Value> {
        match command_id {
            "read" => self.read(payload, ctx).await,
            other => Err(AppError::new(
                codes::E_NOT_FOUND,
                format!("Unknown command '{}'", other),
            )),
        }
    }
}

impl DiagnosticsSnapshotExtension {
    async fn read(&self, payload: Value, ctx: &dyn ExtensionContext) -> AppResult<Value> {
        let normalized_payload = if payload.is_null() { json!({}) } else { payload };
        serde_json::from_value::<ReadPayload>(normalized_payload).map_err(|err| {
            AppError::new(
                codes::E_INVALID_INPUT,
                "Invalid payload for diagnostics.snapshot/read",
            )
            .with_source(err.to_string())
        })?;

        let diagnostics = ctx.diagnostics()?;
        let version_payload = diagnostics.get_version().await?;
        let version = version_payload
            .get("version")
            .cloned()
            .unwrap_or(version_payload);
        let connection = diagnostics.get_connection_status().await?;
        let bootstrap = diagnostics.get_bootstrap_status().await?;

        Ok(json!({
            "version": version,
            "connection": connection,
            "bootstrap": bootstrap,
        }))
    }
}
