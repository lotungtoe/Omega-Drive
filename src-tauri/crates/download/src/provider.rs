use std::sync::OnceLock;
use std::time::{Duration, Instant};

use bytes::Bytes;
use chrono::Utc;
use futures_util::StreamExt;
use reqwest::StatusCode;

use omega_drive_gateway::player::cache::ByteCache;
use omega_drive_gateway::provider::provider_types::MediaSource;
use omega_drive_gateway::provider::storage::PartMetadata;

use crate::DownloadContext;

// ============================================================
// Bandwidth Tracker
// ============================================================

pub struct BwStats {
    pub avg_mbps: f64,
    pub min_mbps: f64,
    pub max_mbps: f64,
    pub total_bytes: u64,
}

pub struct BandwidthTracker {
    total_bytes: u64,
    prev_time: Duration,
    last_elapsed: Duration,
    min_rate: f64,
    max_rate: f64,
    initialized: bool,
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            prev_time: Duration::ZERO,
            last_elapsed: Duration::ZERO,
            min_rate: 0.0,
            max_rate: 0.0,
            initialized: false,
        }
    }

    pub fn record(&mut self, bytes: usize, elapsed: Duration) {
        self.total_bytes += bytes as u64;
        self.last_elapsed = elapsed;
        let chunk_time = elapsed.saturating_sub(self.prev_time);
        if chunk_time >= Duration::from_millis(1) {
            let rate = bytes as f64 / chunk_time.as_secs_f64() / 1024.0 / 1024.0;
            if self.initialized {
                self.min_rate = self.min_rate.min(rate);
                self.max_rate = self.max_rate.max(rate);
            } else {
                self.min_rate = rate;
                self.max_rate = rate;
                self.initialized = true;
            }
        }
        self.prev_time = elapsed;
    }

    pub fn finalize(&self) -> BwStats {
        let avg_mbps = if self.last_elapsed > Duration::from_micros(1) && self.total_bytes > 0 {
            self.total_bytes as f64 / self.last_elapsed.as_secs_f64() / 1024.0 / 1024.0
        } else {
            0.0
        };
        BwStats {
            avg_mbps,
            min_mbps: if self.initialized { self.min_rate } else { 0.0 },
            max_mbps: if self.initialized { self.max_rate } else { 0.0 },
            total_bytes: self.total_bytes,
        }
    }
}

// ============================================================
// HTTP Client
// ============================================================

pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(0)
            .pool_idle_timeout(Duration::from_secs(30))
            .http1_only()
            .build()
            .expect("Failed to build reqwest client")
    })
}

// ============================================================
// Download helpers
// ============================================================

pub async fn download_url(url: &str) -> Result<Vec<u8>, String> {
    let client = http_client();
    let res = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    if status == StatusCode::FORBIDDEN {
        return Err("HTTP_403".to_string());
    }
    if !status.is_success() {
        return Err(format!("HTTP_STATUS_{}", status.as_u16()));
    }
    res.bytes()
        .await
        .map_err(|e| e.to_string())
        .map(|b| b.to_vec())
}

pub async fn download_url_stream(
    url: &str,
    file_id: i64,
    _part_num: u32,
    file_offset: u64,
    sparse_cache: &dyn ByteCache,
) -> Result<(), String> {
    let client = http_client();
    let t_req = Instant::now();
    let res = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    let _ttfb = t_req.elapsed();
    if status == StatusCode::FORBIDDEN {
        return Err("HTTP_403".to_string());
    }
    if !status.is_success() {
        return Err(format!("HTTP_STATUS_{}", status.as_u16()));
    }
    let mut stream = res.bytes_stream();
    let bw_start = Instant::now();
    let mut bw = BandwidthTracker::new();
    let mut pos: u64 = 0;
    let mut write_time = Duration::ZERO;
    let mut _chunk_count: u32 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        let len = chunk.len();
        let data = Bytes::from(chunk);
        bw.record(len, bw_start.elapsed());
        let w_start = Instant::now();
        sparse_cache.write(file_id, file_offset + pos, data).await;
        write_time += w_start.elapsed();
        _chunk_count += 1;
        pos += len as u64;
        if bw_start.elapsed() > Duration::from_secs(10) {
            return Err("SLOW_DOWNLOAD".to_string());
        }
    }
    let _s = bw.finalize();
    let _net_time = bw_start.elapsed().checked_sub(write_time).unwrap_or_default();
    Ok(())
}

// ============================================================
// Chunk metadata
// ============================================================

#[derive(Clone)]
pub struct ChunkMeta {
    pub part_id: i64,
    pub platform: String,
    pub message_id: String,
    pub attachment_name: Option<String>,
    pub part_type: String,
    pub size: u64,
}

pub fn fetch_part_metadata(meta: &ChunkMeta, file_id: i64, part_num: u32) -> PartMetadata {
    PartMetadata {
        id: meta.part_id,
        file_id,
        platform: meta.platform.clone(),
        message_id: meta.message_id.clone(),
        attachment_name: meta.attachment_name.clone(),
        part_index: part_num,
        size: meta.size as i64,
        part_type: meta.part_type.clone(),
        duration: None,
        checksum: None,
    }
}

