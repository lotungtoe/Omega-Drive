use std::sync::Arc;
use bytes::Bytes;
use crate::PlayerContext;
use omega_drive_gateway::provider::storage::PartMetadata;
use crate::playlistbuild::ensure_video_playback_ready;
use crate::singleflight::PartKey;
use omega_drive_download::provider::{download_part_from_provider, download_part_stream};

/// Get data for a part of a video file.
/// Priority: 1. RAM Buffer, 2. SSD Cache, 3. Fetch from source (Discord/Telegram).
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

    let part = {
        let _file = st.file_repo.get_file_by_id(file_id)
            .await
            .map_err(|e| format!("DB error: {e}"))?
            .ok_or_else(|| "File not found".to_string())?;

        let chunk_parts = st.file_repo.get_parts_for_file_by_type(file_id, "chunk")
            .await
            .map_err(|e| format!("DB Chunks error: {e}"))?;
        let selected = chunk_parts
            .into_iter()
            .find(|p| p.part_index == part_num)
            .ok_or_else(|| format!("Part {part_num} not found"))?;

        PartMetadata {
            id: selected.id,
            file_id,
            platform: selected.platform,
            message_id: selected.message_id,
            attachment_name: selected.attachment_name,
            part_index: selected.part_index,
            size: selected.size,
            part_type: selected.part_type.clone(),
            duration: selected.duration,
            checksum: None,
        }
    };

    // 2. Fetch from source
    let part_id = part.id;
    let raw_data = download_part_from_provider(&st.download_ctx, file_id, part_num, &part)
        .await
        .map_err(|err| {
            tracing::error!(
                "[get_file_part_internal] Download FAILED: file_id={}, part={}, error={}",
                file_id,
                part_num,
                err
            );
            err
        })?;

    let processed_data = raw_data;


    tracing::debug!(
        "Stream: downloaded part {} ({}ms)",
        part_id,
        start_time.elapsed().as_millis()
    );

    Ok(processed_data)
}

pub(crate) async fn get_chunk_part_internal(
    st: PlayerContext,
    file_id: i64,
    part_num: u32,
) -> Result<Bytes, String> {
    let _ = ensure_video_playback_ready(&st, file_id).await?;

    let part = {
        let _file = st.file_repo.get_file_by_id(file_id)
            .await
            .map_err(|e| format!("L?i DB: {e}"))?
            .ok_or_else(|| "Kh?ng t?m th?y file".to_string())?;

        let chunk_parts = st.file_repo.get_parts_for_file_by_type(file_id, "chunk")
            .await
            .map_err(|e| format!("L?i DB Chunks: {e}"))?;
        let selected = chunk_parts
            .into_iter()
            .find(|p| p.part_index == part_num)
            .ok_or_else(|| format!("Kh?ng t?m th?y chunk {}", part_num))?;

        PartMetadata {
            id: selected.id,
            file_id,
            platform: selected.platform,
            message_id: selected.message_id,
            attachment_name: selected.attachment_name,
            part_index: selected.part_index,
            size: selected.size,
            part_type: selected.part_type.clone(),
            duration: selected.duration,
            checksum: None,
        }
    };


    // 2. Fetch from source (SingleFlight)
    let key: PartKey = (file_id, part_num, 0, 0);
    let st_clone = st.clone();
    st.player_runtime.part_singleflight
        .run(key, move || async move {
            let start_time = std::time::Instant::now();
            let part_id = part.id;
            let raw_data = download_part_from_provider(&st_clone.download_ctx, file_id, part_num, &part)
                .await
                .map_err(|err| {
                    tracing::error!(
                        "? [get_chunk_part_internal] Download FAILED: file_id={}, part={}, error={}",
                        file_id,
                        part_num,
                        err
                    );
                    err
                })?;

            let processed_data = raw_data;

            tracing::debug!(
                "Stream: downloaded chunk {} ({}ms)",
                part_id,
                start_time.elapsed().as_millis()
            );

            let bytes = Bytes::from(processed_data);
            st_clone.player_runtime.sparse_cache.write(file_id, 0, bytes.clone()).await;
            Ok(bytes)
        })
        .await
}
/// Removes a key from in_flight_coordinator when the download scope exits.
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
    if st.player_runtime.sparse_cache.is_range_filled(file_id, file_offset, part_size) {
        debug_log!("coord", "skip: file={} part={} already cached", file_id, part_num);
        return;
    }
    let _permit = match st.player_runtime.download_semaphore.clone().acquire_owned().await {
        Ok(p) => p,
        Err(_) => return,
    };
    // Re-check cache after semaphore (data may have been written while waiting)
    if st.player_runtime.sparse_cache.is_range_filled(file_id, file_offset, part_size) {
        debug_log!("coord", "skip: file={} part={} now cached (re-check)", file_id, part_num);
        return;
    }
    let coord_start = std::time::Instant::now();
    let _ = ensure_video_playback_ready(&st, file_id).await;
    debug_log!("coord", "timing: file={} part={} ensure_playback={:?}", file_id, part_num, coord_start.elapsed());
    let part = match lookup_part(&st, file_id, part_num).await {
        Some(p) => p,
        None => return,
    };
    debug_log!("coord", "timing: file={} part={} lookup_part={:?}", file_id, part_num, coord_start.elapsed());
    if let Err(e) = download_part_stream(&st.download_ctx, file_id, part_num, &part, file_offset, &*st.player_runtime.sparse_cache).await {
        debug_log!("coord", "fail: file={} part={} err={}", file_id, part_num, e);
        return;
    }
    debug_log!("coord", "done: file={} part={} elapsed={:?}", file_id, part_num, coord_start.elapsed());
}

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

async fn lookup_part(st: &PlayerContext, file_id: i64, part_num: u32) -> Option<PartMetadata> {
    let chunk_parts = match st.file_repo.get_parts_for_file_by_type(file_id, "chunk").await {
        Ok(p) => p,
        Err(e) => { debug_log!("coord", "fail: file={} part={} err={}", file_id, part_num, e); return None; }
    };
    let selected = chunk_parts.into_iter().find(|p| p.part_index == part_num)?;
    Some(PartMetadata {
        id: selected.id,
        file_id,
        platform: selected.platform,
        message_id: selected.message_id,
        attachment_name: selected.attachment_name,
        part_index: selected.part_index,
        size: selected.size,
        part_type: selected.part_type,
        duration: selected.duration,
        checksum: None,
    })
}

/// After coordinator finishes part `from_part`, pre-buffer next parts.
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

    let sparse = &st.player_runtime.sparse_cache;
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

        if sparse.is_range_filled(file_id, file_offset, part_size) {
            continue;
        }

        debug_log!("prefetch", "scheduling: file={} part={} offset={}+{}",
            file_id, next_part, file_offset, part_size);

        let st_clone = st.clone();
        tokio::spawn(async move {
            coordinator_download_part_inner(st_clone, file_id, next_part, file_offset, part_size).await;
        });
    }
}
