use std::{
    collections::HashSet,
    path::Path,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use omega_drive_gateway::core::events::{EventBus, OmegaEvent};
use omega_drive_gateway::provider::{
    file_repository::FileRepository,
    remote_folder::RemoteFolderGateway,
    part_store::PartStoreGateway,
    provider_admin::ProviderAdminGateway,
    provider_types::{
        ByteRange, MediaSource, ProviderConnectionStatus, ProviderUploadConstraints,
        RemoteObjectRef, UploadPartReceipt, UploadPartRequest,
    },
    remote_object::RemoteObjectGateway,
    storage::{
        PartMetadata, ProviderCapability, ProviderMetadata, ProviderQuota, StorageProvider,
    },
    stream::{ProviderByteStream, StreamDownload, StreamGateway},
};
use crate::telegram_real::{TelegramClient, TelegramDownload};

const TELEGRAM_SYNC_PLAYBACK_WARM_IDS: usize = 100;
const TELEGRAM_CONNECTION_PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const TELEGRAM_AUTH_STATE_UNKNOWN: u8 = 0;
const TELEGRAM_AUTH_STATE_OFFLINE: u8 = 1;
const TELEGRAM_AUTH_STATE_ONLINE: u8 = 2;

#[derive(Clone)]
struct TelegramProviderShared {
    enabled: bool,
    configured: bool,
    client: Option<Arc<TelegramClient>>,
    file_repo: Arc<dyn FileRepository>,
    authorized_state: Arc<AtomicU8>,
}

struct TelegramAdminGateway {
    shared: Arc<TelegramProviderShared>,
    storage: Option<Arc<dyn StorageProvider>>,
}

pub struct TelegramPartStoreGateway {
    client: Arc<TelegramClient>,
}

struct TelegramStreamGateway {
    shared: Arc<TelegramProviderShared>,
}

struct TelegramRemoteObjectGateway {
    shared: Arc<TelegramProviderShared>,
}

pub struct TelegramInstallInput {
    pub base_dir: std::path::PathBuf,
    pub configured: bool,
    pub client: Option<Arc<TelegramClient>>,
    pub file_repo: Arc<dyn FileRepository>,
    pub event_bus: Arc<EventBus>,
}

pub struct TelegramInstallOutput {
    pub storage_providers: Vec<Arc<dyn StorageProvider>>,
    pub provider_admin_gateways: Vec<Arc<dyn ProviderAdminGateway>>,
    pub part_store_gateways: Vec<Arc<dyn PartStoreGateway>>,
    pub stream_gateways: Vec<Arc<dyn StreamGateway>>,
    pub remote_folder_gateways: Vec<Arc<dyn RemoteFolderGateway>>,
    pub remote_object_gateways: Vec<Arc<dyn RemoteObjectGateway>>,
}

pub async fn install_telegram(input: TelegramInstallInput) -> Result<TelegramInstallOutput> {
    Ok(build_install_output(input))
}

fn build_install_output(input: TelegramInstallInput) -> TelegramInstallOutput {
    let shared = Arc::new(TelegramProviderShared {
        enabled: input.client.is_some(),
        configured: input.configured,
        client: input.client.clone(),
        file_repo: Arc::clone(&input.file_repo),
        authorized_state: Arc::new(AtomicU8::new(if input.client.is_some() {
            TELEGRAM_AUTH_STATE_UNKNOWN
        } else {
            TELEGRAM_AUTH_STATE_OFFLINE
        })),
    });

    if let Some(tg_client) = None::<Arc<TelegramClient>> {
        let event_bus = Arc::clone(&input.event_bus);
        tokio::spawn(async move {
            let mut last_status: Option<bool> = None;
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                let result =
                    tokio::time::timeout(Duration::from_secs(5), tg_client.is_authorized()).await;
                let is_online = matches!(result, Ok(Ok(true)));
                if last_status != Some(is_online) {
                    last_status = Some(is_online);
                    event_bus.emit(OmegaEvent::TelegramConnectionStatusChanged(is_online));
                    tracing::info!(
                        "Telegram connection status changed: {}",
                        if is_online { "Online" } else { "Offline" }
                    );
                }
            }
        });
    }

    if let Some(active_client) = input.client.clone() {
        spawn_telegram_connection_probe(active_client, &shared, Arc::clone(&input.event_bus));
    }

    let storage = input.client.as_ref().map(|tg| {
        tg.bind_file_repo(Arc::clone(&input.file_repo));
        let storage_impl = Arc::clone(tg);
        let storage: Arc<dyn StorageProvider> = storage_impl;
        storage
    });
    let provider_admin_gateway: Arc<dyn ProviderAdminGateway> = Arc::new(TelegramAdminGateway {
        shared: Arc::clone(&shared),
        storage: storage.clone(),
    });
    let stream_gateway: Arc<dyn StreamGateway> = Arc::new(TelegramStreamGateway {
        shared: Arc::clone(&shared),
    });
    let remote_object_gateway: Arc<dyn RemoteObjectGateway> =
        Arc::new(TelegramRemoteObjectGateway { shared: Arc::clone(&shared) });

    let mut output = TelegramInstallOutput {
        storage_providers: storage.iter().cloned().collect(),
        provider_admin_gateways: vec![provider_admin_gateway],
        part_store_gateways: Vec::new(),
        stream_gateways: vec![stream_gateway],
        remote_folder_gateways: Vec::new(),
        remote_object_gateways: vec![remote_object_gateway],
    };

    if let Some(client) = input.client {
        let part_store_gateway: Arc<dyn PartStoreGateway> =
            Arc::new(TelegramPartStoreGateway { client });
        output.part_store_gateways.push(part_store_gateway);
    }

    output
}

