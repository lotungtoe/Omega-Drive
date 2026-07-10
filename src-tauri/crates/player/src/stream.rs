use bytes::Bytes;
use tokio::sync::mpsc;

use crate::PlayerContext;
use crate::range_stream::{build_range_plan, receiver_stream, BoxByteStream, StreamError, RangePart};

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
    let abs_off = part.file_offset + part.slice_start;
    let slice_len = part.slice_len;
    if slice_len == 0 {
        return Ok(());
    }

    let mut rx = st.byte_stream_provider
        .stream_range(file_id, abs_off, slice_len, "video")
        .await
        .map_err(|e| {
            tracing::error!("stream_range failed: file={} err={}", file_id, e);
            StreamError::Network(e)
        })?;

    while let Some(chunk) = rx.recv().await {
        let chunk = chunk.map_err(StreamError::Network)?;
        if tx.send(Ok(chunk.data)).await.is_err() {
            return Err(StreamError::Canceled);
        }
    }
    Ok(())
}

