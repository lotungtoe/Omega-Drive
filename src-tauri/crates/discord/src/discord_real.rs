use anyhow::{anyhow, Context as AnyhowContext, Result};
use serenity::{
    async_trait,
    http::Http,
    model::{
        channel::GuildChannel,
        gateway::Ready,
        id::{ChannelId, GuildId, MessageId},
    },
    prelude::*,
};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use omega_drive_gateway::provider::discord_backup::{BackupAttachment as BackupBackendAttachment, DiscordBackupBackend};

use omega_drive_gateway::provider::storage::{
    ProviderCapability, ProviderMetadata, ProviderQuota, StorageProvider,
};
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::provider::storage::PartMetadata;

// ─── Handler ────────────────────────────────────────────────────────────────────────

pub struct Handler {
    pub guild_id: GuildId,
    pub file_repo: Arc<dyn FileRepository>,
    pub ready_tx: Mutex<Option<mpsc::Sender<()>>>,
    pub event_bus: Arc<omega_drive_gateway::core::events::EventBus>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: serenity::prelude::Context, ready: Ready) {
        info!("Bot Discord đã online: {}", ready.user.name);
        *crate::DISCORD_CONNECTED
            .write()
            .await = true;
        self.event_bus
            .emit(omega_drive_gateway::core::events::OmegaEvent::DiscordConnectionStatusChanged(true));

        if let Some(tx) = self.ready_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }
    }

    async fn resume(
        &self,
        _ctx: serenity::prelude::Context,
        _: serenity::model::event::ResumedEvent,
    ) {
        info!("Discord bot đã kết nối lại (resumed).");
        *crate::DISCORD_CONNECTED
            .write()
            .await = true;
        self.event_bus
            .emit(omega_drive_gateway::core::events::OmegaEvent::DiscordConnectionStatusChanged(true));
    }

    async fn shard_stage_update(
        &self,
        _ctx: serenity::prelude::Context,
        event: serenity::all::ShardStageUpdateEvent,
    ) {
        use serenity::gateway::ConnectionStage;
        match event.new {
            ConnectionStage::Connected => {
                info!("Discord shard {} đã kết nối.", event.shard_id);
                *crate::DISCORD_CONNECTED
                    .write()
                    .await = true;
                self.event_bus
                    .emit(omega_drive_gateway::core::events::OmegaEvent::DiscordConnectionStatusChanged(true));
            }
            ConnectionStage::Disconnected => {
                info!("Discord shard {} mất kết nối.", event.shard_id);
                *crate::DISCORD_CONNECTED
                    .write()
                    .await = false;
                self.event_bus
                    .emit(omega_drive_gateway::core::events::OmegaEvent::DiscordConnectionStatusChanged(false));
            }
            _ => {}
        }
    }

    async fn channel_delete(
        &self,
        _ctx: serenity::prelude::Context,
        channel: GuildChannel,
        _messages: Option<Vec<serenity::model::channel::Message>>,
    ) {
        let ch_id_str = channel.id.get().to_string();
        if let Err(e) = self.file_repo.set_files_error_by_thread_id(&ch_id_str).await {
            error!("Lỗi cập nhật DB sau khi xóa kênh Discord: {e}");
        } else {
            info!("🗑️ Kênh #{} đã bị xóa trên Discord -> Đã đồng bộ trạng thái vào DB.", channel.name);
        }
    }

    async fn channel_update(
        &self,
        _ctx: serenity::prelude::Context,
        old: Option<GuildChannel>,
        new: GuildChannel,
    ) {
        if new.guild_id != self.guild_id {
            return;
        }
        if new.kind != serenity::model::channel::ChannelType::Text {
            return;
        }

        let old_parent = old.as_ref().and_then(|c| c.parent_id);
        let name_changed = old.as_ref().map(|c| c.name != new.name).unwrap_or(true);
        let new_parent = new.parent_id;

        if old_parent == new_parent && !name_changed {
            return;
        }

        let ch_id_str = new.id.get().to_string();
        let new_channel_name = new.name.clone();

        if name_changed {
            let _ = self.file_repo.rename_file_by_thread_id(&ch_id_str, &new_channel_name).await;
        }
    }
}

// ─── DiscordStorageProvider ────────────────────────────────────────────────────────

pub struct DiscordStorageProvider {
    pub http: Arc<Http>,
    pub _guild_id: GuildId,
    pub file_repo: Arc<dyn FileRepository>,
}

