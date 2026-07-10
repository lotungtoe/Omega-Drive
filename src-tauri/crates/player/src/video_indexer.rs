use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::PlayerContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContainerHint {
    Mp4,
    Mkv,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexHint {
    pub container: Option<ContainerHint>,
    pub hinted_parts: Vec<u32>,
}

#[derive(Clone, Default)]
pub struct VideoIndexer {
    cache: Arc<Mutex<HashMap<i64, IndexHint>>>,
}

impl VideoIndexer {
    pub async fn get_or_probe(
        &self,
        st: &PlayerContext,
        file_id: i64,
        file_size: u64,
        chunk_size: u32,
        total_parts: u32,
    ) -> Option<IndexHint> {
        if let Some(cached) = self.cache.lock().await.get(&file_id).cloned() {
            return Some(cached);
        }

        if let Some(idx) = st.idx_cache.load(file_id).await {
            let chunk_size = chunk_size as u64;
            let idx_part = if chunk_size > 0 {
                (idx.offset / chunk_size) as u32 + 1
            } else {
                1
            };
            let hint = IndexHint {
                container: self.detect_container_from_parts(st, file_id).await,
                hinted_parts: vec![idx_part],
            };
            self.cache.lock().await.insert(file_id, hint.clone());
            return Some(hint);
        }

        let hint = probe_index_hint(st, file_id, file_size, chunk_size, total_parts).await?;
        self.cache.lock().await.insert(file_id, hint.clone());
        Some(hint)
    }

    async fn detect_container_from_parts(
        &self, st: &PlayerContext, file_id: i64,
    ) -> Option<ContainerHint> {
        let head = crate::segmentgen::get_file_part_internal(st, file_id, 1)
            .await
            .ok()?;
        if looks_like_mp4(&head) {
            Some(ContainerHint::Mp4)
        } else if looks_like_mkv(&head) {
            Some(ContainerHint::Mkv)
        } else {
            None
        }
    }
}

async fn probe_index_hint(
    st: &PlayerContext,
    file_id: i64,
    _file_size: u64,
    chunk_size: u32,
    total_parts: u32,
) -> Option<IndexHint> {
    if chunk_size == 0 || total_parts == 0 {
        return None;
    }

    let head = crate::segmentgen::get_file_part_internal(st, file_id, 1)
        .await
        .ok()?;
    let tail = if total_parts > 1 {
        crate::segmentgen::get_file_part_internal(st, file_id, total_parts)
            .await
            .ok()
    } else {
        None
    };

    build_index_hint_from_windows(&head, tail.as_deref(), chunk_size as u64, total_parts)
}

fn build_index_hint_from_windows(
    head: &[u8],
    tail: Option<&[u8]>,
    chunk_size: u64,
    total_parts: u32,
) -> Option<IndexHint> {
    if looks_like_mp4(head) {
        let mut hinted_parts = Vec::new();
        if find_ascii_marker(head, b"moov").is_some() || find_ascii_marker(head, b"stss").is_some()
        {
            hinted_parts.push(1);
        }
        if let Some(tail) = tail {
            if find_ascii_marker(tail, b"moov").is_some()
                || find_ascii_marker(tail, b"stss").is_some()
            {
                hinted_parts.push(total_parts);
            }
        }
        hinted_parts.sort_unstable();
        hinted_parts.dedup();
        return Some(IndexHint {
            container: Some(ContainerHint::Mp4),
            hinted_parts,
        });
    }

    if looks_like_mkv(head) {
        let mut hinted_parts = Vec::new();
        if let Some(tail) = tail {
            if find_binary_marker(tail, &[0x1C, 0x53, 0xBB, 0x6B]).is_some() {
                hinted_parts.push(total_parts);
            }
        }
        if hinted_parts.is_empty() && chunk_size > 0 && total_parts > 0 {
            hinted_parts.push(total_parts);
        }
        return Some(IndexHint {
            container: Some(ContainerHint::Mkv),
            hinted_parts,
        });
    }

    None
}

fn looks_like_mp4(head: &[u8]) -> bool {
    find_ascii_marker(head, b"ftyp").is_some()
}

fn looks_like_mkv(head: &[u8]) -> bool {
    find_binary_marker(head, &[0x1A, 0x45, 0xDF, 0xA3]).is_some()
}

fn find_ascii_marker(bytes: &[u8], marker: &[u8]) -> Option<usize> {
    bytes
        .windows(marker.len())
        .position(|window| window == marker)
}

fn find_binary_marker(bytes: &[u8], marker: &[u8]) -> Option<usize> {
    bytes
        .windows(marker.len())
        .position(|window| window == marker)
}