async fn probe_telegram_authorized(client: &TelegramClient) -> bool {
    matches!(
        tokio::time::timeout(TELEGRAM_CONNECTION_PROBE_TIMEOUT, client.is_authorized()).await,
        Ok(Ok(true))
    )
}

fn update_telegram_authorized_state(shared: &TelegramProviderShared, authorized: bool) {
    let next_state = if authorized {
        TELEGRAM_AUTH_STATE_ONLINE
    } else {
        TELEGRAM_AUTH_STATE_OFFLINE
    };
    shared
        .authorized_state
        .store(next_state, Ordering::Relaxed);
}

fn telegram_authorized_from_state(shared: &TelegramProviderShared) -> bool {
    matches!(
        shared.authorized_state.load(Ordering::Relaxed),
        TELEGRAM_AUTH_STATE_ONLINE
    )
}

fn spawn_telegram_connection_probe(
    client: Arc<TelegramClient>,
    shared: &Arc<TelegramProviderShared>,
    event_bus: Arc<EventBus>,
) {
    let shared_weak = Arc::downgrade(shared);
    let client_weak = Arc::downgrade(&client);
    tokio::spawn(async move {
        let Some(shared) = shared_weak.upgrade() else {
            return;
        };
        let Some(client) = client_weak.upgrade() else {
            return;
        };
        let is_online = probe_telegram_authorized(client.as_ref()).await;
        update_telegram_authorized_state(shared.as_ref(), is_online);
        event_bus.emit(OmegaEvent::TelegramConnectionStatusChanged(is_online));
        tracing::info!(
            "Telegram connection status changed: {}",
            if is_online { "Online" } else { "Offline" }
        );
    });
}

fn telegram_metadata() -> ProviderMetadata {
    ProviderMetadata {
        id: "telegram".to_string(),
        display_name: "Telegram Storage".to_string(),
        icon: "mdi-telegram".to_string(),
        description: "Luu tru dung luong lon thong qua Telegram MTProto.".to_string(),
    }
}

fn telegram_capabilities(enabled: bool) -> Vec<ProviderCapability> {
    if enabled {
        vec![
            ProviderCapability::ResumableUpload,
            ProviderCapability::Streaming,
        ]
    } else {
        Vec::new()
    }
}