#[async_trait]
impl StorageProvider for DiscordStorageProvider {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: "discord".to_string(),
            display_name: "Discord Cloud".to_string(),
            icon: "mdi-discord".to_string(),
            description: "Lưu trữ qua nền tảng Discord (Sử dụng các kênh văn bản).".to_string(),
        }
    }

    fn fetch_capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::ResumableUpload,
            ProviderCapability::Streaming,
        ]
    }

    async fn get_quota(&self) -> Result<ProviderQuota> {
        let used = self.file_repo.get_platform_usage("discord").await.unwrap_or(0);
        Ok(ProviderQuota {
            total_bytes: None,
            used_bytes: used,
        })
    }

    async fn upload_part(
        &self,
        data: Vec<u8>,
        file_id: i64,
        part_idx: i32,
    ) -> Result<PartMetadata> {
        let size = data.len() as i64;
        let (thread_id, filename) = {
            let file = self.file_repo.get_file_by_id(file_id).await?
                .ok_or_else(|| anyhow!("Không tìm thấy file ID: {}", file_id))?;
            let thread_id = file
                .thread_id
                .parse::<u64>()
                .map_err(|_| anyhow!("ID thread không hợp lệ: {}", file.thread_id))?;
            (ChannelId::new(thread_id), file.filename)
        };

        let caption = format!("**{}** • Phần {}", filename, part_idx);
        let zip_name = format!("{}.part{:04}.zip", filename, part_idx);
        let attachment_name = zip_name.clone();

        let (msg_id, _) = send_part(&self.http, thread_id, data, zip_name, caption).await?;

        Ok(PartMetadata {
            id: 0,
            file_id,
            platform: self.metadata().id,
            message_id: msg_id.to_string(),
            attachment_name: Some(attachment_name),
            part_index: part_idx as u32,
            size,
            part_type: "chunk".to_string(),
            duration: None,
            checksum: None,
        })
    }

    async fn download_part(&self, part: &PartMetadata) -> Result<Vec<u8>> {
        let thread_id: u64 = {
            let file =
                self.file_repo.get_file_by_id(part.file_id).await?.ok_or_else(|| {
                    anyhow!(
                        "Không tìm thấy file trong DB cho mảnh này: {}",
                        part.file_id
                    )
                })?;
            file.thread_id.parse()?
        };
        let msg_id: u64 = part.message_id.parse()?;

        let url = fetch_attachment_url(
            &self.http,
            thread_id,
            msg_id,
            part.part_index,
        )
        .await?;

        tracing::debug!("[dl] discord fetch_url: thread={} msg={} idx={} url_len={}",
            thread_id, msg_id, part.part_index, url.len());
        let response = reqwest::get(&url).await?;
        let status = response.status();
        let bytes = response.bytes().await?.to_vec();
        tracing::debug!("[dl] discord download: idx={} status={} bytes={}", part.part_index, status, bytes.len());
        if !status.is_success() {
            return Err(anyhow!("Lỗi tải từ Discord: {}", status));
        }
        Ok(bytes)
    }

    async fn delete_part(&self, part: &PartMetadata) -> Result<()> {
        let thread_id: u64 = {
            let file =
                self.file_repo.get_file_by_id(part.file_id).await?.ok_or_else(|| {
                    anyhow!("Không tìm thấy file trong DB để xóa mảnh: {}", part.file_id)
                })?;
            file.thread_id.parse()?
        };
        let msg_id: u64 = part.message_id.parse()?;

        self.http
            .delete_message(
                ChannelId::new(thread_id),
                MessageId::new(msg_id),
                None,
            )
            .await?;
    Ok(())
}
}

// ─── Helpers for main crate (serenity-free boundary) ──────────────────────

pub async fn try_start_gateway(
    token: &str,
    handler: Handler,
) -> Result<(Arc<Http>, Option<serenity::Client>)> {
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;
    match serenity::Client::builder(token, intents)
        .event_handler(handler)
        .await
    {
        Ok(client) => {
            let http = Arc::clone(&client.http);
            Ok((http, Some(client)))
        }
        Err(_) => Err(anyhow!("Failed to create Discord client")),
    }
}

