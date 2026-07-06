use std::fmt;
use std::pin::Pin;

use bytes::Bytes;
use futures_util::stream::Stream;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RangePart {
    pub part_index: u32,
    pub file_offset: u64,
    pub slice_start: u64,
    pub slice_len: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct RangePlan {
    pub parts: Vec<RangePart>,
}

pub(crate) fn build_range_plan(parts: &[(u32, u64)], start: u64, end: u64) -> RangePlan {
    let mut result = Vec::new();
    let mut offset = 0u64;
    for &(part_index, size) in parts {
        let part_end = offset + size;
        if offset <= end && part_end > start {
            let slice_start = start.saturating_sub(offset);
            let slice_end = (end + 1).saturating_sub(offset).min(size);
            if slice_end > slice_start {
                result.push(RangePart { part_index, file_offset: offset, slice_start, slice_len: slice_end - slice_start });
            }
        }
        offset = part_end;
        if offset > end { break; }
    }
    RangePlan { parts: result }
}

#[derive(Debug)]
pub(crate) enum StreamError {
    Io(std::io::Error),
    Network(String),
    Canceled,
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamError::Io(err) => write!(f, "IO error: {err}"),
            StreamError::Network(err) => write!(f, "Network error: {err}"),
            StreamError::Canceled => write!(f, "Stream canceled"),
        }
    }
}

impl std::error::Error for StreamError {}

impl From<std::io::Error> for StreamError {
    fn from(err: std::io::Error) -> Self {
        StreamError::Io(err)
    }
}

pub(crate) type BoxByteStream = Pin<Box<dyn Stream<Item = Result<Bytes, StreamError>> + Send>>;

pub(crate) fn receiver_stream(
    rx: tokio::sync::mpsc::Receiver<Result<Bytes, StreamError>>,
) -> BoxByteStream {
    Box::pin(futures_util::stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(item) => Some((item, rx)),
            None => None,
        }
    }))
}

// map_stream_error removed (unused)