#[async_trait]
impl ProviderAdminGateway for TelegramAdminGateway {
    fn provider_id(&self) -> &str {
        "telegram"
    }

    fn metadata(&self) -> ProviderMetadata {
        self.storage
            .as_ref()
            .map(|storage| storage.metadata())
            .unwrap_or_else(telegram_metadata)
    }

    fn fetch_capabilities(&self) -> Vec<ProviderCapability> {
        self.storage
            .as_ref()
            .map(|storage| storage.fetch_capabilities())
            .unwrap_or_else(|| telegram_capabilities(self.shared.enabled))
    }

    async fn get_quota(&self) -> Result<ProviderQuota> {
        if let Some(storage) = &self.storage {
            storage.get_quota().await
        } else {
            let used = self.shared.file_repo.get_platform_usage("telegram").await.unwrap_or(0) as u64;
            Ok(ProviderQuota {
                total_bytes: None,
                used_bytes: used,
            })
        }
    }

    async fn connection_status(&self) -> Result<ProviderConnectionStatus> {
        Ok(ProviderConnectionStatus {
            configured: self.shared.configured,
            connected: self.shared.client.is_some(),
            authorized: telegram_authorized_from_state(self.shared.as_ref()),
        })
    }

    async fn fetch_upload_limits(&self) -> Result<ProviderUploadConstraints> {
        Ok(ProviderUploadConstraints::default())
    }

    async fn check_health(&self) -> bool {
        self.shared.client.is_some()
    }
}

#[async_trait]
impl PartStoreGateway for TelegramPartStoreGateway {
    fn provider_id(&self) -> &str {
        "telegram"
    }

    async fn upload_part(&self, request: UploadPartRequest) -> Result<UploadPartReceipt> {
        let size = request.data.len() as u64;
        let file_stem = Path::new(&request.file_name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("part");
        let (message_id, _) = self
            .client
            .send_part_internal(
                request.data,
                request.part_num,
                file_stem,
                &request.caption,
                request.telegram_progress_tx,
            )
            .await?;
        Ok(UploadPartReceipt {
            message_id,
            platform: "telegram".to_string(),
            size,
            attachment_name: None,
        })
    }

    async fn download_part(&self, part: &PartMetadata) -> Result<Vec<u8>> {
        let msg_id = part.message_id.parse::<i64>()?;
        self.client.download_part_internal(msg_id).await
    }

    async fn delete_part(&self, part: &PartMetadata) -> Result<()> {
        let msg_id = part.message_id.parse::<i64>()?;
        self.client.delete_message(msg_id).await
    }

    async fn forward_part(
        &self,
        part: &PartMetadata,
        target_container_id: &str,
    ) -> Result<UploadPartReceipt> {
        let msg_id = part.message_id.parse::<i64>()?;
        let new_msg_id = self
            .client
            .forward_message_internal(msg_id, target_container_id)
            .await?;
        Ok(UploadPartReceipt {
            message_id: new_msg_id,
            platform: "telegram".to_string(),
            size: part.size as u64,
            attachment_name: part.attachment_name.clone(),
        })
    }
}

#[async_trait]
impl StreamGateway for TelegramStreamGateway {
    fn provider_id(&self) -> &str {
        "telegram"
    }

    async fn download_part_bytes(&self, part: &PartMetadata) -> Result<Vec<u8>> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        let msg_id = part.message_id.parse::<i64>()?;
        client.download_part_internal(msg_id).await
    }

