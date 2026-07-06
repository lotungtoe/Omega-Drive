use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::core::{
    error::AppResult,
    events::EventBus,
    file_types::FileType,
};
use omega_drive_gateway::provider::{
    part_store::PartStoreGateway,
    provider_admin::ProviderAdminGateway,
    provider_types::{
        ByteRange, MediaSource, ProviderConnectionStatus,
        ProviderUploadConstraints, RemoteFolderRef, RemoteObjectRef, RemoteUploadTarget,
        UploadPartReceipt, UploadPartRequest,
    },
    remote_folder::RemoteFolderGateway,
    remote_object::RemoteObjectGateway,
    storage::{
        PartMetadata, ProviderCapability, ProviderMetadata, ProviderQuota, StorageProvider,
    },
    stream::StreamGateway,
};
use tracing::{error, info};

use crate::discord_real as discord_impl;
use crate::discord_real::DiscordStorageProvider;
use crate::discord_types::{DiscordChannelId, DiscordGuildId, DiscordHttp};
use crate::globals;
use crate::DISCORD_CONNECTED;

const DISCORD_ENV_TEMPLATE: &str = r#"# [REQUIRED] Discord Bot Token
# Get at: https://discord.com/developers/applications -> Bot -> Token
DISCORD_TOKEN=
"#;

static DISCORD_CONFIGURED: tokio::sync::RwLock<bool> = tokio::sync::RwLock::const_new(false);

pub fn env_template() -> &'static str {
    DISCORD_ENV_TEMPLATE
}

#[derive(Clone)]
struct DiscordProviderShared {
    http: Arc<DiscordHttp>,
    guild_id: DiscordGuildId,
    configured: bool,
    connected: bool,
    file_repo: Arc<dyn FileRepository>,
    event_bus: Arc<EventBus>,
}

struct DiscordAdminGateway {
    shared: Arc<DiscordProviderShared>,
    storage: Arc<dyn StorageProvider>,
}

struct DiscordPartStoreGateway {
    shared: Arc<DiscordProviderShared>,
    storage: Arc<dyn StorageProvider>,
}

struct DiscordStreamGateway {
    shared: Arc<DiscordProviderShared>,
    storage: Arc<dyn StorageProvider>,
}

struct DiscordRemoteFolderGateway {
    shared: Arc<DiscordProviderShared>,
    provider_id: &'static str,
}

struct DiscordRemoteObjectGateway {
    shared: Arc<DiscordProviderShared>,
    provider_id: &'static str,
}

pub struct DiscordInstallInput {
    pub discord_token: String,
    pub tenant_guild_id: Option<String>,
    pub tenant_scope: String,
    pub file_repo: Arc<dyn FileRepository>,
    pub event_bus: Arc<EventBus>,
}

pub struct DiscordInstallOutput {
    pub http: Arc<DiscordHttp>,
    pub guild_id: DiscordGuildId,
    pub configured: bool,
    pub connected: bool,
    pub storage_providers: Vec<Arc<dyn StorageProvider>>,
    pub provider_admin_gateways: Vec<Arc<dyn ProviderAdminGateway>>,
    pub part_store_gateways: Vec<Arc<dyn PartStoreGateway>>,
    pub stream_gateways: Vec<Arc<dyn StreamGateway>>,
    pub remote_folder_gateways: Vec<Arc<dyn RemoteFolderGateway>>,
    pub remote_object_gateways: Vec<Arc<dyn RemoteObjectGateway>>,
}

