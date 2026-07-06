use bytes::Bytes;
use tokio::sync::mpsc;
use futures_util::StreamExt;

use crate::PlayerContext;
use crate::range_stream::{build_range_plan, receiver_stream, BoxByteStream, StreamError, RangePart};
use crate::download::load_chunk_meta;
use crate::sparse::SparseCache;

pub(crate) async fn stream_byte_range(
    st: PlayerContext,
    file_id: i64,
    start: u64,
    end: u64,
    stream_gen: u64,
) -> Result<BoxByteStream, StreamError> {
    let sizes = {
        let all = st.file_repo.get_parts_for_file(file_id)
            .await
            .map_err(|e| StreamError::Network(format!("DB: {e}")))?;
        let mut seen = std::collections::HashSet::new();
        all.into_iter()
            .filter(|p| seen.insert(p.part_index))
            .map(|p| (p.part_index, p.size as u64))
            .collect::<Vec<_>>()
    };
    let plan = build_range_plan(&sizes, start, end);
    let (tx, rx) = mpsc::channel::<Result<Bytes, StreamError>>(32);

    tracing::info!("stream_byte_range: file={} range={}-{} parts={}", file_id, start, end, plan.parts.len());
    debug_log!("seek", "stream_byte_range: file={} range={}-{} parts={}", file_id, start, end, plan.parts.len());

    let total_parts = plan.parts.len();
    tokio::spawn(async move {
        debug_log!("trace", "stream_worker start: file={} parts={} gen={}", file_id, total_parts, stream_gen);
        for (i, part) in plan.parts.into_iter().enumerate() {
            debug_log!("trace", "stream_worker iter: file={} i={}/{} part={} gen={}", file_id, i, total_parts, part.part_index, stream_gen);
            if tx.is_closed() {
                debug_log!("cancel", "stream_byte_range: client dropped, file={} part={}", file_id, part.part_index);
                break;
            }
            let current_gen = crate::bridge::RAW_STREAM_GENERATION
                .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                .lock()
                .expect("Mutex poisoned")
                .get(&file_id)
                .copied();
            if current_gen != Some(stream_gen) {
                debug_log!("cancel", "stream_byte_range: superseded by newer request, file={} part={} gen={} current={:?}",
                    file_id, part.part_index, stream_gen, current_gen);
                break;
            }

            if let Err(err) = emit_part_bytes(&st, file_id, part, &tx).await {
                debug_log!("seek", "part failed: file={} part={} err={}", file_id, part.part_index, err);
                let _ = tx.send(Err(err)).await;
                break;
            }
            debug_log!("trace", "stream_worker part_done: file={} i={}/{} part={} gen={}", file_id, i+1, total_parts, part.part_index, stream_gen);
        }
        debug_log!("trace", "stream_worker done: file={} total={} gen={}", file_id, total_parts, stream_gen);
    });

    Ok(receiver_stream(rx))
}

async fn emit_part_bytes(
    st: &PlayerContext,
    file_id: i64,
    part: RangePart,
    tx: &mpsc::Sender<Result<Bytes, StreamError>>,
) -> Result<(), StreamError> {
    let part_num = part.part_index;
    let meta = load_chunk_meta(st, file_id, part_num)
        .await
        .map_err(StreamError::Network)?;

    let chunk_len = meta.size;
    if chunk_len == 0 {
        return Ok(());
    }

    let slice_start = part.slice_start.min(chunk_len);
    let slice_end = (slice_start + part.slice_len).min(chunk_len);
    if slice_end <= slice_start {
        return Ok(());
    }
    let slice_len = slice_end - slice_start;
    let sparse = &st.player_runtime.sparse_cache;
    let file_off = part.file_offset;

    // Slide pin window so eviction doesn't remove data the bridge is reading
    let center_off = file_off + slice_start + slice_len / 2;
    sparse.set_pin_window(file_id, center_off, chunk_len * 2, chunk_len * 5);

    // Cache-first: check sparse cache
    if sparse.is_range_filled(file_id, file_off + slice_start, slice_len) {
        debug_log!("cache", "hit: file={} part={} offset={}+{}", file_id, part_num, file_off + slice_start, slice_len);
        stream_from_sparse(sparse, file_id, file_off + slice_start, slice_len, tx).await?;
        return Ok(());
    }

    if slice_start > 0 {
        // SEEK PATH: bypass writes to sparse cache AND streams to tx (hybrid)
        let t3_start = std::time::Instant::now();
        try_seek_range(st, file_id, part_num, &meta, file_off, slice_start, slice_len, sparse, tx)
            .await
            .map_err(StreamError::Network)?;
        debug_log!("t3", "file={} part={} t3={}µs", file_id, part_num, t3_start.elapsed().as_micros());
        return Ok(());
    }

    // SEQUENTIAL PATH: spawn coordinator → writes to sparse cache
    debug_log!("trace", "coordinator spawned: file={} part={}", file_id, part_num);
    let st_clone = st.clone();
    tokio::spawn(async move {
        crate::segmentgen::coordinator_download_part(st_clone, file_id, part_num, file_off, chunk_len).await;
    });

    stream_from_sparse(sparse, file_id, file_off + slice_start, slice_len, tx).await
}