    async fn download_part_range(
        &self,
        part: &PartMetadata,
        range: Option<ByteRange>,
    ) -> Result<Vec<u8>> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        let msg_id = part.message_id.parse::<i64>()?;
        match range {
            Some(range) => client.download_part_range_internal(msg_id, range).await,
            None => client.download_part_internal(msg_id).await,
        }
    }

    async fn download_part_range_stream(
        &self,
        part: &PartMetadata,
        range: Option<ByteRange>,
    ) -> Result<ProviderByteStream> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        let msg_id = part.message_id.parse::<i64>()?;
        match range {
            Some(range) => {
                Ok(Arc::clone(client).download_part_range_stream_internal(msg_id, range))
            }
            None => {
                let client = self.shared.client.clone()
                    .ok_or_else(|| anyhow!("Telegram not configured"))?;
                Ok(client.stream_part(msg_id))
            }
        }
    }

    async fn prepare_parts_for_playback(&self, parts: &[PartMetadata]) -> Result<()> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        let file_id = parts.first().map(|p| p.file_id).unwrap_or(0);
        let mut message_ids = Vec::with_capacity(parts.len());
        let mut seen = HashSet::with_capacity(parts.len());
        for part in parts {
            let message_id = part.message_id.parse::<i64>()?;
            if seen.insert(message_id) {
                message_ids.push(message_id);
            }
        }
        if message_ids.is_empty() {
            return Ok(());
        }

        let sync_len = message_ids.len().min(TELEGRAM_SYNC_PLAYBACK_WARM_IDS);
        client.warm_downloadables(file_id, &message_ids[..sync_len]).await?;

        if sync_len < message_ids.len() {
            let client = Arc::clone(client);
            let remaining_ids = message_ids[sync_len..].to_vec();
            tokio::spawn(async move {
                if let Err(err) = client.warm_downloadables(file_id, &remaining_ids).await {
                    tracing::warn!(
                        "Background Telegram playback warmup failed for {} message ids: {}",
                        remaining_ids.len(),
                        err
                    );
                }
            });
        }

        Ok(())
    }

    async fn download_part_to_temp_or_bytes(
        &self,
        part: &PartMetadata,
        threshold_bytes: usize,
        temp_dir: &Path,
    ) -> Result<StreamDownload> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        let msg_id = part.message_id.parse::<i64>()?;
        match client
            .download_part_to_temp_or_bytes(msg_id, threshold_bytes, temp_dir)
            .await?
        {
            TelegramDownload::OnDisk(path) => Ok(StreamDownload::OnDisk(path)),
            TelegramDownload::InMemory(buf) => Ok(StreamDownload::InMemory(buf)),
        }
    }

    async fn resolve_media_source(&self, _part: &PartMetadata) -> Result<MediaSource> {
        Ok(MediaSource::ProviderOwned)
    }
}

#[async_trait]
impl RemoteObjectGateway for TelegramRemoteObjectGateway {
    fn provider_id(&self) -> &str {
        "telegram"
    }

    async fn archive_object(&self, _object: &RemoteObjectRef) -> Result<()> {
        Ok(())
    }

    async fn delete_object(&self, object: &RemoteObjectRef) -> Result<()> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        if let RemoteObjectRef::TelegramMessages { message_ids } = object {
            if !message_ids.is_empty() {
                client.delete_messages_bulk(message_ids.clone()).await?;
            }
        }
        Ok(())
    }

    async fn delete_parts(&self, parts: &[PartMetadata]) -> Result<()> {
        let client = self
            .shared
            .client
            .as_ref()
            .ok_or_else(|| anyhow!("Telegram not configured"))?;
        let mut message_ids = Vec::with_capacity(parts.len());
        for part in parts.iter().filter(|part| part.platform == "telegram") {
            let message_id = part.message_id.parse::<i64>().with_context(|| {
                format!(
                    "Telegram part {} has invalid message_id '{}'",
                    part.id, part.message_id
                )
            })?;
            message_ids.push(message_id);
        }
        if !message_ids.is_empty() {
            client.delete_messages_bulk(message_ids).await?;
        }
        Ok(())
    }

    async fn delete_file_artifacts(&self, _file_id: i64, parts: &[PartMetadata]) -> Result<()> {
        self.delete_parts(parts).await
    }

    async fn object_exists(&self, object: &RemoteObjectRef) -> Result<bool> {
        Ok(matches!(
            object,
            RemoteObjectRef::TelegramMessages { message_ids } if !message_ids.is_empty()
        ))
    }
}