pub async fn install_discord(input: DiscordInstallInput) -> AppResult<DiscordInstallOutput> {
    use crate::discord_real::Handler;

    let configured = !input.discord_token.is_empty() && input.tenant_guild_id.is_some();
    let guild_id_num: u64 = input
        .tenant_guild_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .parse()
        .unwrap_or_else(|_| {
            error!("Discord guild id is not a valid integer; provider will fall back to dummy mode.");
            0
        });
    let guild_id = DiscordGuildId::new(if guild_id_num == 0 { 1 } else { guild_id_num });

    let (http, guild_id_val, client_opt) = if configured {
        let (ready_tx, _) = tokio::sync::mpsc::channel::<()>(1);
        let handler = Handler {
            guild_id,
            ready_tx: tokio::sync::Mutex::new(Some(ready_tx)),
            file_repo: Arc::clone(&input.file_repo),
            event_bus: Arc::clone(&input.event_bus),
        };

        match discord_impl::try_start_gateway(&input.discord_token, handler).await {
            Ok((http, Some(_client))) => (http, guild_id, Some(_client)),
            _ => {
                eprintln!("Discord provider fallback to dummy mode because the client could not connect.");
                (Arc::new(DiscordHttp::new("dummy")), DiscordGuildId::new(1), None)
            }
        }
    } else {
        eprintln!("Discord provider is not configured; running in settings-only mode.");
        (Arc::new(DiscordHttp::new("dummy")), DiscordGuildId::new(1), None)
    };

    let connected = client_opt.is_some();
    if let Some(mut client) = client_opt {
        globals::set_http(Arc::clone(&client.http));
        tokio::spawn(async move {
            let _ = client.start().await;
        });
    }

    globals::set_guild_id(guild_id_val);

    // Background active-monitoring task.
    let eb_monitor = Arc::clone(&input.event_bus);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let is_currently_connected = *DISCORD_CONNECTED.read().await;
            if !is_currently_connected {
                continue;
            }
            let tcp_ok = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                tokio::net::TcpStream::connect("gateway.discord.gg:443"),
            )
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false);
            if !tcp_ok {
                let mut conn = DISCORD_CONNECTED.write().await;
                if *conn {
                    *conn = false;
                    eb_monitor.emit(
                        omega_drive_gateway::core::events::OmegaEvent::DiscordConnectionStatusChanged(
                            false,
                        ),
                    );
                    tracing::warn!(
                        "[Discord Monitor] TCP ping failed; switching provider state to offline."
                    );
                }
            }
        }
    });

    {
        let mut conf = DISCORD_CONFIGURED.write().await;
        *conf = configured;
        let mut conn = DISCORD_CONNECTED.write().await;
        *conn = connected;
    }

    info!("Discord provider installer completed.");

    Ok(build_install_output(input, http, guild_id_val, configured, connected))
}

fn build_install_output(
    input: DiscordInstallInput,
    http: Arc<DiscordHttp>,
    guild_id: DiscordGuildId,
    configured: bool,
    connected: bool,
) -> DiscordInstallOutput {
    let remote_folder_provider_id = if input.tenant_scope == "shared" {
        "discord_shared"
    } else {
        "discord"
    };

    let shared = Arc::new(DiscordProviderShared {
        http: Arc::clone(&http),
        guild_id,
        configured,
        connected,
        file_repo: Arc::clone(&input.file_repo),
        event_bus: Arc::clone(&input.event_bus),
    });

    let storage_impl = Arc::new(DiscordStorageProvider {
        http: Arc::clone(&http),
        _guild_id: guild_id,
        file_repo: Arc::clone(&input.file_repo),
    });
    let storage: Arc<dyn StorageProvider> = storage_impl;
    let provider_admin_gateways = vec![Arc::new(DiscordAdminGateway {
        shared: Arc::clone(&shared),
        storage: Arc::clone(&storage),
    }) as Arc<dyn ProviderAdminGateway>];
    let part_store_gateways = vec![Arc::new(DiscordPartStoreGateway {
        shared: Arc::clone(&shared),
        storage: Arc::clone(&storage),
    }) as Arc<dyn PartStoreGateway>];
    let stream_gateways = vec![Arc::new(DiscordStreamGateway {
        shared: Arc::clone(&shared),
        storage: Arc::clone(&storage),
    }) as Arc<dyn StreamGateway>];
    let remote_folder_gateways = vec![Arc::new(DiscordRemoteFolderGateway {
        shared: Arc::clone(&shared),
        provider_id: remote_folder_provider_id,
    }) as Arc<dyn RemoteFolderGateway>];

    let mut remote_object_gateways: Vec<Arc<dyn RemoteObjectGateway>> =
        vec![Arc::new(DiscordRemoteObjectGateway {
            shared: Arc::clone(&shared),
            provider_id: "discord",
        }) as Arc<dyn RemoteObjectGateway>];

    if input.tenant_scope == "shared" {
        remote_object_gateways.push(Arc::new(DiscordRemoteObjectGateway {
            shared: Arc::clone(&shared),
            provider_id: "discord_shared",
        }) as Arc<dyn RemoteObjectGateway>);
    }

    DiscordInstallOutput {
        http,
        guild_id,
        configured,
        connected,
        storage_providers: vec![Arc::clone(&storage)],
        provider_admin_gateways,
        part_store_gateways,
        stream_gateways,
        remote_folder_gateways,
        remote_object_gateways,
    }
}

