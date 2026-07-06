use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DriveScope {
    #[default]
    My,
    Shared,
}

impl DriveScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::My => "my",
            Self::Shared => "shared",
        }
    }

    pub const fn remote_folder_provider_id(self) -> &'static str {
        match self {
            Self::My => "discord",
            Self::Shared => "discord_shared",
        }
    }
}

impl std::fmt::Display for DriveScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str((*self).as_str())
    }
}

impl std::str::FromStr for DriveScope {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "my" => Ok(Self::My),
            "shared" => Ok(Self::Shared),
            _ => Err("unsupported drive scope"),
        }
    }
}