pub async fn fetch_guild_upload_limit(
    http: &Arc<Http>,
    guild_id: GuildId,
) -> Result<Option<i64>> {
    use serenity::model::guild::PremiumTier;
    match guild_id.to_partial_guild(http).await {
        Ok(guild) => match guild.premium_tier {
            PremiumTier::Tier2 => Ok(Some(50 * 1024 * 1024)),
            PremiumTier::Tier3 => Ok(Some(100 * 1024 * 1024)),
            _ => Ok(Some(25 * 1024 * 1024)),
        },
        Err(err) => {
            tracing::warn!("Unable to read Discord tier, using fallback upload limit: {err}");
            Ok(Some(25 * 1024 * 1024))
        }
    }
}

pub async fn channel_exists(http: &Arc<Http>, thread_id: u64) -> Result<bool> {
    match http.get_channel(thread_id.into()).await {
        Ok(channel) => Ok(channel.guild().is_some()),
        Err(serenity::Error::Http(http_err))
            if http_err.status_code() == Some(reqwest::StatusCode::NOT_FOUND) =>
        {
            Ok(false)
        }
        Err(err) => Err(anyhow!(err)),
    }
}

pub async fn message_exists(
    http: &Arc<Http>,
    thread_id: u64,
    message_id: u64,
) -> Result<bool> {
    match http
        .get_message(ChannelId::new(thread_id), MessageId::new(message_id))
        .await
    {
        Ok(_) => Ok(true),
        Err(serenity::Error::Http(http_err))
            if http_err.status_code() == Some(reqwest::StatusCode::NOT_FOUND) =>
        {
            Ok(false)
        }
        Err(err) => Err(anyhow!(err)),
    }
}

pub async fn send_backup_message(
    http: &Arc<Http>,
    thread_id: u64,
    data: Vec<u8>,
    filename: &str,
) -> Result<()> {
    let attachment = serenity::builder::CreateAttachment::bytes(data, filename);
    let builder = serenity::builder::CreateMessage::new()
        .content("Backup")
        .add_file(attachment);
    ChannelId::new(thread_id)
        .send_message(http, builder)
        .await?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct BackupMessage {
    pub message_id: u64,
    pub attachments: Vec<BackupAttachment>,
}

#[derive(Debug, Clone)]
pub struct BackupAttachment {
    pub message_id: u64,
    pub filename: String,
    pub url: String,
    pub size: u64,
}

pub async fn fetch_backup_messages(
    http: &Arc<Http>,
    thread_id: u64,
    limit: u8,
) -> Result<Vec<BackupMessage>> {
    let builder = serenity::all::GetMessages::new().limit(limit);
    let msgs = ChannelId::new(thread_id)
        .messages(http, builder)
        .await?;
    let mut result = Vec::new();
    for msg in msgs {
        let attachments = msg
            .attachments
            .iter()
            .map(|att| BackupAttachment {
                message_id: msg.id.get(),
                filename: att.filename.clone(),
                url: att.url.clone(),
                size: att.size as u64,
            })
            .collect();
        result.push(BackupMessage {
            message_id: msg.id.get(),
            attachments,
        });
    }
    Ok(result)
}

pub async fn delete_discord_message(
    http: &Arc<Http>,
    thread_id: u64,
    message_id: u64,
) -> Result<()> {
    ChannelId::new(thread_id)
        .delete_message(http, MessageId::new(message_id))
        .await?;
    Ok(())
}

pub async fn create_forum_thread(
    http: &Arc<Http>,
    thread_id: u64,
    thread_name: &str,
) -> Result<u64> {
    let name = if thread_name.len() > 100 {
        &thread_name[..100]
    } else {
        thread_name
    };
    let builder = serenity::builder::CreateThread::new(name);
    let thread = ChannelId::new(thread_id)
        .create_thread(http, builder)
        .await?;
    Ok(thread.id.get())
}

// ─── Các hàm tiện ích tương tác với Discord API ──────────────────────────

pub fn sanitize_name(name: &str) -> String {
    use std::path::Path;
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    let lower = stem.to_lowercase();
    let filtered: String = lower
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == ' ')
        .collect();
    let dashed = filtered.trim().replace(' ', "-");
    let mut result = String::with_capacity(dashed.len());
    let mut last_dash = false;
    for ch in dashed.chars() {
        if ch == '-' {
            if !last_dash {
                result.push('-');
            }
            last_dash = true;
        } else {
            result.push(ch);
            last_dash = false;
        }
    }
    let trimmed = result.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "file".to_string()
    } else {
        trimmed.chars().take(100).collect()
    }
}

