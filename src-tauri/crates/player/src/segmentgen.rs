use crate::PlayerContext;
use crate::playlistbuild::ensure_video_playback_ready;

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
