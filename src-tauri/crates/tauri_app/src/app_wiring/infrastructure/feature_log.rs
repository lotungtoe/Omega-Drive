use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use tracing_subscriber::EnvFilter;

use omega_drive_gateway::core::config::Config;

pub const FEATURE_KEYS: [&str; 6] = [
    "upload",
    "download",
    "player",
    "drive",
    "settings",
    "diagnostics",
];

#[derive(Clone, Debug)]
pub struct FeatureLogState {
    pub enabled: HashMap<String, bool>,
    pub log_dir: PathBuf,
    pub frontend_enabled: bool,
}

impl FeatureLogState {
    pub fn is_enabled(&self, feature: &str) -> bool {
        self.enabled.get(feature).copied().unwrap_or(false)
    }

    pub fn is_target_enabled(&self, _target: &str) -> bool {
        true
    }
}

pub fn init_tracing(cfg: &Config, base_dir: &Path) -> FeatureLogState {
    let log_dir = base_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);

    let mut enabled = HashMap::new();
    for feature in FEATURE_KEYS {
        let config_enabled = cfg
            .logging
            .feature_enabled
            .get(feature)
            .copied()
            .unwrap_or(true);
        enabled.insert(feature.to_string(), config_enabled);
    }

    let mut log_level = cfg.log_level.clone();

    if std::env::var("DEBUG").is_ok() {
        log_level = "info,omega_drive=trace,omega_drive_lib=trace,db::sql=info,telegram=trace".to_string();
    }

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();

    FeatureLogState {
        enabled,
        log_dir,
        frontend_enabled: cfg.logging.frontend_enabled,
    }
}
