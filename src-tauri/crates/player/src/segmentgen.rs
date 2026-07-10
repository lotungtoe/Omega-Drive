use std::sync::Arc;
use crate::PlayerContext;
use crate::playlistbuild::ensure_video_playback_ready;

/// Get data for a part of a video file.
pub async fn get_file_part_internal(
    st: &PlayerContext,
    file_id: i64,
    part_num: u32,
) -> Result<Vec<u8>, String> {
    let start_time = std::time::Instant::now();
    let _ = ensure_video_playback_ready(&st, file_id).await?;
    if part_num == 0 {
        let _ = st.file_repo.mark_file_accessed(file_id).await;
    }

    let chunk_bytes = st.cfg.general.chunk_bytes;
    let part_offset = (part_num as u64 - 1) * chunk_bytes;
    let part = st.file_repo.get_part_by_index(file_id, part_num)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| format!("Part {part_num} not found"))?;
    let part_size = part.size.max(0) as u64;

    let mut rx = st.byte_stream_provider
        .stream_range(file_id, part_offset, part_size, "video")
        .await?;

    let mut raw_data = Vec::with_capacity(part_size as usize);
    while let Some(chunk) = rx.recv().await {
        let chunk = chunk?;
        raw_data.extend_from_slice(&chunk.data);
    }

    tracing::debug!(
        "Stream: downloaded part {} ({}ms)",
        part.id,
        start_time.elapsed().as_millis()
    );

    Ok(raw_data)
}


/// Removes a key from in_flight_coordinator when the download scope exits.
#[allow(dead_code)]
struct CoordGuard {
    key: (i64, u32),
    set: Arc<tokio::sync::Mutex<std::collections::HashSet<(i64, u32)>>>,
}
impl Drop for CoordGuard {
    fn drop(&mut self) {
        let set: Arc<tokio::sync::Mutex<std::collections::HashSet<(i64, u32)>>> = self.set.clone();
        let key = self.key;
        tokio::spawn(async move {
            set.lock().await.remove(&key);
        });
    }
}

#[allow(dead_code)]
async fn coordinator_download_part_inner(
    st: PlayerContext,
    file_id: i64,
    part_num: u32,
    file_offset: u64,
    part_size: u64,
) {
    let key = (file_id, part_num);
    {
        let mut in_flight = st.player_runtime.in_flight_coordinator.lock().await;
        if !in_flight.insert(key) {
            debug_log!("coord", "skip: file={} part={} already in-flight", file_id, part_num);
            return;
        }
    }
    let _guard = CoordGuard {
        key,
        set: st.player_runtime.in_flight_coordinator.clone(),
    };

    debug_log!("coord", "start: file={} part={} offset={}+{}", file_id, part_num, file_offset, part_size);
    let _permit = match st.player_runtime.download_semaphore.clone().acquire_owned().await {
        Ok(p) => p,
        Err(_) => return,
    };
    let coord_start = std::time::Instant::now();
    let _ = ensure_video_playback_ready(&st, file_id).await;
    debug_log!("coord", "timing: file={} part={} ensure_playback={:?}", file_id, part_num, coord_start.elapsed());

    let mut rx = match st.byte_stream_provider
        .stream_range(file_id, file_offset, part_size, "video")
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            debug_log!("coord", "fail: file={} part={} err={}", file_id, part_num, e);
            return;
        }
    };
    while let Some(result) = rx.recv().await {
        if let Err(e) = result {
            debug_log!("coord", "fail: file={} part={} err={}", file_id, part_num, e);
            return;
        }
    }
    debug_log!("coord", "done: file={} part={} elapsed={:?}", file_id, part_num, coord_start.elapsed());
}

#[allow(dead_code)]
pub(crate) async fn coordinator_download_part(
    st: PlayerContext,
    file_id: i64,
    part_num: u32,
    file_offset: u64,
    part_size: u64,
) {
    coordinator_download_part_inner(st.clone(), file_id, part_num, file_offset, part_size).await;
    schedule_ahead_downloads(st, file_id, part_num).await;
}

/// After coordinator finishes part `from_part`, pre-buffer next parts.
#[allow(dead_code)]
async fn schedule_ahead_downloads(st: PlayerContext, file_id: i64, from_part: u32) {
    let prefetch_ahead = st.player_runtime.prefetch_ahead;
    if prefetch_ahead == 0 {
        return;
    }

    let all_parts = match st.file_repo.get_parts_for_file_by_type(file_id, "chunk").await {
        Ok(p) => p,
        Err(_) => return,
    };
    if all_parts.is_empty() {
        return;
    }

    let mut sorted: Vec<(u32, u64)> = all_parts
        .iter()
        .map(|p| (p.part_index, p.size.max(0) as u64))
        .collect();
    sorted.sort_by_key(|(idx, _)| *idx);

    let mut offsets: std::collections::HashMap<u32, u64> = std::collections::HashMap::new();
    let mut acc: u64 = 0;
    for (idx, size) in &sorted {
        offsets.insert(*idx, acc);
        acc += size;
    }

    for i in 1..=prefetch_ahead {
        let next_part = from_part + i;
        let part_size = match sorted.iter().find(|(idx, _)| *idx == next_part) {
            Some((_, s)) => *s,
            None => break,
        };
        if part_size == 0 {
            continue;
        }
        let file_offset = match offsets.get(&next_part) {
            Some(o) => *o,
            None => break,
        };

        debug_log!("prefetch", "scheduling: file={} part={} offset={}+{}",
            file_id, next_part, file_offset, part_size);

        let st_clone = st.clone();
        tokio::spawn(async move {
            coordinator_download_part_inner(st_clone, file_id, next_part, file_offset, part_size).await;
        });
    }
}