async fn stream_from_sparse(
    sparse: &SparseCache,
    file_id: i64,
    start_off: u64,
    len: u64,
    tx: &mpsc::Sender<Result<Bytes, StreamError>>,
) -> Result<(), StreamError> {
    let mut cur = start_off;
    let end = start_off + len;
    while cur < end {
        let read_len = std::cmp::min(65536u64, end - cur);
        let data = sparse.wait_range(file_id, cur, read_len)
            .await
            .map_err(StreamError::Network)?;
        if tx.send(Ok(data)).await.is_err() {
            return Err(StreamError::Canceled);
        }
        cur += read_len;
    }
    Ok(())
}

async fn try_seek_range(
    st: &PlayerContext,
    file_id: i64,
    part_num: u32,
    meta: &crate::download::ChunkMeta,
    file_offset: u64,
    slice_start: u64,
    slice_len: u64,
    sparse_cache: &SparseCache,
    tx: &mpsc::Sender<Result<Bytes, StreamError>>,
) -> Result<(), String> {
    let t_start = std::time::Instant::now();
    debug_log!("seek", "try_seek_range start: file={} part={} slice={}+{}", file_id, part_num, slice_start, slice_len);
    let result = {
        let part_meta = crate::download::fetch_part_metadata(meta, file_id, part_num);
        if part_meta.platform == "discord" {
            try_seek_range_discord(st, file_id, part_num, &part_meta, file_offset, slice_start, slice_len, sparse_cache, tx).await
        } else if part_meta.platform == "telegram" {
            try_seek_range_telegram(st, file_id, part_num, &part_meta, file_offset, slice_start, slice_len, sparse_cache, tx).await
        } else {
            Err("Unsupported platform".to_string())
        }
    };
    if result.is_ok() {
        debug_log!("seek", "try_seek_range ok: file={} part={} elapsed={:?}", file_id, part_num, t_start.elapsed());
    } else {
        debug_log!("seek", "try_seek_range fail: file={} part={} elapsed={:?}", file_id, part_num, t_start.elapsed());
    }
    result
}

async fn try_seek_range_discord(
    st: &PlayerContext,
    file_id: i64,
    part_num: u32,
    part: &omega_drive_gateway::provider::storage::PartMetadata,
    file_offset: u64,
    slice_start: u64,
    slice_len: u64,
    sparse_cache: &SparseCache,
    tx: &mpsc::Sender<Result<Bytes, StreamError>>,
) -> Result<(), String> {
    let url = crate::segmentgen::resolve_cached_discord_url(st, file_id, part_num, part).await?;
    if let Err(e) = try_seek_range_discord_stream(&url, file_id, part_num, file_offset, slice_start, slice_len, sparse_cache, tx).await {
        if e == "HTTP_403" {
            let fresh = crate::segmentgen::resolve_cached_discord_url(st, file_id, part_num, part).await?;
            try_seek_range_discord_stream(&fresh, file_id, part_num, file_offset, slice_start, slice_len, sparse_cache, tx).await?;
            return Ok(());
        }
        return Err(e);
    }
    Ok(())
}

