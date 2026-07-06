//! providers/mod.rs â€” CĂ¡c bĂªn cung cáº¥p dá»‹ch vá»¥ lÆ°u trá»¯ (Storage Providers).
//!
//! ThÆ° má»¥c nĂ y chá»©a cĂ¡c module xá»­ lĂ½ viá»‡c tÆ°Æ¡ng tĂ¡c trá»±c tiáº¿p vá»›i API cá»§a
//! cĂ¡c ná»n táº£ng phĂ­a thá»© ba nhÆ° Discord vĂ  Telegram.

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