async fn lookup_discord_thread_id(file_repo: &dyn FileRepository, file_id: i64) -> Result<u64> {
    let file = file_repo.get_file_by_id(file_id).await?
        .ok_or_else(|| anyhow!("Khong tim thay file trong DB: {}", file_id))?;
    let thread_id = file
        .thread_id
        .parse::<u64>()
        .map_err(|_| anyhow!("Thread ID khong hop le: {}", file.thread_id))?;
    Ok(thread_id)
}

async fn fetch_discord_media_source(
    shared: &DiscordProviderShared,
    part: &PartMetadata,
) -> Result<(String, Option<DateTime<Utc>>)> {
    let thread_id = lookup_discord_thread_id(&*shared.file_repo, part.file_id).await?;
    let msg_id: u64 = part.message_id.parse()?;
    let fresh = discord_impl::fetch_attachment_url(
        &shared.http,
        thread_id,
        msg_id,
        part.part_index,
    )
    .await?;
    let expiry = expiry_from_discord_url(&fresh, 10);
    Ok((fresh, Some(expiry)))
}

fn slice_bytes(data: Vec<u8>, range: Option<ByteRange>) -> Vec<u8> {
    let Some(range) = range else {
        return data;
    };
    let start = range.start.min(data.len() as u64) as usize;
    let end = range
        .start
        .saturating_add(range.len)
        .min(data.len() as u64) as usize;
    data[start..end].to_vec()
}

#[async_trait]
impl ProviderAdminGateway for DiscordAdminGateway {
    fn provider_id(&self) -> &str {
        "discord"
    }

    fn metadata(&self) -> ProviderMetadata {
        self.storage.metadata()
    }

    fn fetch_capabilities(&self) -> Vec<ProviderCapability> {
        self.storage.fetch_capabilities()
    }

    async fn get_quota(&self) -> Result<ProviderQuota> {
        self.storage.get_quota().await
    }

    async fn connection_status(&self) -> Result<ProviderConnectionStatus> {
        let flag_active = *DISCORD_CONNECTED.read().await;
        if !flag_active {
            return Ok(ProviderConnectionStatus {
                configured: self.shared.configured,
                connected: false,
                authorized: false,
            });
        }
        let tcp_ok = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::TcpStream::connect("gateway.discord.gg:443"),
        )
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false);
        if !tcp_ok {
            *DISCORD_CONNECTED.write().await = false;
            self.shared
                .event_bus
                .emit(omega_drive_gateway::core::events::OmegaEvent::DiscordConnectionStatusChanged(
                    false,
                ));
            tracing::warn!("[Discord] TCP ping gateway.discord.gg failed; marking provider offline.");
        }
        Ok(ProviderConnectionStatus {
            configured: self.shared.configured,
            connected: tcp_ok,
            authorized: tcp_ok,
        })
    }

    async fn fetch_upload_limits(&self) -> Result<ProviderUploadConstraints> {
        let max_part_bytes =
            discord_impl::fetch_guild_upload_limit(&self.shared.http, self.shared.guild_id)
                .await?;
        Ok(ProviderUploadConstraints {
            max_part_bytes: max_part_bytes.map(|v| v as u64),
        })
    }

    async fn check_health(&self) -> bool {
        self.shared.connected
    }
}

