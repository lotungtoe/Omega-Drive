//! Canonical provider-platform identities.
//!
//! The DB, registries, and part metadata all key providers by these strings.
//! Compare against these constants, never inline literals, so a typo fails
//! at compile time instead of silently missing at runtime. Tests keep raw
//! literals on purpose: they pin the values below.
//!
//! [`Platform`] is the typed form: convert infallibly from [`ProviderType`]
//! (closed enum) or fallibly from stored strings via [`FromStr`]. Branch on
//! the enum with `match`, never on strings, so a new provider is a compile
//! error at every branch instead of a silent runtime miss.

use std::fmt;
use std::str::FromStr;

use crate::upload::upload_plan::ProviderType;

pub const PLATFORM_DISCORD: &str = "discord";
pub const PLATFORM_TELEGRAM: &str = "telegram";

/// UI display name for a platform id. Unknown ids pass through unchanged.
pub fn platform_display_name(id: &str) -> &str {
    match id {
        PLATFORM_DISCORD => "Discord",
        PLATFORM_TELEGRAM => "Telegram",
        other => other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Discord,
    Telegram,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Discord => PLATFORM_DISCORD,
            Platform::Telegram => PLATFORM_TELEGRAM,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<ProviderType> for Platform {
    fn from(provider: ProviderType) -> Self {
        match provider {
            ProviderType::Discord => Platform::Discord,
            ProviderType::Telegram => Platform::Telegram,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownPlatform;

impl fmt::Display for UnknownPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown platform")
    }
}

impl std::error::Error for UnknownPlatform {}

impl FromStr for Platform {
    type Err = UnknownPlatform;

    fn from_str(id: &str) -> Result<Self, UnknownPlatform> {
        match id {
            PLATFORM_DISCORD => Ok(Platform::Discord),
            PLATFORM_TELEGRAM => Ok(Platform::Telegram),
            _ => Err(UnknownPlatform),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{platform_display_name, PLATFORM_DISCORD, PLATFORM_TELEGRAM};
    use super::{Platform, UnknownPlatform};
    use crate::upload::upload_plan::ProviderType;

    #[test]
    fn ids_match_stored_values() {
        assert_eq!(PLATFORM_DISCORD, "discord");
        assert_eq!(PLATFORM_TELEGRAM, "telegram");
    }

    #[test]
    fn display_names_pass_through_unknown() {
        assert_eq!(platform_display_name("discord"), "Discord");
        assert_eq!(platform_display_name("telegram"), "Telegram");
        assert_eq!(platform_display_name("other"), "other");
    }

    #[test]
    fn provider_type_converts_infallibly() {
        assert_eq!(Platform::from(ProviderType::Discord), Platform::Discord);
        assert_eq!(Platform::from(ProviderType::Telegram), Platform::Telegram);
    }

    #[test]
    fn parse_accepts_known_rejects_unknown() {
        assert_eq!("discord".parse::<Platform>(), Ok(Platform::Discord));
        assert_eq!("telegram".parse::<Platform>(), Ok(Platform::Telegram));
        assert_eq!("mydrive".parse::<Platform>(), Err(UnknownPlatform));
    }

    #[test]
    fn display_roundtrips_through_parse() {
        for platform in [Platform::Discord, Platform::Telegram] {
            assert_eq!(platform.to_string().parse::<Platform>(), Ok(platform));
        }
    }
}
