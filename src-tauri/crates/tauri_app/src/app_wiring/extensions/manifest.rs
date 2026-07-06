use serde::{Deserialize, Serialize};

use omega_drive_gateway::core::{
    error::{AppError, AppResult},
    error_codes as codes,
};

fn default_frontend() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionDependencyKey {
    UiEvents,
    Drive,
    Upload,
    Download,
    Playback,
    Settings,
    Plugins,
    Diagnostics,
    FeatureLogs,
}

impl ExtensionDependencyKey {
    pub const ALL: [Self; 9] = [
        Self::UiEvents,
        Self::Drive,
        Self::Upload,
        Self::Download,
        Self::Playback,
        Self::Settings,
        Self::Plugins,
        Self::Diagnostics,
        Self::FeatureLogs,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UiEvents => "ui_events",
            Self::Drive => "drive",
            Self::Upload => "upload",
            Self::Download => "download",
            Self::Playback => "playback",
            Self::Settings => "settings",
            Self::Plugins => "plugins",
            Self::Diagnostics => "diagnostics",
            Self::FeatureLogs => "feature_logs",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub version: String,
    pub description: String,
    #[serde(default = "default_frontend")]
    pub frontend: bool,
    pub commands: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<ExtensionDependencyKey>,
}

impl ExtensionManifest {
    pub fn parse_toml(raw: &str) -> AppResult<Self> {
        let manifest: Self = toml::from_str(raw).map_err(|err| {
            AppError::new(codes::E_INVALID_INPUT, "Extension manifest is invalid")
                .with_source(err.to_string())
        })?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> AppResult<()> {
        if !is_valid_extension_id(&self.id) {
            return Err(AppError::new(
                codes::E_INVALID_INPUT,
                format!(
                    "Invalid extension id '{}': expected segment.segment with lowercase ascii",
                    self.id
                ),
            ));
        }

        if self.version.trim().is_empty() {
            return Err(AppError::new(
                codes::E_INVALID_INPUT,
                format!("Extension '{}' is missing version", self.id),
            ));
        }

        if self.description.trim().is_empty() {
            return Err(AppError::new(
                codes::E_INVALID_INPUT,
                format!("Extension '{}' is missing description", self.id),
            ));
        }

        if self.commands.is_empty() {
            return Err(AppError::new(
                codes::E_INVALID_INPUT,
                format!("Extension '{}' must declare at least one command", self.id),
            ));
        }

        let mut seen = std::collections::HashSet::new();
        for command in &self.commands {
            if !is_valid_command_id(command) {
                return Err(AppError::new(
                    codes::E_INVALID_INPUT,
                    format!(
                        "Invalid command id '{}' in extension '{}'",
                        command, self.id
                    ),
                ));
            }
            if !seen.insert(command.clone()) {
                return Err(AppError::new(
                    codes::E_INVALID_INPUT,
                    format!("Duplicate command '{}' in extension '{}'", command, self.id),
                ));
            }
        }

        Ok(())
    }

    pub fn supports_command(&self, command_id: &str) -> bool {
        self.commands.iter().any(|command| command == command_id)
    }
}

pub fn is_valid_extension_id(value: &str) -> bool {
    value.contains('.')
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '.')
}

pub fn is_valid_command_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}
