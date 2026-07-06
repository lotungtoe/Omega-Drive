use std::sync::Arc;

use async_trait::async_trait;
use omega_drive_lib::{
    core::error_codes as codes,
    extensions::{
        context::ExtensionContext,
        contract::{InternalExtension, InvocationMeta},
        manifest::ExtensionManifest,
        registry::ExtensionRegistry,
    },
};
use serde_json::{json, Value};

struct FakeExtension {
    manifest: ExtensionManifest,
}

#[async_trait]
impl InternalExtension for FakeExtension {
    fn manifest(&self) -> &ExtensionManifest {
        &self.manifest
    }

    async fn handle(
        &self,
        _command_id: &str,
        _payload: Value,
        _ctx: &dyn ExtensionContext,
        _meta: InvocationMeta,
    ) -> omega_drive_lib::core::error::AppResult<Value> {
        Ok(json!({ "ok": true }))
    }
}

fn fake_extension(manifest: ExtensionManifest) -> Arc<dyn InternalExtension> {
    Arc::new(FakeExtension { manifest })
}

#[test]
fn registry_loads_generated_snapshot_extension() {
    let registry = ExtensionRegistry::global().expect("global registry should initialize");
    assert!(registry.get("diagnostics.snapshot").is_some());
}

#[test]
fn duplicate_extension_id_is_rejected() {
    let manifest = ExtensionManifest::parse_toml(
        r#"
id = "tests.example"
version = "1.0.0"
description = "Example"
commands = ["read"]
"#,
    )
    .unwrap();

    let mut registry = ExtensionRegistry::new();
    registry.register(fake_extension(manifest.clone())).unwrap();
    let err = registry.register(fake_extension(manifest)).unwrap_err();

    assert_eq!(err.code, codes::E_CONFLICT);
}

#[test]
fn invalid_dependency_key_is_rejected() {
    let err = ExtensionManifest::parse_toml(
        r#"
id = "tests.example"
version = "1.0.0"
description = "Example"
commands = ["read"]
dependencies = ["not_real"]
"#,
    )
    .unwrap_err();

    assert_eq!(err.code, codes::E_INVALID_INPUT);
}

#[test]
fn duplicate_command_in_manifest_is_rejected() {
    let err = ExtensionManifest::parse_toml(
        r#"
id = "tests.example"
version = "1.0.0"
description = "Example"
commands = ["read", "read"]
"#,
    )
    .unwrap_err();

    assert_eq!(err.code, codes::E_INVALID_INPUT);
}
