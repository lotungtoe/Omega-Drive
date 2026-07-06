use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, to_value, Value};

use crate::app_wiring::app_runtime::AppState;
use crate::app_wiring::infrastructure::diagnostics::helpers::collect_bootstrap_status;
use crate::app_wiring::infrastructure::feature_log::FEATURE_KEYS;
use omega_drive_gateway::core::error::AppResult;
use omega_drive_db::files as db_files;

use super::ports::{
    DiagnosticsPort, DownloadPort, FeatureLogsPort, PlaybackPort, PluginsPort, SettingsPort,
    UiEventsPort, UploadPort,
};

pub struct UiEventsPortAdapter;
impl UiEventsPort for UiEventsPortAdapter {}

pub struct UploadPortAdapter;
impl UploadPort for UploadPortAdapter {}

pub struct DownloadPortAdapter;
impl DownloadPort for DownloadPortAdapter {}

pub struct PlaybackPortAdapter;
impl PlaybackPort for PlaybackPortAdapter {}

pub struct SettingsPortAdapter;
impl SettingsPort for SettingsPortAdapter {}

pub struct PluginsPortAdapter;

impl PluginsPortAdapter {
    pub fn new(_state: Arc<AppState>) -> Self {
        Self
    }
}

impl PluginsPort for PluginsPortAdapter {}

pub struct FeatureLogsPortAdapter;
impl FeatureLogsPort for FeatureLogsPortAdapter {}

pub struct DiagnosticsPortAdapter {
    state: Arc<AppState>,
}

impl DiagnosticsPortAdapter {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl DiagnosticsPort for DiagnosticsPortAdapter {
    async fn get_version(&self) -> AppResult<Value> {
        let db = self.state.db_read.lock().await;
        let history_len = db_files::get_all_file_count(db.conn()).unwrap_or(0);
        Ok(json!({ "version": "0.1.0-omega", "history_len": history_len }))
    }

    async fn get_connection_status(&self) -> AppResult<Value> {
        let provider_runtime = self.state.provider_runtime();
        let discord_status = match provider_runtime.provider_admin_registry.get("discord") {
            Some(gateway) => gateway.connection_status().await.ok(),
            None => None,
        };
        let telegram_status = match provider_runtime.provider_admin_registry.get("telegram") {
            Some(gateway) => gateway.connection_status().await.ok(),
            None => None,
        };

        Ok(json!({
            "discord": {
                "connected": discord_status
                    .as_ref()
                    .map(|status| status.connected)
                    .unwrap_or(false),
            },
            "telegram": {
                "connected": telegram_status
                    .as_ref()
                    .map(|status| status.connected)
                    .unwrap_or(false),
                "authorized": telegram_status
                    .as_ref()
                    .map(|status| status.authorized)
                    .unwrap_or(false),
            }
        }))
    }

    async fn get_bootstrap_status(&self) -> AppResult<Value> {
        let snapshot = collect_bootstrap_status(self.state.as_ref()).await;
        let mut payload = to_value(snapshot)?;
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("featureKeys".into(), json!(FEATURE_KEYS));
        }
        Ok(payload)
    }
}
