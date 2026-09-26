//! Canonical provider-platform identities.
//!
//! The DB, registries, and part metadata all key providers by these strings.
//! Compare against these constants, never inline literals, so a typo fails
//! at compile time instead of silently missing at runtime. Tests keep raw
//! literals on purpose: they pin the values below.

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

#[cfg(test)]
mod tests {
    use super::{platform_display_name, PLATFORM_DISCORD, PLATFORM_TELEGRAM};

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
}