#[async_trait]
impl PartStoreGateway for DiscordPartStoreGateway {
    fn provider_id(&self) -> &str {
        "discord"
    }

    async fn upload_part(&self, request: UploadPartRequest) -> Result<UploadPartReceipt> {
        let RemoteUploadTarget::DiscordThread { thread_id, .. } = request.target;
        let size = request.data.len() as u64;
        let attachment_name = request.file_name.clone();
        let (message_id, _) = discord_impl::send_part(
            &self.shared.http,
            DiscordChannelId::new(thread_id),
            request.data,
            request.file_name,
            request.caption,
        )
        .await?;
        Ok(UploadPartReceipt {
            message_id,
            platform: "discord".to_string(),
            size,
            attachment_name: Some(attachment_name),
        })
    }

    async fn upload_parts_batch(
        &self,
        requests: Vec<UploadPartRequest>,
    ) -> Result<Vec<UploadPartReceipt>> {
        let mut requests = requests.into_iter();
        let Some(first) = requests.next() else {
            return Ok(vec![]);
        };
        let UploadPartRequest {
            target,
            data,
            file_name,
            caption,
            ..
        } = first;
        let RemoteUploadTarget::DiscordThread { thread_id, .. } = target;
        let thread_id = DiscordChannelId::new(thread_id);

        let mut parts_payload = Vec::new();
        let mut receipts_meta = Vec::new();
        let first_size = data.len() as u64;
        parts_payload.push((data, file_name.clone()));
        receipts_meta.push((first_size, file_name));

        for request in requests {
            let size = request.data.len() as u64;
            parts_payload.push((request.data, request.file_name.clone()));
            receipts_meta.push((size, request.file_name));
        }

        let (message_id, _link) =
            discord_impl::send_part_batch(&self.shared.http, thread_id, parts_payload, caption)
                .await?;

        let receipts = receipts_meta
            .into_iter()
            .map(|(size, file_name)| UploadPartReceipt {
                message_id,
                platform: "discord".to_string(),
                size,
                attachment_name: Some(file_name),
            })
            .collect();
        Ok(receipts)
    }

    async fn download_part(&self, part: &PartMetadata) -> Result<Vec<u8>> {
        self.storage.download_part(part).await
    }

    async fn delete_part(&self, part: &PartMetadata) -> Result<()> {
        self.storage.delete_part(part).await
    }

    async fn forward_part(
        &self,
        part: &PartMetadata,
        target_container_id: &str,
    ) -> Result<UploadPartReceipt> {
        let bytes = self.download_part(part).await?;
        let thread_id = target_container_id.parse::<u64>()?;
        let attachment_name = part
            .attachment_name
            .clone()
            .unwrap_or_else(|| format!("part_{}.bin", part.part_index));
        let (message_id, _) = discord_impl::send_part(
            &self.shared.http,
            DiscordChannelId::new(thread_id),
            bytes,
            attachment_name.clone(),
            format!("Forwarded Part {}", part.part_index),
        )
        .await?;
        Ok(UploadPartReceipt {
            message_id,
            platform: "discord".to_string(),
            size: part.size as u64,
            attachment_name: Some(attachment_name),
        })
    }
}

#[async_trait]
impl StreamGateway for DiscordStreamGateway {
    fn provider_id(&self) -> &str {
        "discord"
    }

    async fn download_part_bytes(&self, part: &PartMetadata) -> Result<Vec<u8>> {
        self.storage.download_part(part).await
    }

    async fn download_part_range(
        &self,
        part: &PartMetadata,
        range: Option<ByteRange>,
    ) -> Result<Vec<u8>> {
        let data = self.storage.download_part(part).await?;
        Ok(slice_bytes(data, range))
    }

    async fn resolve_media_source(&self, part: &PartMetadata) -> Result<MediaSource> {
        let (url, expiry) = fetch_discord_media_source(self.shared.as_ref(), part).await?;
        Ok(MediaSource::ResolvedUrl { url, expiry })
    }

