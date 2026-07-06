//! providers/mod.rs — Storage Providers.
//!
//! This directory contains modules for interacting directly with third-party platform APIs such as Discord and Telegram.

pub mod config;
pub mod discord_provider;
pub mod install;
pub mod runtime;
pub mod telegram_provider;

// Re-exports tá»« external crates
pub use omega_drive_discord::discord_auth;
pub use omega_drive_discord::discord_types;
pub use omega_drive_telegram::telegram_auth;
pub use omega_drive_telegram::telegram_session;

pub(crate) fn builtin_installers() -> Vec<install::ProviderInstaller> {
    vec![
        discord_provider::build_provider_installer(),
        telegram_provider::build_provider_installer(),
    ]
}

pub fn cleanup_provider_temp_files(max_age: std::time::Duration) {
    for installer in builtin_installers() {
        installer.cleanup_temp_files(max_age);
    }
}
