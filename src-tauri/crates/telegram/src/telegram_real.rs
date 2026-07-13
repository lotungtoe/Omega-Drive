//! telegram.rs — Send/receive data via Telegram.
//!
//! Omega version:
//! - Uses `grammers` library (MTProto) for direct Telegram communication.
//! - Supports login via OTP and 2FA password.
//! - Automatically manages sessions via SQLite.

use omega_drive_gateway::provider::{
    file_repository::FileRepository,
    provider_types::ByteRange,
    storage::{PartMetadata, ProviderCapability, ProviderMetadata, ProviderQuota, StorageProvider},
    stream::ProviderByteStream,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use fs2::FileExt;
use futures_util::stream;
use grammers_client::{media::Downloadable, message::InputMessage, Client};
use grammers_mtsender::{InvocationError, SenderPool};
use grammers_session::types::{PeerAuth, PeerId, PeerRef};
use grammers_tl_types as tl;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio::{
    io::{AsyncRead, ReadBuf},
    sync::OnceCell,
};

use crate::telegram_session::FileTelegramSession;

const TELEGRAM_MAX_DOWNLOAD_CHUNK: i32 = 512 * 1024;

// ─── ProgressReader ──────────────────────────────────────────────────────────

/// Filter to track read progress (used to show upload % in large chunks).
pub struct ProgressReader<R> {
    inner: R,
    tx: Option<tokio::sync::mpsc::UnboundedSender<usize>>,
}

pub enum TelegramDownload {
    InMemory(Vec<u8>),
    OnDisk(PathBuf),
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct TelegramMetadataCache {
    id: i64,
    access_hash: i64,
    file_reference: Vec<u8>,
}

impl TelegramMetadataCache {
    fn from_input_location(loc: &tl::enums::InputFileLocation) -> Option<Self> {
        match loc {
            tl::enums::InputFileLocation::InputDocumentFileLocation(d) => Some(Self {
                id: d.id,
                access_hash: d.access_hash,
                file_reference: d.file_reference.clone(),
            }),
            tl::enums::InputFileLocation::InputEncryptedFileLocation(e) => Some(Self {
                id: e.id,
                access_hash: e.access_hash,
                file_reference: Vec::new(), // Encrypted doesn't have file_ref in the same way
            }),
            _ => None,
        }
    }
}

#[derive(Clone)]
struct TelegramDownloadableRef {
    location: Option<tl::enums::InputFileLocation>,
    data: Option<Vec<u8>>,
    size: Option<usize>,
}

impl Downloadable for TelegramDownloadableRef {
    fn to_raw_input_location(&self) -> Option<tl::enums::InputFileLocation> {
        self.location.clone()
    }

    fn to_data(&self) -> Option<Vec<u8>> {
        self.data.clone()
    }

    fn size(&self) -> Option<usize> {
        self.size
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct DiskCacheEntry {
    id: i64,
    access_hash: i64,
    fr: Vec<u8>,
    size: Option<usize>,
    ts: u64,
}

fn parse_chat_id(id_str: &str) -> Result<i64> {
    id_str
        .parse::<i64>()
        .map_err(|e| anyhow!("Invalid Telegram ID: {}. Error: {}", id_str, e))
}

fn slice_range_bytes(data: &[u8], range: ByteRange) -> Vec<u8> {
    let start = range.start.min(data.len() as u64) as usize;
    let end = range.start.saturating_add(range.len).min(data.len() as u64) as usize;
    data[start..end].to_vec()
}

fn empty_provider_stream() -> ProviderByteStream {
    Box::pin(stream::empty())
}

fn receiver_provider_stream(rx: mpsc::Receiver<Result<Bytes>>) -> ProviderByteStream {
    Box::pin(stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    }))
}

pub fn cleanup_telegram_temp_files(max_age: Duration) {
    let temp_dir = std::env::temp_dir();
    let Ok(entries) = std::fs::read_dir(&temp_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => continue,
        };
        if !name.starts_with("omega_tg_") || !name.ends_with(".tmp") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified.elapsed().map(|d| d < max_age).unwrap_or(true) {
            continue;
        }

        let Ok(file) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        else {
            continue;
        };
        if file.try_lock_exclusive().is_ok() {
            drop(file);
            let _ = std::fs::remove_file(&path);
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        match Pin::new(&mut self.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let after = buf.filled().len();
                let bytes_read = after - before;
                if bytes_read > 0 {
                    if let Some(tx) = &self.tx {
                        let _ = tx.send(bytes_read);
                    }
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

// ─── DownloadRateLimiter ──────────────────────────────────────────────────────

/// Token bucket rate limiter cho Telegram download requests.
/// Refill 1 permit per interval, limits sustained rate.
/// Max burst = initial `max_permits`.
struct DownloadRateLimiter {
    sem: tokio::sync::Semaphore,
}

impl DownloadRateLimiter {
    fn new(max_permits: usize, refill_per_sec: f64) -> Arc<Self> {
        let this = Arc::new(Self {
            sem: tokio::sync::Semaphore::new(max_permits),
        });
        let weak = Arc::downgrade(&this);
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_secs_f64(1.0 / refill_per_sec);
            while weak.upgrade().is_some() {
                tokio::time::sleep(interval).await;
                if let Some(s) = weak.upgrade() {
                    match s.sem.try_acquire() {
                        Ok(p) => drop(p),
                        Err(tokio::sync::TryAcquireError::NoPermits) => {
                            s.sem.add_permits(1);
                        }
                        Err(tokio::sync::TryAcquireError::Closed) => break,
                    }
                }
            }
        });
        this
    }
}

// ─── TelegramClient ──────────────────────────────────────────────────────────

/// TelegramClient: Central object handling all Telegram API communication.
pub struct TelegramClient {
    client: Client,
    chat_id: String,
    cached_chat: OnceCell<PeerRef>,
    downloadable_cache: tokio::sync::RwLock<HashMap<i64, TelegramDownloadableRef>>,
    pub file_repo: OnceCell<Arc<dyn FileRepository>>,
    download_limiter: Arc<DownloadRateLimiter>,
    seek_limiter: Arc<tokio::sync::Semaphore>,
    cache_dir: PathBuf,
}

impl TelegramClient {
    /// Initialize connection and set up Telegram controller.
    pub async fn connect(
        api_id: i32,
        _api_hash: &str,
        _phone: &str,
        chat_id: &str,
        session_path: &str,
        cache_dir: &Path,
    ) -> Result<Arc<Self>> {
        // Open SQLite session file to avoid re-login.
        let session = Arc::new(
            FileTelegramSession::open(session_path)
                .with_context(|| format!("Khong the mo/tao file session: {session_path}"))?,
        );

        let pool = SenderPool::new(Arc::clone(&session), api_id);
        let fat_handle = pool.handle.clone();

        // Run background MTProto packet handler.
        tokio::spawn(async move { pool.runner.run().await });

        let client = Client::new(fat_handle);

        // Check if the application is authorized.
        if !client.is_authorized().await? {
            tracing::info!(
                "🔐 Telegram: No valid login session. Use CLI to log in."
            );
        } else {
            tracing::info!("✅ Telegram: Successfully restored previous session.");
        }

        const RATE_LIMITER_BURST: usize = 4;
        const RATE_LIMITER_REFILL_PER_SEC: f64 = 3.0;
        let tel_cache_dir = cache_dir.join("telegram");
        let _ = std::fs::create_dir_all(&tel_cache_dir);
        Ok(Arc::new(Self {
            client,
            chat_id: chat_id.to_string(),
            cached_chat: OnceCell::new(),
            downloadable_cache: tokio::sync::RwLock::new(HashMap::new()),
            file_repo: OnceCell::new(),
            download_limiter: DownloadRateLimiter::new(RATE_LIMITER_BURST, RATE_LIMITER_REFILL_PER_SEC),
            seek_limiter: Arc::new(tokio::sync::Semaphore::new(4)),
            cache_dir: tel_cache_dir,
        }))
    }

    /// Bind FileRepository to instance after connection is initialized.
    pub fn bind_file_repo(&self, file_repo: Arc<dyn FileRepository>) {
        let _ = self.file_repo.set(file_repo);
    }

    fn file_ref_cache_path(&self, file_id: i64) -> PathBuf {
        self.cache_dir.join(format!("{}.json", file_id))
    }

    async fn load_file_ref_cache(&self, file_id: i64) -> HashMap<i64, TelegramDownloadableRef> {
        let path = self.file_ref_cache_path(file_id);
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(_) => return HashMap::new(),
        };
        let now = std::time::UNIX_EPOCH.elapsed().unwrap_or_default().as_secs();
        serde_json::from_str::<HashMap<String, DiskCacheEntry>>(&content)
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, e)| e.ts > now.saturating_sub(24 * 3600))
            .filter_map(|(k, e): (String, DiskCacheEntry)| {
                let msg_id = k.parse::<i64>().ok()?;
                Some((msg_id, TelegramDownloadableRef {
                    location: Some(tl::enums::InputFileLocation::InputDocumentFileLocation(
                        tl::types::InputDocumentFileLocation {
                            id: e.id,
                            access_hash: e.access_hash,
                            file_reference: e.fr,
                            thumb_size: String::new(),
                        },
                    )),
                    data: None,
                    size: e.size,
                }))
            })
            .collect()
    }

    async fn save_file_ref_cache(&self, file_id: i64, entries: &HashMap<i64, TelegramDownloadableRef>) {
        if entries.is_empty() { return; }
        let now = std::time::UNIX_EPOCH.elapsed().unwrap_or_default().as_secs();
        let json: HashMap<String, DiskCacheEntry> = entries.iter()
            .filter_map(|(&msg_id, dl)| {
                let loc = match dl.location.as_ref()? {
                    tl::enums::InputFileLocation::InputDocumentFileLocation(d) => d,
                    _ => return None,
                };
                Some((msg_id.to_string(), DiskCacheEntry {
                    id: loc.id,
                    access_hash: loc.access_hash,
                    fr: loc.file_reference.clone(),
                    size: dl.size,
                    ts: now,
                }))
            })
            .collect();
        if json.is_empty() { return; }
        let path = self.file_ref_cache_path(file_id);
        if let Ok(data) = serde_json::to_string(&json) {
            let _ = tokio::fs::write(&path, data).await;
        }
    }

    async fn fetch_downloadables_from_telegram(&self, message_ids: &[i64]) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let chat = self.get_chat().await?;
        for batch in message_ids.chunks(100) {
            let ids: Vec<i32> = batch
                .iter()
                .copied()
                .map(|message_id| {
                    i32::try_from(message_id)
                        .map_err(|_| anyhow!("ID Telegram {message_id} vuot pham vi i32"))
                })
                .collect::<Result<_>>()?;
            let msgs = self
                .client
                .get_messages_by_id(chat, &ids)
                .await
                .context("Loi truy van batch tin nhan Telegram")?;

            let mut resolved = Vec::new();
            for msg in msgs.into_iter().flatten() {
                let Some(media) = msg.media() else {
                    continue;
                };
                resolved.push((
                    i64::from(msg.id()),
                    TelegramDownloadableRef {
                        location: media.to_raw_input_location(),
                        data: media.to_data(),
                        size: media.size(),
                    },
                ));
            }

            if !resolved.is_empty() {
                let mut cache = self.downloadable_cache.write().await;
                for (message_id, downloadable) in &resolved {
                    cache.insert(*message_id, downloadable.clone());
                }
            }
        }

        Ok(())
    }

    /// Check if client is authorized (logged in).
    pub async fn is_authorized(&self) -> Result<bool> {
        self.client
            .is_authorized()
            .await
            .context("Error checking login status")
    }

    /// Get peer info for sending/receiving data.
    async fn get_chat(&self) -> Result<PeerRef> {
        let chat = self
            .cached_chat
            .get_or_try_init(|| self.resolve_chat())
            .await?;
        Ok(*chat)
    }

    async fn resolve_downloadable(&self, message_id: i64) -> Result<TelegramDownloadableRef> {
        tracing::warn!(target: "feature::player", "🔍 Telegram: Resolving downloadable for message {}", message_id);
        let debug_mode = std::env::var("DEBUG").is_ok();

        // 1. Check Memory Cache
        if let Some(cached) = self
            .downloadable_cache
            .read()
            .await
            .get(&message_id)
            .cloned()
        {
            if debug_mode {
                tracing::warn!(target: "feature::player", "[Stream] Metadata for Message {}: HIT (Memory Cache)", message_id);
            }
            return Ok(cached);
        }

        if debug_mode {
            tracing::info!(
                "[Stream] Metadata for Message {}: MISS (Calling Telegram API...)",
                message_id
            );
        }

        // 3. Fallback: Query Telegram API
        let chat = self.get_chat().await?;
        let message_id_i32 = i32::try_from(message_id)
            .map_err(|_| anyhow!("ID Telegram {message_id} vuot pham vi i32"))?;

        let msgs = self
            .client
            .get_messages_by_id(chat, &[message_id_i32])
            .await
            .context("Loi truy van tin nhan Telegram")?;

        let msg = msgs
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| anyhow!("Khong tim thay tin nhan {message_id} tren Telegram."))?;

        let media = msg
            .media()
            .ok_or_else(|| anyhow!("Tin nhan {message_id} khong chua du lieu file!"))?;

        let downloadable = TelegramDownloadableRef {
            location: media.to_raw_input_location(),
            data: media.to_data(),
            size: media.size(),
        };

        // Save to Memory Cache
        let t_write = Instant::now();
        self.downloadable_cache
            .write()
            .await
            .insert(message_id, downloadable.clone());
        let t_write = t_write.elapsed();
        tracing::debug!(target: "telegram::trace", "tg_cache: msg={} write_lock={}µs (api_fetch)", message_id, t_write.as_micros());

        Ok(downloadable)
    }

    pub async fn warm_downloadables(&self, file_id: i64, message_ids: &[i64]) -> Result<()> {
        let disk_entries = self.load_file_ref_cache(file_id).await;
        if !disk_entries.is_empty() {
            let mut cache = self.downloadable_cache.write().await;
            for (msg_id, dl) in disk_entries {
                cache.entry(msg_id).or_insert(dl);
            }
        }

        let mut missing_ids = Vec::new();
        {
            let cache = self.downloadable_cache.read().await;
            for &message_id in message_ids {
                if !cache.contains_key(&message_id) {
                    missing_ids.push(message_id);
                }
            }
        }
        if missing_ids.is_empty() {
            return Ok(());
        }

        self.fetch_downloadables_from_telegram(&missing_ids).await?;

        let mut to_save = HashMap::new();
        let cache = self.downloadable_cache.read().await;
        for &msg_id in message_ids {
            if let Some(dl) = cache.get(&msg_id) {
                to_save.insert(msg_id, dl.clone());
            }
        }
        drop(cache);
        self.save_file_ref_cache(file_id, &to_save).await;

        Ok(())
    }

    async fn download_range_segment(
        &self,
        location: &tl::enums::InputFileLocation,
        range: ByteRange,
    ) -> Result<Vec<u8>> {
        const WORKERS: usize = 4;
        const CHUNK: i32 = TELEGRAM_MAX_DOWNLOAD_CHUNK;
        let chunk_align = CHUNK as u64;

        let start = range.start;
        let raw_end = start.saturating_add(range.len);
        let fetch_start = start & !(chunk_align - 1);
        let end = (raw_end + chunk_align - 1) & !(chunk_align - 1);
        let head = (start - fetch_start) as usize;

        let mut buf = Vec::with_capacity((end - fetch_start) as usize);

        let mut offset = fetch_start;
        while offset < end {
            let batch_end = (offset + (WORKERS as u64 * CHUNK as u64)).min(end);
            let batch_offsets: Vec<u64> = (0..WORKERS)
                .map(|i| offset + i as u64 * CHUNK as u64)
                .take_while(|&off| off < batch_end)
                .collect();

            let results = futures_util::future::join_all(batch_offsets.iter().map(|&off| {
                let loc = location.clone();
                let lim = Arc::clone(&self.download_limiter);
                async move {
                    let actual_limit = CHUNK.min((end - off) as i32);
                    let req = tl::functions::upload::GetFile {
                        precise: false,
                        cdn_supported: false,
                        location: loc,
                        offset: off as i64,
                        limit: actual_limit,
                    };
                    // ponytail: token bucket rate limiter, tune refill_per_sec if throughput matters
                    let t_acquire = Instant::now();
                    let _permit = lim.sem.acquire().await.unwrap_or_else(|_|
                        panic!("Telegram download limiter closed")
                    );
                    let t_acquire = t_acquire.elapsed();
                    let t_invoke = Instant::now();
                    let r = Self::invoke_download_with_retry(&self.client, &req).await;
                    let t_invoke = t_invoke.elapsed();
                    tracing::debug!(target: "telegram::trace", "tg_req: off={} acquire={}µs invoke={}µs", off, t_acquire.as_micros(), t_invoke.as_micros());
                    r
                }
            }))
            .await;

            for result in results {
                match result? {
                    tl::enums::upload::File::File(f) => buf.extend_from_slice(&f.bytes),
                    tl::enums::upload::File::CdnRedirect(_) => {
                        anyhow::bail!("CDN redirect not supported")
                    }
                }
            }

            offset = batch_end;
        }

        let want = range.len as usize;
        if head >= buf.len() {
            return Ok(Vec::new());
        }
        let end_idx = head.saturating_add(want).min(buf.len());
        Ok(buf[head..end_idx].to_vec())
    }

    /// Invoke upload.getFile with retry on transient errors.
    /// Permanent errors (bad params, expired ref) fail immediately.
    async fn invoke_download_with_retry(
        client: &Client,
        req: &tl::functions::upload::GetFile,
    ) -> Result<tl::enums::upload::File> {
        let mut last_err = None;
        for delay_ms in [0u64, 500, 2000] {
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            match client.invoke(req).await {
                Ok(f) => {
                    match &f {
                        tl::enums::upload::File::File(inner) => {
                            tracing::debug!(target: "tg_invoke", "ok off={} File(type={:?}, mtime={}, bytes_len={})", req.offset, inner.r#type, inner.mtime, inner.bytes.len());
                        }
                        tl::enums::upload::File::CdnRedirect(_) => {
                            tracing::debug!(target: "tg_invoke", "ok off={} CdnRedirect", req.offset);
                        }
                    }
                    return Ok(f);
                }
                Err(InvocationError::Rpc(ref rpc))
                    if matches!(
                        rpc.name.as_str(),
                        "FILE_REFERENCE_EXPIRED"
                    ) =>
                {
                    return Err(anyhow!("FILE_REFERENCE_EXPIRED: {}", rpc));
                }
                Err(InvocationError::Rpc(ref rpc))
                    if matches!(
                        rpc.name.as_str(),
                        "LIMIT_INVALID"
                            | "OFFSET_INVALID"
                            | "LOCATION_INVALID"
                            | "FILE_ID_INVALID"
                    ) =>
                {
                    return Err(anyhow!("Permanent RPC error: {}", rpc));
                }
                Err(e) => {
                    tracing::debug!(target: "tg_invoke", "err off={} err={:?}", req.offset, e);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.expect("last_err set in error branch (loop must produce at least one error if we reach here)").into())
    }


    async fn emit_part_range_stream(
        self: Arc<Self>,
        message_id: i64,
        range: ByteRange,
        tx: mpsc::Sender<Result<Bytes>>,
    ) -> Result<()> {
        tracing::warn!(target: "feature::player", "🎞️ Telegram: Starting range stream for message {}", message_id);
        if range.len == 0 {
            return Ok(());
        }

        if std::env::var("DEBUG").is_ok() {
            tracing::warn!(target: "feature::player", "[Stream] Request range: bytes={}-{} (Size: {} KB)", range.start, range.start + range.len - 1, range.len / 1024);
        }

        let downloadable = self.resolve_downloadable(message_id).await?;
        if let Some(data) = downloadable.to_data() {
            let sliced = slice_range_bytes(&data, range);
            if !sliced.is_empty() {
                let _ = tx.send(Ok(Bytes::from(sliced))).await;
            }
            return Ok(());
        }

        let mut location = downloadable
            .location
            .as_ref()
            .ok_or_else(|| anyhow!("Downloadable has no location"))?
            .clone();

        let range_end = downloadable
            .size()
            .map(|size| range.start.saturating_add(range.len).min(size as u64))
            .unwrap_or_else(|| range.start.saturating_add(range.len));

        if range_end <= range.start {
            return Ok(());
        }

        let raw_start = range.start;
        let raw_len = range_end - range.start;

        let mut tried_reresolve = false;
        let r = loop {
            const WORKERS: usize = 4;
            const CHUNK: i32 = TELEGRAM_MAX_DOWNLOAD_CHUNK;
            let chunk_align = CHUNK as u64;

            let fetch_start = raw_start & !(chunk_align - 1);
            let fetch_end = (range_end + chunk_align - 1) & !(chunk_align - 1);
            let mut head_skip = (raw_start - fetch_start) as usize;
            let mut remaining = raw_len;

            let mut offset = fetch_start;
            let mut batch_err = None;
            while offset < fetch_end {
                let batch_end = (offset + WORKERS as u64 * CHUNK as u64).min(fetch_end);
                let batch_offsets: Vec<u64> = (0..WORKERS)
                    .map(|i| offset + i as u64 * CHUNK as u64)
                    .take_while(|&off| off < batch_end)
                    .collect();

                let results = futures_util::future::join_all(batch_offsets.iter().map(|&off| {
                    let loc = location.clone();
                    let lim = Arc::clone(&self.seek_limiter);
                    let client = self.client.clone();
                    async move {
                        let actual_limit = CHUNK.min((fetch_end - off) as i32);
                        let req = tl::functions::upload::GetFile {
                            precise: false,
                            cdn_supported: false,
                            location: loc,
                            offset: off as i64,
                            limit: actual_limit,
                        };
                        let _permit = lim.acquire().await.unwrap_or_else(|_|
                            panic!("Telegram seek limiter closed")
                        );
                        Self::invoke_download_with_retry(&client, &req).await
                    }
                }))
                .await;

                for result in results {
                    if remaining == 0 { break; }

                    let bytes = match result {
                        Ok(tl::enums::upload::File::File(f)) => f.bytes,
                        Ok(tl::enums::upload::File::CdnRedirect(_)) => {
                            anyhow::bail!("CDN redirect not supported")
                        }
                        Err(e) => {
                            batch_err = Some(e);
                            break;
                        }
                    };

                    let data = if head_skip > 0 {
                        if head_skip >= bytes.len() {
                            head_skip -= bytes.len();
                            continue;
                        }
                        let trimmed = &bytes[head_skip..];
                        head_skip = 0;
                        trimmed
                    } else {
                        &bytes[..]
                    };

                    let take = (remaining as usize).min(data.len());
                    if take > 0 {
                        if tx.send(Ok(Bytes::copy_from_slice(&data[..take]))).await.is_err() {
                            return Ok(());
                        }
                        remaining -= take as u64;
                    }
                }

                if batch_err.is_some() { break; }
                offset = batch_end;
            }

            if let Some(e) = batch_err {
                let msg = e.to_string();
                if msg.contains("FILE_REFERENCE_EXPIRED") && !tried_reresolve {
                    self.downloadable_cache.write().await.remove(&message_id);
                    if let Ok(fresh) = self.resolve_downloadable(message_id).await {
                        if let Some(ref loc) = fresh.location {
                            location = loc.clone();
                            tried_reresolve = true;
                            continue;
                        }
                    }
                }
                break Err(e);
            }
            break Ok(());
        };
        r
    }

    pub fn download_part_range_stream_internal(
        self: Arc<Self>,
        message_id: i64,
        range: ByteRange,
    ) -> ProviderByteStream {
        if range.len == 0 {
            return empty_provider_stream();
        }

        let (tx, rx) = mpsc::channel::<Result<Bytes>>(8);
        tokio::spawn(async move {
            if let Err(err) = self
                .emit_part_range_stream(message_id, range, tx.clone())
                .await
            {
                let _ = tx.send(Err(err)).await;
            }
        });

        receiver_provider_stream(rx)
    }

    /// Look up actual channel/user info by ID or Username.
    async fn resolve_chat(&self) -> Result<PeerRef> {
        let raw_id = parse_chat_id(&self.chat_id)?;

        // Handle regular user IDs
        if raw_id > 0 {
            return Ok(PeerRef {
                id: PeerId::user(raw_id).expect("Invalid User ID"),
                auth: PeerAuth::from_hash(0),
            });
        }

        // Handle legacy group/chat IDs
        if raw_id > -1_000_000_000_000 {
            let bare_id = raw_id.abs();
            return Ok(PeerRef {
                id: PeerId::chat(bare_id).expect("Invalid Chat ID"),
                auth: PeerAuth::from_hash(0),
            });
        }

        // Handle Channel or Supergroup IDs - Usually starts with -100...
        let channel_id = raw_id.abs() - 1_000_000_000_000;

        let result = self
            .client
            .invoke(&tl::functions::channels::GetChannels {
                id: vec![tl::enums::InputChannel::Channel(tl::types::InputChannel {
                    channel_id,
                    access_hash: 0,
                })],
            })
            .await
            .context("Telegram channel not found. Make sure you have joined this channel.")?;

        let chats = match &result {
            tl::enums::messages::Chats::Chats(c) => &c.chats,
            tl::enums::messages::Chats::Slice(c) => &c.chats,
        };

        let access_hash = chats
            .iter()
            .find_map(|ch| match ch {
                tl::enums::Chat::Channel(c) if c.id == channel_id => {
                    Some(c.access_hash.unwrap_or(0))
                }
                _ => None,
            })
            .unwrap_or(0);

        Ok(PeerRef {
            id: PeerId::channel(channel_id).expect("Invalid Channel ID"),
            auth: PeerAuth::from_hash(access_hash),
        })
    }

    /// Send a binary data part to Telegram as a file attachment.
    pub async fn send_part_internal(
        &self,
        data: Vec<u8>,
        part_num: u32,
        filename: &str,
        caption: &str,
        progress_tx: Option<tokio::sync::mpsc::UnboundedSender<usize>>,
    ) -> Result<(i64, Option<String>)> {
        let chat = self.get_chat().await?;
        let part_name = format!("{filename}.part{part_num}");
        let size = data.len();

        tracing::info!(
            "  📨 Telegram: Uploading part {part_num} ({:.1} MB)",
            size as f64 / 1_048_576.0
        );

        let cursor = std::io::Cursor::new(data);
        let progress_reader = ProgressReader {
            inner: cursor,
            tx: progress_tx,
        };
        let mut reader = tokio::io::BufReader::new(progress_reader);

        // Upload data stream to Telegram server
        let uploaded = self
            .client
            .upload_stream(&mut reader, size, part_name)
            .await
            .context("Error uploading stream to Telegram")?;

        // Send actual message with uploaded file
        let msg = self
            .client
            .send_message(chat, InputMessage::new().text(caption).document(uploaded))
            .await
            .context("Error sending message with Telegram file")?;

        tracing::info!(
            "  ✅ Telegram: Sent part {part_num}. Message ID: {}",
            msg.id()
        );

        // Serialize metadata to save to DB immediately
        let mut meta_json = None;
        if let Some(media) = msg.media() {
            if let Some(loc) = media.to_raw_input_location() {
                if let Some(meta_cache) = TelegramMetadataCache::from_input_location(&loc) {
                    meta_json = serde_json::to_string(&meta_cache).ok();
                }
            }
        }

        Ok((i64::from(msg.id()), meta_json))
    }

    /// Download a file from Telegram by message ID.
    pub async fn download_part_internal(&self, message_id: i64) -> Result<Vec<u8>> {
        let downloadable = self.resolve_downloadable(message_id).await?;
        let mut buf = Vec::with_capacity(downloadable.size().unwrap_or_default());
        let mut dl = self.client.iter_download(&downloadable);

        // Load data chunks into memory.
        while let Some(chunk) = dl.next().await.context("Error downloading data from Telegram")?
        {
            buf.extend_from_slice(&chunk);
        }

        tracing::info!(
            "  ✅ Telegram: Downloaded {:.1} MB from message ID {message_id}",
            buf.len() as f64 / 1_048_576.0
        );
        Ok(buf)
    }

    pub fn stream_part(
        self: Arc<Self>,
        message_id: i64,
    ) -> ProviderByteStream {
        let (tx, rx) = mpsc::channel::<Result<Bytes>>(8);
        tokio::spawn(async move {
            let downloadable = match self.resolve_downloadable(message_id).await {
                Ok(d) => d,
                Err(e) => { let _ = tx.send(Err(e)).await; return; }
            };
            let mut dl = self.client.iter_download(&downloadable);
            while let Some(chunk) = match dl.next().await {
                Ok(c) => c,
                Err(e) => { let _ = tx.send(Err(anyhow!("Error downloading data from Telegram: {e}"))).await; return; }
            } {
                if tx.send(Ok(Bytes::from(chunk))).await.is_err() {
                    return;
                }
            }
        });
        receiver_provider_stream(rx)
    }

    pub async fn download_part_range_internal(
        &self,
        message_id: i64,
        range: ByteRange,
    ) -> Result<Vec<u8>> {
        if range.len == 0 {
            return Ok(Vec::new());
        }

        let downloadable = self.resolve_downloadable(message_id).await?;
        if let Some(data) = downloadable.to_data() {
            return Ok(slice_range_bytes(&data, range));
        }

        let location = downloadable
            .location
            .as_ref()
            .ok_or_else(|| anyhow!("Downloadable has no location"))?;

        let range_end = downloadable
            .size()
            .map(|size| range.start.saturating_add(range.len).min(size as u64))
            .unwrap_or_else(|| range.start.saturating_add(range.len));

        if range_end <= range.start {
            return Ok(Vec::new());
        }

        let effective_range = ByteRange {
            start: range.start,
            len: range_end.saturating_sub(range.start),
        };

        self.download_range_segment(location, effective_range).await
    }

    /// Load Telegram part into RAM until threshold, then spool to temp file.
    pub async fn download_part_to_temp_or_bytes(
        &self,
        message_id: i64,
        threshold_bytes: usize,
        temp_dir: &Path,
    ) -> Result<TelegramDownload> {
        let downloadable = self.resolve_downloadable(message_id).await?;
        let mut buf = Vec::new();
        let mut file: Option<tokio::fs::File> = None;
        let mut temp_path: Option<PathBuf> = None;

        if threshold_bytes == 0 {
            let path = temp_dir.join(format!(
                "omega_tg_{}_{}.tmp",
                message_id,
                uuid::Uuid::new_v4()
            ));
            let f = tokio::fs::File::create(&path).await?;
            file = Some(f);
            temp_path = Some(path);
        }

        let mut dl = self.client.iter_download(&downloadable);
        while let Some(chunk) = dl.next().await.context("Error downloading data from Telegram")?
        {
            if let Some(f) = file.as_mut() {
                f.write_all(&chunk).await?;
                continue;
            }

            buf.extend_from_slice(&chunk);
            if threshold_bytes > 0 && buf.len() >= threshold_bytes {
                let path = temp_dir.join(format!(
                    "omega_tg_{}_{}.tmp",
                    message_id,
                    uuid::Uuid::new_v4()
                ));
                let mut f = tokio::fs::File::create(&path).await?;
                f.write_all(&buf).await?;
                buf.clear();
                file = Some(f);
                temp_path = Some(path);
            }
        }

        if let Some(mut f) = file {
            f.flush().await?;
            let path = temp_path.ok_or_else(|| anyhow!("Missing temp path"))?;
            return Ok(TelegramDownload::OnDisk(path));
        }

        Ok(TelegramDownload::InMemory(buf))
    }

    /// Delete a Telegram message (when user permanently deletes a file).
    pub async fn delete_message(&self, message_id: i64) -> Result<()> {
        self.delete_messages_bulk(vec![message_id]).await
    }

    /// Batch delete multiple Telegram messages.
    /// Helps avoid Rate Limits and deserialization errors when deleting large files.
    pub async fn delete_messages_bulk(&self, message_ids: Vec<i64>) -> Result<()> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let chat = self.get_chat().await?;

        // Convert to i32 (Telegram Message ID is i32) and filter errors
        let ids: Vec<i32> = message_ids
            .into_iter()
            .filter_map(|id| i32::try_from(id).ok())
            .collect();

        // Telegram allows deleting up to 100 messages per request
        for chunk in ids.chunks(100) {
            if let Err(e) = self.client.delete_messages(chat, chunk).await {
                tracing::error!(
                    "❌ Telegram: Error deleting batch of {} messages: {}",
                    chunk.len(),
                    e
                );
            } else {
                tracing::info!(
                    "🗑️  Telegram: Successfully deleted batch of {} messages.",
                    chunk.len()
                );
            }
        }

        Ok(())
    }

    /// Forward a message to another chat (e.g. Shared Channel).
    /// This is a fast way to copy a file without download/upload.
    pub async fn forward_message_internal(
        &self,
        message_id: i64,
        target_chat_id: &str,
    ) -> Result<i64> {
        let from_chat = self.get_chat().await?;

        // Parse target chat
        let target_raw_id = parse_chat_id(target_chat_id)?;
        // Resolve target peer (assume channel if starts with -100)
        let target_peer = if target_chat_id.starts_with("-100") {
            let channel_id = target_raw_id.abs() - 1_000_000_000_000;
            PeerRef {
                id: PeerId::channel(channel_id).expect("Invalid Channel ID"),
                auth: PeerAuth::from_hash(0), // grammers sẽ tự resolve nếu cần hoặc lỗi nếu hash sai, nhưng forward thường cần peer chính xác
            }
        } else {
            PeerRef {
                id: PeerId::user(target_raw_id).expect("Invalid User ID"),
                auth: PeerAuth::from_hash(0),
            }
        };

        let message_id_i32 = i32::try_from(message_id)
            .map_err(|_| anyhow!("Telegram ID {message_id} exceeds i32 range"))?;

        let forwarded = self
            .client
            .forward_messages(target_peer, &[message_id_i32], from_chat)
            .await
            .context("Error forwarding Telegram message")?;

        let msg = forwarded
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| anyhow!("Forward failed, did not receive a new message."))?;

        Ok(i64::from(msg.id()))
    }
}

