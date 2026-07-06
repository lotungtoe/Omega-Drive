pub mod discord_types;
pub mod discord_auth;
pub mod discord_real;
pub mod globals;
pub mod installer;

pub use discord_real::DiscordBackupGateway;

pub static DISCORD_CONNECTED: tokio::sync::RwLock<bool> =
    tokio::sync::RwLock::const_new(false);
