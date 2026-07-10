use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::StatusCode;
use tokio::sync::mpsc;

use omega_drive_gateway::download::byte_stream_provider::{ByteStreamProvider, StreamChunk};
use omega_drive_gateway::provider::storage::PartMetadata;

use crate::provider::{http_client, resolve_cached_discord_url, BandwidthTracker};
use crate::DownloadContext;

pub struct DownloadByteStreamProvider {
    ctx: Arc<DownloadContext>,
}

impl DownloadByteStreamProvider {
    pub fn new(ctx: Arc<DownloadContext>) -> Self {
        Self { ctx }
    }
}

#[async_trait::async_trait]
impl ByteStreamProvider for DownloadByteStreamProvider {
    async fn stream_range(
        &self,
        file_id: i64,
        offset: u64,
        len: u64,
        namespace: &str,
    ) -> Result<mpsc::Receiver<Result<StreamChunk, String>>, String> {
        let cfg = self
            .ctx
            .cfg
            .read()
            .map_err(|e| format!("Config lock error: {e}"))?;
        let chunk_bytes = cfg.general.chunk_bytes;
        drop(cfg);

        let (tx, rx) = mpsc::channel(32);
        let ctx = self.ctx.clone();
        let ns = namespace.to_string();

        tokio::spawn(async move {
            if let Err(e) = stream_range_impl(ctx, file_id, offset, len, &ns, chunk_bytes, tx).await
            {
                tracing::error!(
                    "[ByteStreamProvider] stream_range failed: file={} err={}",
                    file_id,
                    e
                );
            }
        });

        Ok(rx)
    }
}

async fn stream_range_impl(
    ctx: Arc<DownloadContext>,
    file_id: i64,
    offset: u64,
    len: u64,
    namespace: &str,
    chunk_bytes: u64,
    tx: mpsc::Sender<Result<StreamChunk, String>>,
) -> Result<(), String> {
    let first_part = offset / chunk_bytes + 1;
    let end_offset = offset + len;
    let last_part = (end_offset - 1) / chunk_bytes + 1;

    let mut remaining = len;
    let mut cur_file_off = offset;

    for part_num_u64 in first_part..=last_part {
        let part_num = part_num_u64 as u32;
        let part_start = (part_num_u64 - 1) * chunk_bytes;
        let part_off = cur_file_off - part_start;

        let db_part = ctx
            .file_repo
            .get_part_by_index(file_id, part_num)
            .await
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| format!("Part {part_num} not found"))?;

        let part_size = db_part.size.max(0) as u64;
        let fetch_len = remaining.min(part_size - part_off);

        if let Some(data) = ctx.mem_cache.read(file_id, cur_file_off, fetch_len).await {
            let _ = tx
                .send(Ok(StreamChunk {
                    file_id,
                    file_offset: cur_file_off,
                    data,
                }))
                .await;
            cur_file_off += fetch_len;
            remaining -= fetch_len;
            continue;
        }

        let part_meta = PartMetadata {
            id: db_part.id,
            file_id,
            platform: db_part.platform.clone(),
            message_id: db_part.message_id.clone(),
            attachment_name: db_part.attachment_name.clone(),
            part_index: db_part.part_index,
            size: db_part.size,
            part_type: db_part.part_type.clone(),
            duration: db_part.duration,
            checksum: db_part.checksum.clone(),
        };

        if part_meta.platform == "discord" {
            download_discord_part(&ctx, file_id, part_num, &part_meta, part_start, namespace)
                .await?;
        } else if part_meta.platform == "telegram" {
            download_telegram_part(&ctx, file_id, &part_meta, part_start, namespace).await?;
        } else {
            return Err(format!("Unsupported platform: {}", part_meta.platform));
        }

        let data = ctx
            .mem_cache
            .read(file_id, cur_file_off, fetch_len)
            .await
            .ok_or_else(|| format!("Data not cached after download part {part_num}"))?;
        let _ = tx
            .send(Ok(StreamChunk {
                file_id,
                file_offset: cur_file_off,
                data,
            }))
            .await;

        cur_file_off += fetch_len;
        remaining -= fetch_len;
    }

    Ok(())
}

async fn download_discord_part(
    ctx: &DownloadContext,
    file_id: i64,
    part_num: u32,
    part: &PartMetadata,
    file_offset: u64,
    namespace: &str,
) -> Result<(), String> {
    let mut url = resolve_cached_discord_url(ctx, file_id, part_num, part).await?;
    let mut retry = 0;
    loop {
        let client = http_client();
        let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
        let status = res.status();

        if status == StatusCode::FORBIDDEN {
            if retry >= 2 {
                return Err("HTTP_403".to_string());
            }
            tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
            url = resolve_cached_discord_url(ctx, file_id, part_num, part).await?;
            retry += 1;
            continue;
        }
        if !status.is_success() {
            if retry >= 2 {
                return Err(format!("HTTP_STATUS_{}", status.as_u16()));
            }
            tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
            retry += 1;
            continue;
        }

        let bw_start = Instant::now();
        let mut bw = BandwidthTracker::new();
        let mut pos: u64 = 0;
        let mut stream = res.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            let len = chunk.len();
            let data = Bytes::from(chunk);
            bw.record(len, bw_start.elapsed());
            ctx.mem_cache
                .write(file_id, file_offset + pos, data, namespace)
                .await;
            pos += len as u64;
            if bw_start.elapsed() > Duration::from_secs(10) {
                return Err("SLOW_DOWNLOAD".to_string());
            }
        }
        return Ok(());
    }
}

async fn download_telegram_part(
    ctx: &DownloadContext,
    file_id: i64,
    part: &PartMetadata,
    file_offset: u64,
    namespace: &str,
) -> Result<(), String> {
    let gateway = ctx
        .provider_runtime
        .stream_registry
        .get("telegram")
        .ok_or_else(|| "Telegram gateway unavailable".to_string())?;

    let mut retry = 0;
    loop {
        let mut stream = match gateway.download_part_range_stream(part, None).await {
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

        let bw_start = Instant::now();
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
            bw.record(len, bw_start.elapsed());
            ctx.mem_cache
                .write(file_id, file_offset + pos, chunk, namespace)
                .await;
            pos += len as u64;
        }

        if err.is_none() {
            return Ok(());
        }
    }
}