pub async fn get_or_create_category(
    http: &Arc<Http>,
    guild_id: GuildId,
    name: &str,
) -> Result<GuildChannel> {
    let safe = sanitize_name(name);
    let guild = guild_id
        .to_partial_guild(http)
        .await
        .context("Không thể lấy thông tin Server")?;
    let channels = guild
        .channels(http)
        .await
        .context("Không thể lấy danh sách kênh")?;

    for ch in channels.values() {
        if ch.kind == serenity::model::channel::ChannelType::Category
            && ch.name.to_lowercase() == safe
        {
            return Ok(ch.clone());
        }
    }

    let cat = guild
        .create_channel(
            http,
            serenity::builder::CreateChannel::new(&safe)
                .kind(serenity::model::channel::ChannelType::Category),
        )
        .await
        .context("Lỗi tạo Category Discord")?;
    info!("📌 Đã tạo Category mới trên Discord: {safe}");
    Ok(cat)
}

pub async fn get_or_create_fixed_channel(
    http: &Arc<Http>,
    guild_id: GuildId,
    name: &str,
    category_id: Option<ChannelId>,
) -> Result<GuildChannel> {
    let safe = sanitize_name(name);
    let guild = guild_id
        .to_partial_guild(http)
        .await
        .context("Không thể lấy thông tin Server")?;
    let channels = guild
        .channels(http)
        .await
        .context("Không thể lấy danh sách kênh")?;

    for ch in channels.values() {
        if ch.kind == serenity::model::channel::ChannelType::Text
            && ch.name.to_lowercase() == safe
            && ch.parent_id == category_id
        {
            return Ok(ch.clone());
        }
    }

    let mut builder = serenity::builder::CreateChannel::new(&safe)
        .kind(serenity::model::channel::ChannelType::Text);
    if let Some(cat_id) = category_id {
        builder = builder.category(cat_id);
    }
    let ch = guild
        .create_channel(http, builder)
        .await
        .context("Lỗi tạo kênh cố định")?;
    info!("🔗 Đã tạo kênh cố định: {safe}");
    Ok(ch)
}

pub async fn create_file_thread(
    http: &Arc<Http>,
    thread_id: ChannelId,
    file_name: &str,
) -> Result<ChannelId> {
    let content =
        format!("📄 **File Store**: `{file_name}`\n*Dữ liệu được lưu trữ trong thread này.*");
    let builder = serenity::builder::CreateMessage::new().content(content);
    let msg = thread_id
        .send_message(http, builder)
        .await
        .context("Lỗi gửi tin nhắn khởi tạo thread")?;

    let thread_name = if file_name.len() > 100 {
        &file_name[..100]
    } else {
        file_name
    };

    let thread = thread_id
        .create_thread_from_message(
            http,
            msg.id,
            serenity::builder::CreateThread::new(thread_name),
        )
        .await
        .context("Lỗi tạo thread từ tin nhắn")?;

    info!("🧵 Đã tạo thread mới cho file: {}", thread.name);
    Ok(thread.id)
}

pub async fn archive_thread(http: &Arc<Http>, thread_id: u64) -> Result<()> {
    let payload = serde_json::json!({ "archived": true });
    http.edit_channel(thread_id.into(), &payload, None)
        .await
        .context("Lỗi lưu trữ (archive) thread")?;
    info!("📦 Đã hoàn tất lưu trữ thread: {}", thread_id);
    Ok(())
}

pub async fn delete_channel(http: &Arc<Http>, thread_id: u64) -> Result<()> {
    ChannelId::new(thread_id).delete(http).await?;
    Ok(())
}

pub async fn delete_file_thread(http: &Arc<Http>, thread_id: u64) -> Result<()> {
    let parent_id = if let Ok(channel) = http.get_channel(thread_id.into()).await {
        channel.guild().and_then(|g| g.parent_id)
    } else {
        None
    };

    let _ = serenity::all::ChannelId::new(thread_id).delete(http).await;

    if let Some(pid) = parent_id {
        let _ = http.delete_message(pid, thread_id.into(), None).await;
    }

    info!("Đã dọn dẹp sạch cả Thread và Tin nhắn gốc: {}", thread_id);
    Ok(())
}