    async fn resolve_message_attachments(
        &self,
        part: &PartMetadata,
    ) -> Result<Vec<(String, String)>> {
        discord_impl::fetch_message_attachments(
            &self.shared.http,
            lookup_discord_thread_id(&*self.shared.file_repo, part.file_id).await?,
            part.message_id.parse()?,
        )
        .await
    }
}

#[async_trait]
impl RemoteFolderGateway for DiscordRemoteFolderGateway {
    fn provider_id(&self) -> &str {
        self.provider_id
    }

    async fn create_folder(
        &self,
        name: &str,
        _parent: Option<&RemoteFolderRef>,
    ) -> Result<RemoteFolderRef> {
        let cat =
            discord_impl::get_or_create_category(&self.shared.http, self.shared.guild_id, name)
                .await?;
        Ok(RemoteFolderRef {
            provider_id: self.provider_id.to_string(),
            remote_id: cat.id.get().to_string(),
        })
    }

    async fn rename_folder(&self, folder: &RemoteFolderRef, new_name: &str) -> Result<()> {
        let category_id = folder.remote_id.parse::<u64>()?;
        discord_impl::rename_category(&self.shared.http, self.shared.guild_id, category_id, new_name)
            .await
    }

    async fn delete_folder(&self, folder: &RemoteFolderRef) -> Result<()> {
        let category_id = folder.remote_id.parse::<u64>()?;
        discord_impl::delete_category(&self.shared.http, self.shared.guild_id, category_id).await
    }

    async fn ensure_upload_target(
        &self,
        file_name: &str,
        folder: Option<&RemoteFolderRef>,
    ) -> Result<RemoteUploadTarget> {
        let parent_category_id = if let Some(folder) = folder {
            DiscordChannelId::new(folder.remote_id.parse::<u64>()?)
        } else {
            discord_impl::get_or_create_category(&self.shared.http, self.shared.guild_id, "Files")
                .await?
                .id
        };
        let file_type = file_type_from_filename(file_name);
        let channel_name = match file_type {
            FileType::Unknown => discord_impl::sanitize_name(file_name),
            _ => file_type.shared_drive_channel().to_string(),
        };
        let target_channel = discord_impl::get_or_create_fixed_channel(
            &self.shared.http,
            self.shared.guild_id,
            &channel_name,
            Some(parent_category_id),
        )
        .await?;
        let thread_id =
            discord_impl::create_file_thread(&self.shared.http, target_channel.id, file_name)
                .await?;
        Ok(RemoteUploadTarget::DiscordThread {
            thread_id: thread_id.get(),
            archive_on_finalize: true,
        })
    }
}

#[async_trait]
impl RemoteObjectGateway for DiscordRemoteObjectGateway {
    fn provider_id(&self) -> &str {
        self.provider_id
    }

    async fn archive_object(&self, object: &RemoteObjectRef) -> Result<()> {
        let RemoteObjectRef::DiscordThread { thread_id } = object else {
            return Ok(());
        };
        discord_impl::archive_thread(&self.shared.http, *thread_id).await
    }

