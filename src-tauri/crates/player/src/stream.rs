use std::fmt;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::stream::Stream;

use crate::PlayerContext;

#[derive(Debug)]
pub(crate) enum StreamError {
    Network(String),
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Network(err) => write!(f, "Network error: {err}"),
        }
    }
}

impl std::error::Error for StreamError {}

pub(crate) type BoxByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>;

pub(crate) async fn stream_byte_range(
    st: PlayerContext,
    file_id: i64,
    start: u64,
    end: u64,
    stream_gen: u64,
    namespace: &str,
) -> Result<BoxByteStream, StreamError> {
    let len = end - start + 1;
    let rx = st.byte_stream_provider
        .stream_range(file_id, start, len, namespace)
        .await
        .map_err(|e| {
            tracing::error!("stream_range failed: file={} err={}", file_id, e);
            StreamError::Network(e)
        })?;

    let stream = futures_util::stream::unfold(
        (rx, file_id, stream_gen),
        |(mut rx, file_id, gen)| async move {
            loop {
                let current_gen = crate::bridge::RAW_STREAM_GENERATION
                    .get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
                    .lock()
                    .expect("Mutex poisoned")
                    .get(&file_id)
                    .copied();
                if current_gen != Some(gen) {
                    debug_log!("cancel", "stream_byte_range: superseded, file={} gen={} current={:?}", file_id, gen, current_gen);
                    return None;
                }
                match rx.recv().await {
                    Some(Ok(chunk)) => {
                        return Some((Ok(chunk.data), (rx, file_id, gen)));
                    }
                    Some(Err(e)) => {
                        return Some((Err(StreamError::Network(e)), (rx, file_id, gen)));
                    }
                    None => return None,
                }
            }
        },
    );

    Ok(Box::pin(stream) as BoxByteStream)
}