async fn try_seek_range_telegram(
    st: &PlayerContext,
    file_id: i64,
    part_num: u32,
    part: &omega_drive_gateway::provider::storage::PartMetadata,
    file_offset: u64,
    slice_start: u64,
    slice_len: u64,
    sparse_cache: &SparseCache,
    tx: &mpsc::Sender<Result<Bytes, StreamError>>,
) -> Result<(), String> {
    debug_log!("seek", "try_seek_range_telegram start: file={} part={} slice={}+{}", file_id, part_num, slice_start, slice_len);
    use omega_drive_gateway::provider::provider_types::ByteRange;
    let gateway = st.stream_registry
        .get("telegram")
        .ok_or_else(|| "Telegram chua duoc cau hinh".to_string())?;
    let bw_start = std::time::Instant::now();
    let mut bw = crate::download::BandwidthTracker::new();
    let range = ByteRange { start: slice_start, len: slice_len };
    let mut retry = 0;
    let result = loop {
        let mut stream = match gateway.download_part_range_stream(part, Some(range.clone())).await {
            Ok(s) => s,
            Err(e) => {
                if retry >= 2 {
                    break Err(e.to_string());
                }
                tokio::time::sleep(std::time::Duration::from_millis(500 + retry as u64 * 500)).await;
                retry += 1;
                continue;
            }
        };
        let mut pos: u64 = 0;
        let mut ok = true;
        let mut err: Option<String> = None;
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    if retry >= 2 {
                        err = Some(e.to_string());
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500 + retry as u64 * 500)).await;
                    retry += 1;
                    ok = false;
                    break;
                }
            };
            let len = chunk.len();
            let data = Bytes::from(chunk);
            bw.record(len, bw_start.elapsed());
            sparse_cache.write(file_id, file_offset + slice_start + pos, data.clone()).await;
            if tx.send(Ok(data)).await.is_err() {
                err = Some("client dropped".to_string());
                break;
            }
            pos += len as u64;
        }
        if let Some(e) = err {
            break Err(e);
        }
        if ok {
            break Ok(());
        }
    };
    match &result {
        Ok(()) => {
            let s = bw.finalize();
            debug_log!("cdn_bw", "file={} part={} telegram bypass avg={:.1}MB/s min={:.1}MB/s max={:.1}MB/s size={} elapsed={:?}",
                file_id, part_num, s.avg_mbps, s.min_mbps, s.max_mbps, s.total_bytes, bw_start.elapsed());
        }
        Err(_) => {}
    }
    result
}

async fn try_seek_range_discord_stream(
    url: &str,
    file_id: i64,
    part_num: u32,
    file_offset: u64,
    slice_start: u64,
    slice_len: u64,
    sparse_cache: &SparseCache,
    tx: &mpsc::Sender<Result<Bytes, StreamError>>,
) -> Result<(), String> {
    if let Some(t1) = crate::bridge::take_t1_mark(file_id) {
        debug_log!("t1", "file={} t1={}µs", file_id, t1.as_micros());
    }
    let client = crate::download::http_client();
    let range = format!("bytes={}-{}", slice_start, slice_start + slice_len - 1);
    let t_req = std::time::Instant::now();
    let res = client
        .get(url)
        .header(reqwest::header::RANGE, range)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    let ttfb = t_req.elapsed();
    debug_log!("cdn_bw", "file={} part={} discord bypass ttfb={:?} status={} (range {}-{})",
        file_id, part_num, ttfb, status, slice_start, slice_start + slice_len - 1);
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("HTTP_403".to_string());
    }
    if !status.is_success() {
        return Err(format!("HTTP_STATUS_{}", status.as_u16()));
    }
    let mut stream = res.bytes_stream();
    let bw_start = std::time::Instant::now();
    let mut bw = crate::download::BandwidthTracker::new();
    let mut pos: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        let len = chunk.len();
        let data = Bytes::from(chunk);
        bw.record(len, bw_start.elapsed());
        sparse_cache.write(file_id, file_offset + slice_start + pos, data.clone()).await;
        if tx.send(Ok(data)).await.is_err() {
            return Err("client dropped".to_string());
        }
        pos += len as u64;
    }
    let s = bw.finalize();
    debug_log!("cdn_bw", "file={} part={} discord bypass avg={:.1}MB/s min={:.1}MB/s max={:.1}MB/s size={} elapsed={:?}",
        file_id, part_num, s.avg_mbps, s.min_mbps, s.max_mbps, s.total_bytes, bw_start.elapsed());
    Ok(())
}