    async fn delete_object(&self, object: &RemoteObjectRef) -> Result<()> {
        match object {
            RemoteObjectRef::DiscordThread { thread_id } => {
                discord_impl::delete_file_thread(&self.shared.http, *thread_id).await
            }
            RemoteObjectRef::DiscordChannel { thread_id } => {
                discord_impl::delete_channel(&self.shared.http, *thread_id)
                    .await
                    .context("Failed to delete Discord channel")?;
                Ok(())
            }
            RemoteObjectRef::DiscordMessage {
                thread_id,
                message_id,
            } => {
                discord_impl::delete_discord_message(&self.shared.http, *thread_id, *message_id)
                    .await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    async fn delete_file_artifacts(&self, file_id: i64, _parts: &[PartMetadata]) -> Result<()> {
        let thread_id = lookup_discord_thread_id(&*self.shared.file_repo, file_id).await?;
        info!("Deleting full Discord thread {} for file {}.", thread_id, file_id);
        discord_impl::delete_file_thread(&self.shared.http, thread_id).await
    }

    async fn object_exists(&self, object: &RemoteObjectRef) -> Result<bool> {
        match object {
            RemoteObjectRef::DiscordThread { thread_id } => {
                discord_impl::channel_exists(&self.shared.http, *thread_id).await
            }
            RemoteObjectRef::DiscordMessage {
                thread_id,
                message_id,
            } => discord_impl::message_exists(&self.shared.http, *thread_id, *message_id).await,
            RemoteObjectRef::DiscordChannel { thread_id } => {
                discord_impl::channel_exists(&self.shared.http, *thread_id).await
            }
            _ => Ok(false),
        }
    }

    async fn post_note(&self, object: &RemoteObjectRef, content: &str) -> Result<()> {
        match object {
            RemoteObjectRef::DiscordThread { thread_id }
            | RemoteObjectRef::DiscordChannel { thread_id } => {
                discord_impl::post_thread_note(&self.shared.http, *thread_id, content).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

// ██ Utility functions (inlined from omega_drive_core to remove that dependency) ██

fn normalize_extension(ext: &str) -> Option<String> {
    let trimmed = ext.trim().trim_start_matches('.').to_lowercase();
    if trimmed.is_empty() { None } else { Some(trimmed) }
}

fn file_type_from_extension(ext: &str) -> Option<FileType> {
    match ext {
        "pdf" | "doc" | "docx" | "txt" | "rtf" | "md" | "mdx" | "ppt" | "pptx" | "odt" | "epub"
        | "srt" | "vtt" | "ass" | "ssa" | "sub" => Some(FileType::Document),
        "xls" | "xlsx" | "csv" | "tsv" | "ods" => Some(FileType::Sheet),
        "json" | "jsonc" | "xml" | "html" | "htm" | "css" | "js" | "jsx" | "ts" | "tsx" | "py"
        | "java" | "c" | "cpp" | "h" | "hpp" | "rs" | "go" | "rb" | "php" | "swift" | "kt"
        | "dart" | "lua" | "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" | "ini"
        | "cfg" | "toml" | "yaml" | "yml" | "vue" | "svelte" | "scss" | "sass" | "less" | "env"
        | "log" | "conf" | "properties" | "gradle" | "kts" | "stylus" | "styl" | "sql" => Some(FileType::Code),
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "heic" | "avif" | "ico"
        | "tiff" | "tif" | "raw" | "dng" | "cr2" | "cr3" | "nef" | "arw" | "orf" | "sr2"
        | "srw" | "pef" | "rw2" | "raf" | "3fr" | "dcr" | "erf" | "fff" | "kdc" | "mos" | "mrw"
        | "nrw" | "x3f" => Some(FileType::Image),
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" | "flv" | "m2ts" | "mpeg" | "mpg"
        | "3gp" | "3g2" | "asf" | "wmv" | "vob" | "ogv" | "rm" | "rmvb" | "mxf" | "f4v" | "f4p"
        | "dv" | "nut" | "m3u8" => Some(FileType::Video),
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "wma" => Some(FileType::Audio),
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => Some(FileType::Archive),
        _ => None,
    }
}

fn file_type_from_filename(filename: &str) -> FileType {
    std::path::Path::new(filename)
        .extension()
        .and_then(|v| v.to_str())
        .and_then(normalize_extension)
        .and_then(|ext| file_type_from_extension(&ext))
        .unwrap_or(FileType::Unknown)
}

fn parse_discord_expires(url: &str) -> Option<DateTime<Utc>> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let key = it.next()?;
        let value = it.next().unwrap_or("");
        if key == "ex" && !value.is_empty() {
            if let Ok(ts) = u64::from_str_radix(value, 16) {
                let ts_i64 = i64::try_from(ts).ok()?;
                return DateTime::from_timestamp(ts_i64, 0);
            }
        }
    }
    None
}

fn expiry_from_discord_url(url: &str, fallback_minutes: i64) -> DateTime<Utc> {
    parse_discord_expires(url)
        .unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(fallback_minutes))
}
