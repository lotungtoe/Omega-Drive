use std::{collections::HashSet, sync::Arc};

use crate::app_wiring::app_runtime::AppState;
use omega_drive_gateway::core::error::{AppError, AppResult};
use omega_drive_gateway::core::error_codes as codes;

use super::{
    adapters::{
        DiagnosticsPortAdapter, DownloadPortAdapter, FeatureLogsPortAdapter, PlaybackPortAdapter,
        PluginsPortAdapter, SettingsPortAdapter, UiEventsPortAdapter, UploadPortAdapter,
    },
    manifest::ExtensionDependencyKey,
    ports::{
        DiagnosticsPort, DownloadPort, FeatureLogsPort, PlaybackPort, PluginsPort, SettingsPort,
        UiEventsPort, UploadPort,
    },
};

pub trait ExtensionContext: Send + Sync {
    fn ui_events(&self) -> AppResult<Arc<dyn UiEventsPort>>;
    fn upload(&self) -> AppResult<Arc<dyn UploadPort>>;
    fn download(&self) -> AppResult<Arc<dyn DownloadPort>>;
    fn playback(&self) -> AppResult<Arc<dyn PlaybackPort>>;
    fn settings(&self) -> AppResult<Arc<dyn SettingsPort>>;
    fn plugins(&self) -> AppResult<Arc<dyn PluginsPort>>;
    fn diagnostics(&self) -> AppResult<Arc<dyn DiagnosticsPort>>;
    fn feature_logs(&self) -> AppResult<Arc<dyn FeatureLogsPort>>;
}

pub struct ScopedExtensionContext {
    allowed: HashSet<ExtensionDependencyKey>,
    ui_events: Arc<dyn UiEventsPort>,
    upload: Arc<dyn UploadPort>,
    download: Arc<dyn DownloadPort>,
    playback: Arc<dyn PlaybackPort>,
    settings: Arc<dyn SettingsPort>,
    plugins: Arc<dyn PluginsPort>,
    diagnostics: Arc<dyn DiagnosticsPort>,
    feature_logs: Arc<dyn FeatureLogsPort>,
}

impl ScopedExtensionContext {
    pub fn from_state(
        state: Arc<AppState>,
        allowed: impl IntoIterator<Item = ExtensionDependencyKey>,
    ) -> Self {
        Self {
            allowed: allowed.into_iter().collect(),
            ui_events: Arc::new(UiEventsPortAdapter),
            upload: Arc::new(UploadPortAdapter),
            download: Arc::new(DownloadPortAdapter),
            playback: Arc::new(PlaybackPortAdapter),
            settings: Arc::new(SettingsPortAdapter),
            plugins: Arc::new(PluginsPortAdapter::new(state.clone())),
            diagnostics: Arc::new(DiagnosticsPortAdapter::new(state)),
            feature_logs: Arc::new(FeatureLogsPortAdapter),
        }
    }

    fn ensure_allowed(&self, dependency: ExtensionDependencyKey) -> AppResult<()> {
        if self.allowed.contains(&dependency) {
            return Ok(());
        }

        Err(AppError::new(
            codes::E_PERMISSION,
            format!(
                "Extension dependency '{}' is not granted",
                dependency.as_str()
            ),
        ))
    }
}

impl ExtensionContext for ScopedExtensionContext {
    fn ui_events(&self) -> AppResult<Arc<dyn UiEventsPort>> {
        self.ensure_allowed(ExtensionDependencyKey::UiEvents)?;
        Ok(self.ui_events.clone())
    }

    fn upload(&self) -> AppResult<Arc<dyn UploadPort>> {
        self.ensure_allowed(ExtensionDependencyKey::Upload)?;
        Ok(self.upload.clone())
    }

    fn download(&self) -> AppResult<Arc<dyn DownloadPort>> {
        self.ensure_allowed(ExtensionDependencyKey::Download)?;
        Ok(self.download.clone())
    }

    fn playback(&self) -> AppResult<Arc<dyn PlaybackPort>> {
        self.ensure_allowed(ExtensionDependencyKey::Playback)?;
        Ok(self.playback.clone())
    }

    fn settings(&self) -> AppResult<Arc<dyn SettingsPort>> {
        self.ensure_allowed(ExtensionDependencyKey::Settings)?;
        Ok(self.settings.clone())
    }

    fn plugins(&self) -> AppResult<Arc<dyn PluginsPort>> {
        self.ensure_allowed(ExtensionDependencyKey::Plugins)?;
        Ok(self.plugins.clone())
    }

    fn diagnostics(&self) -> AppResult<Arc<dyn DiagnosticsPort>> {
        self.ensure_allowed(ExtensionDependencyKey::Diagnostics)?;
        Ok(self.diagnostics.clone())
    }

    fn feature_logs(&self) -> AppResult<Arc<dyn FeatureLogsPort>> {
        self.ensure_allowed(ExtensionDependencyKey::FeatureLogs)?;
        Ok(self.feature_logs.clone())
    }
}