pub async fn delete_category(
    http: &Arc<Http>,
    guild_id: GuildId,
    category_id: u64,
) -> Result<()> {
    let guild = guild_id
        .to_partial_guild(http)
        .await
        .context("Không thể lấy thông tin Server")?;
    let channels = guild
        .channels(http)
        .await
        .context("Không thể lấy danh sách kênh")?;
    let cat_id = ChannelId::new(category_id);

    for ch in channels.values().filter(|c| c.parent_id == Some(cat_id)) {
        move_channel_to_category(http, ch.id.get(), None)
            .await
            .with_context(|| {
                format!(
                    "Lỗi đưa kênh {} ra khỏi category {}",
                    ch.id.get(),
                    category_id
                )
            })?;
    }

    cat_id
        .delete(http)
        .await
        .context("Lỗi xóa Category Discord")?;
    Ok(())
}

pub async fn rename_category(
    http: &Arc<Http>,
    _guild_id: GuildId,
    category_id: u64,
    new_name: &str,
) -> Result<()> {
    let safe = sanitize_name(new_name);
    let builder = serenity::builder::EditChannel::new().name(&safe);
    serenity::model::id::ChannelId::new(category_id)
        .edit(http, builder)
        .await
        .context("Lỗi đổi tên Category Discord")?;
    Ok(())
}

pub async fn send_part(
    http: &Arc<Http>,
    thread_id: ChannelId,
    zip_bytes: Vec<u8>,
    zip_name: String,
    content: String,
) -> Result<(i64, String)> {
    let len = zip_bytes.len();
    let attachment = serenity::builder::CreateAttachment::bytes(zip_bytes, &zip_name);
    let builder = serenity::builder::CreateMessage::new()
        .content(&content)
        .add_file(attachment);
    let msg = thread_id.send_message(http, builder).await.map_err(|e| {
        anyhow!(
            "Lỗi gửi tin nhắn kèm file Discord: {:?} (Dung lượng: {} bytes)",
            e,
            len
        )
    })?;
    let raw_id = msg.id.get();
    let msg_id = i64::try_from(raw_id)
        .map_err(|_| anyhow!("Discord message id {raw_id} vượt quá phạm vi i64"))?;
    Ok((msg_id, msg.link()))
}

pub async fn send_part_batch(
    http: &Arc<Http>,
    thread_id: ChannelId,
    parts: Vec<(Vec<u8>, String)>,
    content: String,
) -> Result<(i64, String)> {
    let mut builder = serenity::builder::CreateMessage::new().content(&content);
    let mut total_len = 0;
    let parts_count = parts.len();

    for (zip_bytes, zip_name) in parts {
        total_len += zip_bytes.len();
        let attachment = serenity::builder::CreateAttachment::bytes(zip_bytes, &zip_name);
        builder = builder.add_file(attachment);
    }

    let msg = thread_id.send_message(http, builder).await.map_err(|e| {
        anyhow!(
            "Lỗi gửi batch {} file lên Discord: {:?} (Tổng dung lượng: {} bytes)",
            parts_count,
            e,
            total_len
        )
    })?;

    let raw_id = msg.id.get();
    let msg_id = i64::try_from(raw_id)
        .map_err(|_| anyhow!("Discord message id {raw_id} vượt quá phạm vi i64"))?;
    Ok((msg_id, msg.link()))
}

pub async fn post_thread_note(
    http: &Arc<Http>,
    thread_id: u64,
    content: &str,
) -> Result<i64> {
    let builder = serenity::builder::CreateMessage::new().content(content);
    let msg = ChannelId::new(thread_id)
        .send_message(http, builder)
        .await
        .context("Failed to send manifest note into Discord thread")?;
    let raw_id = msg.id.get();
    let msg_id = i64::try_from(raw_id)
        .map_err(|_| anyhow!("Discord message id {raw_id} vượt quá phạm vi i64"))?;
    Ok(msg_id)
}

// ponytail: cache 1 Discord API call per message, serves all parts
fn msg_attachment_cache() -> &'static std::sync::Mutex<HashMap<(u64, u64), Vec<(String, String)>>> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<(u64, u64), Vec<(String, String)>>>> = OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

fn find_msg_attachment<'a>(
    entries: &'a [(String, String)],
    part_index: u32,
) -> Option<&'a String> {
    let marker = format!(".part{}", part_index);
    entries.iter().find(|(n, _)| n.ends_with(&marker)).map(|(_, u)| u)
}

