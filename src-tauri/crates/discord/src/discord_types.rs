use std::sync::Arc;

pub use serenity::http::Http as DiscordHttp;
pub use serenity::model::id::{
    ChannelId as DiscordChannelId, GuildId as DiscordGuildId, MessageId as DiscordMessageId,
};

/// Delete a Discord channel by raw u64 ID.
pub async fn delete_channel(http: &Arc<DiscordHttp>, thread_id: u64) -> Result<(), anyhow::Error> {
    DiscordChannelId::new(thread_id)
        .delete(http)
        .await
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("Failed to delete Discord channel: {e}"))
}