#[async_trait]
impl StorageProvider for TelegramClient {
    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            id: "telegram".to_string(),
            display_name: "Telegram Storage".to_string(),
            icon: "mdi-telegram".to_string(),
            description: "Large file storage via Telegram MTProto.".to_string(),
        }
    }

    fn fetch_capabilities(&self) -> Vec<ProviderCapability> {
        vec![
            ProviderCapability::ResumableUpload,
            ProviderCapability::Streaming,
        ]
    }

    async fn get_quota(&self) -> Result<ProviderQuota> {
        let used = if let Some(file_repo) = self.file_repo.get() {
            file_repo.get_platform_usage("telegram").await.unwrap_or(0) as u64
        } else {
            0
        };
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
        let (msg_id, _) = self
            .send_part_internal(data, part_idx as u32, "part", "Auto Upload", None)
            .await?;
        Ok(PartMetadata {
            id: 0,
            file_id,
            platform: self.metadata().id,
            message_id: msg_id.to_string(),
            part_index: part_idx as u32,
            size,
            checksum: None,
        })
    }

    async fn download_part(&self, part: &PartMetadata) -> Result<Vec<u8>> {
        let msg_id = part
            .message_id
            .parse()
            .map_err(|e| anyhow!("Invalid message ID: {}", e))?;
        self.download_part_internal(msg_id).await
    }

    async fn delete_part(&self, part: &PartMetadata) -> Result<()> {
        let msg_id = part
            .message_id
            .parse()
            .map_err(|e| anyhow!("Invalid message ID: {}", e))?;
        self.delete_message(msg_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_slice_range_bytes_clamps_to_data_len() {
        let data = b"0123456789";
        let sliced = slice_range_bytes(data, ByteRange { start: 7, len: 20 });
        assert_eq!(sliced, b"789");
    }
}
