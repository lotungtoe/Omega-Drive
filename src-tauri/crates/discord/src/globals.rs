use std::sync::Arc;

use anyhow::{anyhow, Context};
use omega_drive_gateway::provider::discord_backup::BackupAttachment;
use tokio::sync::OnceCell;

use crate::discord_types::{DiscordGuildId, DiscordHttp};
use crate::discord_real as discord_impl;
use crate::DiscordBackupGateway;

static DISCORD_HTTP: OnceCell<Arc<DiscordHttp>> = OnceCell::const_new();
static DISCORD_GUILD_ID: OnceCell<DiscordGuildId> = OnceCell::const_new();

pub fn set_http(http: Arc<DiscordHttp>) {
    let _ = DISCORD_HTTP.set(http);
}

pub fn set_guild_id(id: DiscordGuildId) {
    let _ = DISCORD_GUILD_ID.set(id);
}

pub fn discord_http() -> Option<Arc<DiscordHttp>> {
    DISCORD_HTTP.get().map(Arc::clone)
}

pub fn discord_guild_id() -> Option<DiscordGuildId> {
    DISCORD_GUILD_ID.get().copied()
}

pub fn discord_backup_gateway() -> Option<DiscordBackupGateway> {
    let http = DISCORD_HTTP.get()?;
    let guild_id = *DISCORD_GUILD_ID.get()?;
    Some(DiscordBackupGateway::new(Arc::clone(http), guild_id))
}

pub async fn upload_backup_file(
    thread_id: u64,
    data: Vec<u8>,
    filename: &str,
) -> anyhow::Result<()> {
    let http = DISCORD_HTTP
        .get()
        .ok_or_else(|| anyhow!("Discord HTTP not initialized"))?;
    discord_impl::send_backup_message(http, thread_id, data, filename)
        .await
        .context("Failed to send backup file to Discord")?;
    Ok(())
}

pub async fn list_backup_messages(
    thread_id: u64,
    limit: u8,
) -> anyhow::Result<Vec<BackupAttachment>> {
    let http = DISCORD_HTTP
        .get()
        .ok_or_else(|| anyhow!("Discord HTTP not initialized"))?;
    let msgs = discord_impl::fetch_backup_messages(http, thread_id, limit)
        .await
        .context("Failed to list backup messages")?;
    let mut result = Vec::new();
    for msg in &msgs {
        for att in &msg.attachments {
            result.push(BackupAttachment {
                message_id: att.message_id,
                filename: att.filename.clone(),
                url: att.url.clone(),
                size: att.size,
            });
        }
    }
    Ok(result)
}

pub async fn download_backup_attachment(url: &str) -> anyhow::Result<Vec<u8>> {
    let resp = reqwest::get(url)
        .await
        .context("Failed to download backup attachment")?;
    let bytes = resp.bytes().await.context("Failed to read attachment bytes")?;
    Ok(bytes.to_vec())
}

pub async fn create_backup_thread(thread_name: &str) -> anyhow::Result<u64> {
    let http = DISCORD_HTTP
        .get()
        .ok_or_else(|| anyhow!("Discord HTTP not initialized"))?;
    let guild_id = DISCORD_GUILD_ID
        .get()
        .ok_or_else(|| anyhow!("Discord guild ID not initialized"))?;
    let category = discord_impl::get_or_create_category(http, *guild_id, "Backup").await?;
    let channel =
        discord_impl::get_or_create_fixed_channel(http, *guild_id, "db", Some(category.id))
            .await?;

    let thread_name_safe = if thread_name.len() > 100 {
        &thread_name[..100]
    } else {
        thread_name
    };

    discord_impl::create_forum_thread(http, channel.id.get(), thread_name_safe)
        .await
}

pub async fn archive_backup_thread(thread_id: u64) -> anyhow::Result<()> {
    let http = DISCORD_HTTP
        .get()
        .ok_or_else(|| anyhow!("Discord HTTP not initialized"))?;
    discord_impl::archive_thread(http, thread_id).await
}

pub async fn delete_backup_thread(thread_id: u64) -> anyhow::Result<()> {
    let http = DISCORD_HTTP
        .get()
        .ok_or_else(|| anyhow!("Discord HTTP not initialized"))?;
    discord_impl::delete_file_thread(http, thread_id).await
}