pub async fn load_chunk_meta(
    state: &DownloadContext,
    file_id: i64,
    part_num: u32,
) -> Result<ChunkMeta, String> {
    let _file = state.file_repo.get_file_by_id(file_id)
        .await
        .map_err(|e| format!("Loi DB: {e}"))?
        .ok_or_else(|| "Khong tim thay file".to_string())?;
    let selected = state.file_repo.get_part_by_index(file_id, part_num)
        .await
        .map_err(|e| format!("DB chunk error: {e}"))?
        .ok_or_else(|| format!("Chunk {part_num} not found"))?;
    let meta = ChunkMeta {
        part_id: selected.id,
        platform: selected.platform.clone(),
        message_id: selected.message_id,
        attachment_name: selected.attachment_name,
        part_type: selected.part_type,
        size: selected.size.max(0) as u64,
    };
    Ok(meta)
}

// ============================================================
// Discord URL resolution
// ============================================================

pub async fn resolve_cached_discord_url(
    state: &DownloadContext,
    file_id: i64,
    part_num: u32,
    part: &PartMetadata,
) -> Result<String, String> {
    let cache_key = format!("{}:{}", file_id, part_num);
    if let Some(url) = {
        let cache = state.cdn_link_cache.read().await;
        cache.get(&cache_key).and_then(|(u, e)| {
            if *e > Utc::now() {
                Some(u.clone())
            } else {
                None
            }
        })
    } {
        return Ok(url);
    }

    let gateway = state.provider_runtime.stream_registry
        .get("discord")
        .ok_or_else(|| "Discord stream gateway unavailable".to_string())?;
    let source = gateway
        .resolve_media_source(part)
        .await
        .map_err(|e| format!("Loi Discord: {e}"))?;
    match source {
        MediaSource::ResolvedUrl { url, expiry } => {
            if let Some(expiry) = expiry {
                let mut cache = state.cdn_link_cache.write().await;
                cache.insert(cache_key, (url.clone(), expiry));
            }
            Ok(url)
        }
        MediaSource::ProviderOwned => Err("Discord stream gateway did not return URL".to_string()),
    }
}

// ============================================================
// Download part from provider
// ============================================================

pub async fn download_part_from_provider(
    state: &DownloadContext,
    file_id: i64,
    part_num: u32,
    part: &PartMetadata,
) -> Result<Vec<u8>, String> {
    #[cfg(debug_assertions)]
    let t2_start = Instant::now();
    let result = download_part_from_provider_inner(state, file_id, part_num, part).await;
    #[cfg(debug_assertions)]
    if std::env::var("DEBUG").is_ok() {
        let t2_us = t2_start.elapsed().as_micros() as u64;
        tracing::info!(target: "latency", "T2_provider={}µs file={} part={} platform={}", t2_us, file_id, part_num, part.platform);
    }
    result
}

pub async fn download_part_from_provider_inner(
    state: &DownloadContext,
    file_id: i64,
    part_num: u32,
    part: &PartMetadata,
) -> Result<Vec<u8>, String> {
    if part.platform == "discord" {
        let url = resolve_cached_discord_url(state, file_id, part_num, part).await?;
        let mut res = download_url(&url).await;
        if matches!(res.as_ref().err(), Some(e) if e == "HTTP_403") {
            let fresh = resolve_cached_discord_url(state, file_id, part_num, part).await?;
            res = download_url(&fresh).await;
        }
        return res;
    }

    if part.platform == "telegram" {
        let gateway = state.provider_runtime.stream_registry
            .get("telegram")
            .ok_or_else(|| "Telegram chua duoc cau hinh".to_string())?;
        return gateway
            .download_part_bytes(part)
            .await
            .map_err(|e| e.to_string());
    }

    Err("Khong xac dinh duoc duong dan tai".to_string())
}

// ============================================================
// Download part stream (for coordinator)
// ============================================================

pub async fn download_part_stream(
    state: &DownloadContext,
    file_id: i64,
    part_num: u32,
    part: &PartMetadata,
    file_offset: u64,
    sparse_cache: &dyn ByteCache,
) -> Result<(), String> {
    if part.platform == "discord" {
        let mut url = resolve_cached_discord_url(state, file_id, part_num, part).await?;
        let mut retry = 0;
        loop {
            let res = download_url_stream(&url, file_id, part_num, file_offset, sparse_cache).await;
            if res.is_ok() || retry >= 2 {
                return res;
            }
            let is_403 = matches!(res.as_ref().err(), Some(e) if e == "HTTP_403");
            tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
            if is_403 {
                url = resolve_cached_discord_url(state, file_id, part_num, part).await?;
            }
            retry += 1;
        }
    }
    if part.platform == "telegram" {
        let gateway = state.provider_runtime.stream_registry
            .get("telegram")
            .ok_or_else(|| "Telegram chua duoc cau hinh".to_string())?;
        let bw_start = Instant::now();
        let mut retry = 0;
        loop {
            let mut stream = match gateway
                .download_part_range_stream(part, None)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    if retry >= 2 {
                        return Err(e.to_string());
                    }
                    tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
                    retry += 1;
                    continue;
                }
            };
            let mut bw = BandwidthTracker::new();
            let mut pos: u64 = 0;
            let mut err: Option<String> = None;
            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(e) => {
                        if retry >= 2 {
                            return Err(e.to_string());
                        }
                        tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
                        retry += 1;
                        err = Some("stream error".to_string());
                        break;
                    }
                };
                let len = chunk.len();
                let data = Bytes::from(chunk);
                bw.record(len, bw_start.elapsed());
                sparse_cache.write(file_id, file_offset + pos, data).await;
                pos += len as u64;
            }
            if err.is_none() {
                let _s = bw.finalize();
                return Ok(());
            }
        }
    }
    Err("Khong xac dinh duoc duong dan tai".to_string())
}