pub async fn fetch_attachment_url(
    http: &Arc<Http>,
    thread_id: u64,
    message_id: u64,
    part_index: u32,
) -> Result<String> {
    let cache_key = (thread_id, message_id);
    let marker = format!(".part{}", part_index);
    {
        let cache = msg_attachment_cache().lock().expect("Mutex poisoned");
        if let Some(entries) = cache.get(&cache_key) {
            if let Some(url) = find_msg_attachment(entries, part_index) {
                return Ok(url.clone());
            }
        }
    }
    let entries = fetch_message_attachments(http, thread_id, message_id).await?;
    find_msg_attachment(&entries, part_index)
        .cloned()
        .ok_or_else(|| anyhow!("Tin nhắn không chứa file đính kèm với: {}", marker))
}

pub async fn fetch_message_attachments(
    http: &Arc<Http>,
    thread_id: u64,
    message_id: u64,
) -> Result<Vec<(String, String)>> {
    let msg = ChannelId::new(thread_id)
        .message(http, message_id)
        .await
        .context("Lỗi tìm tin nhắn Discord")?;
    let entries: Vec<_> = msg
        .attachments
        .iter()
        .map(|a| (a.filename.clone(), a.url.clone()))
        .collect();
    msg_attachment_cache().lock().expect("Mutex poisoned").insert((thread_id, message_id), entries.clone());
    Ok(entries)
}

pub async fn move_channel_to_category(
    http: &Arc<Http>,
    thread_id: u64,
    category_id: Option<u64>,
) -> Result<()> {
    match category_id {
        Some(cat) => {
            let payload = serde_json::json!({ "parent_id": cat.to_string() });
            http.edit_channel(thread_id.into(), &payload, None)
                .await
                .context("Lỗi di chuyển kênh vào thư mục")?;
        }
        None => {
            let payload = serde_json::json!({ "parent_id": null });
            http.edit_channel(thread_id.into(), &payload, None)
                .await
                .context("Lỗi khôi phục kênh về thư mục gốc")?;
        }
    }
    Ok(())
}

// ponytail: DiscordBackupGateway wraps DiscordHttp + guild_id for the backup_backend trait
pub struct DiscordBackupGateway {
    http: Arc<Http>,
    guild_id: GuildId,
}

impl DiscordBackupGateway {
    pub fn new(http: Arc<Http>, guild_id: GuildId) -> Self {
        Self { http, guild_id }
    }
}

#[async_trait]
impl DiscordBackupBackend for DiscordBackupGateway {
    async fn list_backup_messages(&self, thread_id: u64, limit: u32) -> Result<Vec<BackupBackendAttachment>, String> {
        let msgs = fetch_backup_messages(&self.http, thread_id, limit as u8)
            .await
            .map_err(|e| format!("List backup messages failed: {e}"))?;
        let mut result = Vec::new();
        for msg in msgs {
            for att in msg.attachments {
                result.push(BackupBackendAttachment {
                    message_id: att.message_id,
                    filename: att.filename,
                    url: att.url,
                    size: att.size,
                });
            }
        }
        Ok(result)
    }

    async fn download_backup_attachment(&self, url: &str) -> Result<Vec<u8>, String> {
        let bytes = reqwest::get(url)
            .await
            .map_err(|e| format!("HTTP get failed: {e}"))?
            .bytes()
            .await
            .map_err(|e| format!("Read body failed: {e}"))?;
        Ok(bytes.to_vec())
    }

    async fn create_backup_thread(&self, name: &str) -> Result<u64, String> {
        let category = get_or_create_category(&self.http, self.guild_id, "Backup")
            .await
            .map_err(|e| format!("Get/create Backup category failed: {e}"))?;
        let channel = get_or_create_fixed_channel(&self.http, self.guild_id, "db", Some(category.id))
            .await
            .map_err(|e| format!("Get/create db channel failed: {e}"))?;
        create_forum_thread(&self.http, channel.id.get(), name)
            .await
            .map_err(|e| format!("Create thread failed: {e}"))
    }

    async fn upload_backup_file(&self, thread_id: u64, data: Vec<u8>, filename: &str) -> Result<(), String> {
        send_backup_message(&self.http, thread_id, data, filename)
            .await
            .map_err(|e| format!("Upload failed: {e}"))
    }

    async fn delete_backup_thread(&self, thread_id: u64) -> Result<(), String> {
        delete_channel(&self.http, thread_id)
            .await
            .map_err(|e| format!("Delete thread failed: {e}"))
    }
}
