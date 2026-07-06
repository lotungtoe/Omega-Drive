use std::sync::Arc;

use async_trait::async_trait;
use omega_drive_lib::{
    core::{
        error::{AppError, AppResult},
        error_codes as codes,
    },
    extensions::{
        context::ExtensionContext,
        contract::InvocationMeta,
        ports::{
            DiagnosticsPort, DownloadPort, FeatureLogsPort, PlaybackPort, PluginsPort,
            SettingsPort, UiEventsPort, UploadPort,
        },
        registry::ExtensionRegistry,
    },
};
use serde_json::{json, Value};

struct EmptyUiEvents;
impl UiEventsPort for EmptyUiEvents {}

struct EmptyUpload;
impl UploadPort for EmptyUpload {}

struct EmptyDownload;
impl DownloadPort for EmptyDownload {}

struct EmptyPlayback;
impl PlaybackPort for EmptyPlayback {}

struct EmptySettings;
impl SettingsPort for EmptySettings {}

struct EmptyPlugins;
impl PluginsPort for EmptyPlugins {}

struct EmptyFeatureLogs;
impl FeatureLogsPort for EmptyFeatureLogs {}

struct StubDiagnostics {
    version: Value,
    connection: Value,
    bootstrap: Value,
}

#[async_trait]
impl DiagnosticsPort for StubDiagnostics {
    async fn get_version(&self) -> AppResult<Value> {
        Ok(self.version.clone())
    }

    async fn get_connection_status(&self) -> AppResult<Value> {
        Ok(self.connection.clone())
    }

    async fn get_bootstrap_status(&self) -> AppResult<Value> {
        Ok(self.bootstrap.clone())
    }
}

struct TestContext {
    diagnostics: AppResult<Arc<dyn DiagnosticsPort>>,
}

impl TestContext {
    fn allowed() -> Self {
        Self {
            diagnostics: Ok(Arc::new(StubDiagnostics {
                version: json!({ "version": "0.1.0-omega", "history_len": 7 }),
                connection: json!({ "discord": { "connected": true } }),
                bootstrap: json!({ "ffmpegReady": true }),
            })),
        }
    }

    fn denied() -> Self {
        Self {
            diagnostics: Err(AppError::new(
                codes::E_PERMISSION,
                "Extension dependency 'diagnostics' is not granted",
            )),
        }
    }
}

impl ExtensionContext for TestContext {
    fn ui_events(&self) -> AppResult<Arc<dyn UiEventsPort>> {
        Ok(Arc::new(EmptyUiEvents))
    }

    fn upload(&self) -> AppResult<Arc<dyn UploadPort>> {
        Ok(Arc::new(EmptyUpload))
    }

    fn download(&self) -> AppResult<Arc<dyn DownloadPort>> {
        Ok(Arc::new(EmptyDownload))
    }

    fn playback(&self) -> AppResult<Arc<dyn PlaybackPort>> {
        Ok(Arc::new(EmptyPlayback))
    }

    fn settings(&self) -> AppResult<Arc<dyn SettingsPort>> {
        Ok(Arc::new(EmptySettings))
    }

    fn plugins(&self) -> AppResult<Arc<dyn PluginsPort>> {
        Ok(Arc::new(EmptyPlugins))
    }

    fn diagnostics(&self) -> AppResult<Arc<dyn DiagnosticsPort>> {
        self.diagnostics.clone()
    }

    fn feature_logs(&self) -> AppResult<Arc<dyn FeatureLogsPort>> {
        Ok(Arc::new(EmptyFeatureLogs))
    }
}

#[tokio::test]
async fn gateway_dispatches_snapshot_extension() {
    let registry = ExtensionRegistry::global().unwrap();
    let ctx = TestContext::allowed();

    let value = registry
        .dispatch_with_context(
            "diagnostics.snapshot",
            "read",
            json!({}),
            &ctx,
            InvocationMeta {
                window_label: Some("main".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(value["version"], "0.1.0-omega");
    assert_eq!(value["connection"]["discord"]["connected"], true);
    assert_eq!(value["bootstrap"]["ffmpegReady"], true);
}

#[tokio::test]
async fn unknown_extension_returns_not_found() {
    let registry = ExtensionRegistry::global().unwrap();
    let ctx = TestContext::allowed();

    let err = registry
        .dispatch_with_context(
            "missing.extension",
            "read",
            json!({}),
            &ctx,
            InvocationMeta::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(err.code, codes::E_NOT_FOUND);
}

#[tokio::test]
async fn unknown_command_returns_not_found() {
    let registry = ExtensionRegistry::global().unwrap();
    let ctx = TestContext::allowed();

    let err = registry
        .dispatch_with_context(
            "diagnostics.snapshot",
            "missing",
            json!({}),
            &ctx,
            InvocationMeta::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(err.code, codes::E_NOT_FOUND);
}

#[tokio::test]
async fn invalid_payload_returns_invalid_input() {
    let registry = ExtensionRegistry::global().unwrap();
    let ctx = TestContext::allowed();

    let err = registry
        .dispatch_with_context(
            "diagnostics.snapshot",
            "read",
            json!("bad"),
            &ctx,
            InvocationMeta::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(err.code, codes::E_INVALID_INPUT);
}

#[tokio::test]
async fn denied_dependency_returns_permission_error() {
    let registry = ExtensionRegistry::global().unwrap();
    let ctx = TestContext::denied();

    let err = registry
        .dispatch_with_context(
            "diagnostics.snapshot",
            "read",
            json!({}),
            &ctx,
            InvocationMeta::default(),
        )
        .await
        .unwrap_err();

    assert_eq!(err.code, codes::E_PERMISSION);
}
