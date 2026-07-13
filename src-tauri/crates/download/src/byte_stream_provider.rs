use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::StatusCode;
use tokio::sync::mpsc;

use omega_drive_gateway::download::byte_stream_provider::{ByteStreamProvider, StreamChunk};
use omega_drive_gateway::provider::provider_types::ByteRange;
use omega_drive_gateway::provider::storage::PartMetadata;

use crate::provider::{http_client, resolve_cached_discord_url};
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
        let (tx, rx) = mpsc::channel(32);

        // Cache hit trước — 0 lock, 0 config
        if let Some(data) = self.ctx.mem_cache.read(file_id, offset, len).await {
            let _ = tx.send(Ok(StreamChunk { file_id, file_offset: offset, data })).await;
            return Ok(rx);
        }

        let ctx = self.ctx.clone();
        let ns = namespace.to_string();

        tokio::spawn(async move {
            if let Err(e) = stream_range_impl(ctx, file_id, offset, len, &ns, tx).await
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

fn lower_bound_part(sorted: &[u32], starts: &HashMap<u32, u64>, parts: &HashMap<u32, PartMetadata>, offset: u64) -> usize {
    let mut lo = 0usize;
    let mut hi = sorted.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        let part = sorted[mid];
        let end = starts[&part] + parts[&part].size.max(0) as u64;
        if end <= offset {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo.min(sorted.len() - 1)
}

async fn stream_range_impl(
    ctx: Arc<DownloadContext>,
    file_id: i64,
    offset: u64,
    len: u64,
    namespace: &str,
    tx: mpsc::Sender<Result<StreamChunk, String>>,
) -> Result<(), String> {
    let mut remaining = len;
    let mut cur_file_off = offset;

    let cached_parts = {
        let mut cache = ctx.parts_cache.lock().map_err(|e| e.to_string())?;
        cache.get_cloned(file_id)
    };
    let all_parts: HashMap<u32, PartMetadata> = match cached_parts {
        Some(parts) => parts.into_iter().map(|p| (p.part_index, p)).collect(),
        None => {
            let parts = ctx.file_repo.get_parts_for_file(file_id).await
                .map_err(|e| format!("DB error: {e}"))?;
            let map: HashMap<u32, PartMetadata> = parts.iter().map(|p| (p.part_index, p.clone())).collect();
            let mut cache = ctx.parts_cache.lock().map_err(|e| e.to_string())?;
            cache.insert(file_id, parts);
            map
        }
    };

    // Build cumulative byte offsets từ DB — không dùng config chunk_bytes
    let mut part_starts: HashMap<u32, u64> = HashMap::with_capacity(all_parts.len());
    let mut sorted_idx: Vec<u32> = all_parts.keys().copied().collect();
    sorted_idx.sort_unstable();
    if sorted_idx.is_empty() {
        tracing::error!("[stream_range] file={} has zero parts, cannot stream", file_id);
        return Err(format!("File {file_id} has no parts"));
    }

    let mut cumul = 0u64;
    for &idx in &sorted_idx {
        part_starts.insert(idx, cumul);
        cumul += all_parts[&idx].size.max(0) as u64;
    }

    let file_size = cumul;
    if offset >= file_size {
        tracing::error!(
            "[stream_range] offset {} beyond file size {} for file {}",
            offset, file_size, file_id
        );
        return Err(format!("offset {offset} beyond file size {file_size} for file {file_id}"));
    }
    let len = len.min(file_size.saturating_sub(offset));

    let first_idx = lower_bound_part(&sorted_idx, &part_starts, &all_parts, offset);
    let last_idx = lower_bound_part(&sorted_idx, &part_starts, &all_parts, offset + len - 1);
    let first_part = sorted_idx[first_idx];
    let last_part = sorted_idx[last_idx];

    for part_num_u64 in first_part as u64..=last_part as u64 {
        let part_num = part_num_u64 as u32;
        let part_start = part_starts[&part_num];
        let part_off = cur_file_off - part_start;

        let db_part = all_parts.get(&part_num)
            .ok_or_else(|| format!("Part {part_num} not found"))?;

        let part_size = db_part.size.max(0) as u64;
        if part_off >= part_size {
            tracing::warn!(
                "[stream_range] offset {} beyond part {} (size {}), file={}, truncating",
                cur_file_off, part_num, part_size, file_id
            );
            break;
        }
        let fetch_len = if part_num_u64 == first_part as u64 && part_num_u64 == last_part as u64 {
            remaining.min(part_size - part_off)
        } else if part_num_u64 == first_part as u64 {
            part_size - part_off
        } else if part_num_u64 == last_part as u64 {
            remaining
        } else {
            part_size
        };

        // Cache hit — forward cached data directly
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

        // Cache miss — download, write to cache, and forward data in one pass
        let part_meta = db_part.clone();

        if part_meta.platform == "discord" {
            download_and_forward_discord(
                &ctx, file_id, &part_meta,
                part_start, cur_file_off, fetch_len, namespace, &tx,
            ).await?;
        } else if part_meta.platform == "telegram" {
            download_and_forward_telegram(
                &ctx, file_id, &part_meta,
                part_start, cur_file_off, fetch_len, namespace, &tx,
            ).await?;
        } else {
            return Err(format!("Unsupported platform: {}", part_meta.platform));
        }

        cur_file_off += fetch_len;
        remaining -= fetch_len;
    }

    if remaining > 0 {
        tracing::warn!(
            "[stream_range] truncated: file={} still needs {} bytes after exhausting all parts",
            file_id, remaining
        );
    }

    Ok(())
}

async fn download_and_forward_discord(
    ctx: &DownloadContext,
    file_id: i64,
    part: &PartMetadata,
    part_start: u64,
    request_off: u64,
    request_len: u64,
    namespace: &str,
    tx: &mpsc::Sender<Result<StreamChunk, String>>,
) -> Result<(), String> {
    let mut url = resolve_cached_discord_url(ctx, file_id, part).await?;
    let mut retry = 0;
    loop {
        let client = http_client();
        let skip = request_off.saturating_sub(part_start);
        let full_part = request_len >= part.size.max(0) as u64;
        let req = client.get(&url);
        let res = if full_part {
            req.send().await
        } else {
            req.header("Range", format!("bytes={}-{}", skip, skip + request_len - 1))
                .send().await
        }.map_err(|e| e.to_string())?;
        let status = res.status();

        if status == StatusCode::FORBIDDEN {
            if retry >= 2 { return Err("HTTP_403".to_string()); }
            tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
            url = resolve_cached_discord_url(ctx, file_id, part).await?;
            retry += 1;
            continue;
        }
        if !status.is_success() {
            if retry >= 2 { return Err(format!("HTTP_STATUS_{}", status.as_u16())); }
            tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
            retry += 1;
            continue;
        }

        let mut pos: u64 = 0;
        let mut stream = res.bytes_stream();

        let request_end = request_off + request_len;
        let part_end = part_start + part.size.max(0) as u64;
        let max_forward = request_end.min(part_end).saturating_sub(request_off.max(part_start));
        let mut forwarded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            let chunk_bytes = Bytes::from(chunk);
            let chunk_len = chunk_bytes.len() as u64;
            let file_off = request_off + pos;

            // Always write full chunk to cache
            ctx.mem_cache
                .write(file_id, file_off, chunk_bytes.clone(), namespace)
                .await;

            // Forward only the requested sub-range
            if forwarded < max_forward {
                let available = (chunk_len as usize).min((max_forward - forwarded) as usize);
                if available > 0 {
                    let fwd = chunk_bytes.slice(..available);
                    forwarded += available as u64;
                    if tx.send(Ok(StreamChunk {
                        file_id,
                        file_offset: file_off,
                        data: fwd,
                    })).await.is_err() { return Ok(()); }
                }
            }

            pos += chunk_len;
        }
        return Ok(());
    }
}

async fn download_and_forward_telegram(
    ctx: &DownloadContext,
    file_id: i64,
    part: &PartMetadata,
    part_start: u64,
    request_off: u64,
    request_len: u64,
    namespace: &str,
    tx: &mpsc::Sender<Result<StreamChunk, String>>,
) -> Result<(), String> {
    let gateway = ctx
        .provider_runtime
        .stream_registry
        .get("telegram")
        .ok_or_else(|| "Telegram gateway unavailable".to_string())?;

    let skip = request_off.saturating_sub(part_start);
    let request_end = request_off + request_len;
    let part_end = part_start + part.size.max(0) as u64;
    let max_forward = request_end.min(part_end).saturating_sub(request_off.max(part_start));
    let range = ByteRange { start: skip, len: request_len };

    let mut retry = 0;
    loop {
        let mut stream = match gateway.download_part_range_stream(part, Some(range.clone())).await {
            Ok(s) => s,
            Err(e) => {
                if retry >= 2 { return Err(e.to_string()); }
                tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
                retry += 1;
                continue;
            }
        };

        let mut pos: u64 = 0;
        let mut forwarded: u64 = 0;
        let mut err: Option<String> = None;

        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    if retry >= 2 { return Err(e.to_string()); }
                    tokio::time::sleep(Duration::from_millis(500 + retry as u64 * 500)).await;
                    retry += 1;
                    err = Some("stream error".to_string());
                    break;
                }
            };
            let chunk_len = chunk.len() as u64;

            // Always write full chunk to cache (response starts at request_off)
            ctx.mem_cache
                .write(file_id, request_off + pos, chunk.clone(), namespace)
                .await;

            // Forward only the requested sub-range
            if forwarded < max_forward {
                let available = (chunk_len as usize).min((max_forward - forwarded) as usize);
                if available > 0 {
                    let fwd = chunk.slice(..available);
                    forwarded += available as u64;
                    if tx.send(Ok(StreamChunk {
                        file_id,
                        file_offset: request_off + pos,
                        data: fwd,
                    })).await.is_err() { return Ok(()); }
                }
            }

            pos += chunk_len;
        }

        if err.is_none() { return Ok(()); }
    }
}

#[cfg(test)]
#[path = "byte_stream_provider_test.rs"]
mod byte_stream_provider_tests;
