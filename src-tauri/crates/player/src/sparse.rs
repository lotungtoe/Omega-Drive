use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Arc;

use bytes::Bytes;
use parking_lot::Mutex;

const EVICT_WATERMARK_RATIO: f64 = 0.9;

pub struct SparseCache {
    inner: Arc<Mutex<Inner>>,
    notify: tokio::sync::watch::Sender<u64>,
}

struct Inner {
    files: HashMap<i64, FileChunks>,
    lru: VecDeque<(i64, u64)>,
    pinned: HashSet<(i64, u64, u64)>,
    max_bytes: usize,
    current_bytes: usize,
    write_version: u64,
}

type FileChunks = BTreeMap<u64, Bytes>;

impl SparseCache {
    pub(crate) fn new(max_bytes: usize) -> Self {
        let (tx, _) = tokio::sync::watch::channel(0);
        Self {
            inner: Arc::new(Mutex::new(Inner {
                files: HashMap::new(),
                lru: VecDeque::new(),
                pinned: HashSet::new(),
                max_bytes,
                current_bytes: 0,
                write_version: 0,
            })),
            notify: tx,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn current_bytes(&self) -> usize {
        self.inner.lock().current_bytes
    }

    pub(crate) async fn write(&self, file_id: i64, offset: u64, data: Bytes) {
        if data.is_empty() {
            return;
        }
        let data_end = offset + data.len() as u64;
        let needs_evict;
        let ver;
        {
            let mut inner = self.inner.lock();
            let chunks = inner.files.entry(file_id).or_default();

            let mut net_change: isize = 0;
            let mut merge_start = offset;
            let mut merge_end = data_end;
            let mut old: Vec<(u64, Bytes)> = Vec::new();

            // Left chunk (touches or overlaps)
            if let Some((&pos, bytes)) = chunks.range(..offset).next_back() {
                let chunk_end = pos + bytes.len() as u64;
                if chunk_end > offset {
                    merge_start = merge_start.min(pos);
                    merge_end = merge_end.max(chunk_end);
                    old.push((pos, bytes.clone()));
                    net_change -= bytes.len() as isize;
                    chunks.remove(&pos);
                }
            }

            // Right chunks — iterative expansion for chain merges
            loop {
                let mut expanded = false;
                let keys: Vec<u64> =
                    chunks.range(merge_start..=merge_end).map(|(&k, _)| k).collect();
                for &pos in &keys {
                    if let Some(bytes) = chunks.remove(&pos) {
                        let b_len = bytes.len();
                        let ce = pos + b_len as u64;
                        if ce > merge_end {
                            merge_end = ce;
                            expanded = true;
                        }
                        old.push((pos, bytes));
                        net_change -= b_len as isize;
                    }
                }
                if !expanded {
                    break;
                }
            }

            // Build merged buffer
            let final_len = (merge_end - merge_start) as usize;
            let mut buf = vec![0u8; final_len];
            for (pos, bytes) in &old {
                let rel = (pos - merge_start) as usize;
                buf[rel..rel + bytes.len()].copy_from_slice(bytes);
            }
            let rel = (offset - merge_start) as usize;
            let rel_end = rel + data.len();
            buf[rel..rel_end].copy_from_slice(&data);

            let merged = Bytes::from(buf);
            let final_len = merged.len();
            chunks.insert(merge_start, merged);
            net_change += final_len as isize;
            inner.current_bytes = (inner.current_bytes as isize + net_change).max(0) as usize;

            inner.lru.retain(|&k| k != (file_id, merge_start));
            inner.lru.push_front((file_id, merge_start));

            inner.write_version = inner.write_version.wrapping_add(1);
            ver = inner.write_version;

            needs_evict =
                inner.current_bytes as f64 > inner.max_bytes as f64 * EVICT_WATERMARK_RATIO;
        debug_log!("sparse", "write: file={} off={} len={} merge_old={} cache={:.0}%",
            file_id, offset, data.len(), old.len(),
            inner.current_bytes as f64 / inner.max_bytes as f64 * 100.0);
        }
        self.notify.send(ver).ok();
        if needs_evict {
            let e_start = std::time::Instant::now();
            self.evict_watermark();
            debug_log!("sparse", "evict: file={} elapsed={:?}", file_id, e_start.elapsed());
        }
    }

    fn range_filled(&self, inner: &Inner, file_id: i64, offset: u64, len: u64) -> bool {
        if len == 0 {
            return true;
        }
        let chunks = match inner.files.get(&file_id) {
            Some(c) => c,
            None => return false,
        };
        let end = offset.saturating_add(len);
        let mut cur = offset;
        if let Some((&chunk_off, chunk_data)) = chunks.range(..=offset).rev().next() {
            let chunk_end = chunk_off + chunk_data.len() as u64;
            if chunk_end > cur {
                cur = chunk_end;
            }
        }
        if cur >= end {
            return true;
        }
        for (&chunk_off, chunk_data) in chunks.range(offset..) {
            if chunk_off > cur {
                break;
            }
            let chunk_end = chunk_off + chunk_data.len() as u64;
            if chunk_end > cur {
                cur = chunk_end;
            }
            if cur >= end {
                return true;
            }
        }
        false
    }

    fn assemble_range(inner: &Inner, file_id: i64, offset: u64, len: u64) -> Option<Bytes> {
        let chunks = inner.files.get(&file_id)?;
        let end = offset + len;
        let mut cur = offset;
        let mut buf = Vec::with_capacity(len as usize);

        if let Some((&chunk_off, chunk_data)) = chunks.range(..=offset).rev().next() {
            let chunk_end = chunk_off + chunk_data.len() as u64;
            if chunk_end > cur {
                let skip = (cur - chunk_off) as usize;
                let take = ((chunk_end - cur) as usize).min((end - cur) as usize);
                buf.extend_from_slice(&chunk_data[skip..skip + take]);
                cur += take as u64;
                if cur >= end {
                    return Some(Bytes::from(buf));
                }
            }
        }

        for (&chunk_off, chunk_data) in chunks.range(offset..) {
            if chunk_off > cur || cur >= end {
                break;
            }
            let chunk_end = chunk_off + chunk_data.len() as u64;
            if chunk_end <= cur {
                continue;
            }
            let skip = cur.saturating_sub(chunk_off) as usize;
            let take = ((chunk_end - cur) as usize).min((end - cur) as usize);
            buf.extend_from_slice(&chunk_data[skip..skip + take]);
            cur += take as u64;
        }

        Some(Bytes::from(buf))
    }

    pub(crate) fn is_range_filled(&self, file_id: i64, offset: u64, len: u64) -> bool {
        let inner = self.inner.lock();
        self.range_filled(&inner, file_id, offset, len)
    }

    pub(crate) async fn wait_range(
        &self,
        file_id: i64,
        offset: u64,
        len: u64,
    ) -> Result<Bytes, String> {
        let mut rx = self.notify.subscribe();
        let w_start = std::time::Instant::now();
        loop {
            {
                let inner = self.inner.lock();
                if self.range_filled(&inner, file_id, offset, len) {
                    let elapsed = w_start.elapsed();
                    if elapsed > std::time::Duration::from_millis(1) {
                        debug_log!("wait", "wait_range blocked: file={} off={} len={} elapsed={:?}",
                            file_id, offset, len, elapsed);
                    }
                    return Ok(Self::assemble_range(&inner, file_id, offset, len).unwrap());
                }
            }
            let _ = rx.changed().await;
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }

    pub(crate) fn set_pin_window(
        &self,
        file_id: i64,
        center_file_offset: u64,
        before_bytes: u64,
        after_bytes: u64,
    ) {
        let mut inner = self.inner.lock();
        inner.pinned.retain(|(fid, _, _)| *fid != file_id);
        let start = center_file_offset.saturating_sub(before_bytes);
        let end = center_file_offset.saturating_add(after_bytes);
        inner.pinned.insert((file_id, start, end));
    }

    pub(crate) fn unpin_file(&self, file_id: i64) {
        let mut inner = self.inner.lock();
        inner.pinned.retain(|(fid, _, _)| *fid != file_id);
    }

    pub(crate) fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.files.clear();
        inner.lru.clear();
        inner.pinned.clear();
        inner.current_bytes = 0;
    }

    fn evict_watermark(&self) {
        loop {
            let should_continue = {
                let mut inner = self.inner.lock();
                if inner.current_bytes as f64 <= inner.max_bytes as f64 * EVICT_WATERMARK_RATIO
                {
                    false
                } else {
                    let mut to_evict: Option<(i64, u64)> = None;
                    for &(fid, off) in inner.lru.iter().rev() {
                        if !inner.files.get(&fid).and_then(|c| c.get(&off)).is_some() {
                            continue;
                        }
                        if inner.pinned.iter().any(|(pf, ps, pe)| {
                            *pf == fid && off >= *ps && off < *pe
                        }) {
                            continue;
                        }
                        to_evict = Some((fid, off));
                        break;
                    }
                    let (fid, off) = match to_evict {
                        Some(e) => e,
                        None => return,
                    };
                    inner.lru.retain(|&k| k != (fid, off));
                    if let Some(data) = inner.files.get_mut(&fid).and_then(|c| c.remove(&off)) {
                        inner.current_bytes = inner.current_bytes.saturating_sub(data.len());
                    }
                    true
                }
            };
            if !should_continue {
                break;
            }
        }
    }
}

use omega_drive_gateway::player::cache::ByteCache;
use async_trait::async_trait;

#[async_trait]
impl ByteCache for SparseCache {
    async fn write(&self, file_id: i64, offset: u64, data: Bytes) {
        SparseCache::write(self, file_id, offset, data).await
    }

    async fn is_range_filled(&self, file_id: i64, offset: u64, len: u64) -> bool {
        SparseCache::is_range_filled(self, file_id, offset, len)
    }

    async fn wait_range(&self, file_id: i64, offset: u64, len: u64) -> Result<Bytes, String> {
        SparseCache::wait_range(self, file_id, offset, len).await
    }

    async fn set_pin_window(&self, file_id: i64, center: u64, half: u64, max: u64) {
        SparseCache::set_pin_window(self, file_id, center, half, max)
    }

    async fn clear(&self) {
        SparseCache::clear(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use std::sync::Arc;
    use tokio::time::sleep;

    #[tokio::test]
    async fn write_then_read() {
        let cache = SparseCache::new(1024 * 1024);
        cache.write(1, 0, Bytes::from("hello")).await;
        assert!(cache.is_range_filled(1, 0, 5));
        let out = cache.wait_range(1, 0, 5).await.unwrap();
        assert_eq!(&out[..], b"hello");
    }

    #[tokio::test]
    async fn wait_blocks_until_data() {
        let cache = Arc::new(SparseCache::new(1024 * 1024));
        let c2 = cache.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(50)).await;
            c2.write(1, 0, Bytes::from("data")).await;
        });
        let t = Instant::now();
        let out = cache.wait_range(1, 0, 4).await.unwrap();
        assert!(t.elapsed() >= Duration::from_millis(30));
        assert_eq!(&out[..], b"data");
    }

    #[tokio::test]
    async fn merge_adjacent_chunks() {
        let cache = SparseCache::new(1024 * 1024);
        cache.write(1, 0, Bytes::from("aaa")).await;
        cache.write(1, 6, Bytes::from("ccc")).await;
        cache.write(1, 3, Bytes::from("bbb")).await;
        assert!(cache.is_range_filled(1, 0, 9));
        let out = cache.wait_range(1, 0, 9).await.unwrap();
        assert_eq!(&out[..], b"aaabbbccc");
    }

    #[tokio::test]
    async fn merge_overlapping() {
        let cache = SparseCache::new(1024 * 1024);
        cache.write(1, 0, Bytes::from("AAAAAA")).await;
        cache.write(1, 3, Bytes::from("BBB")).await;
        assert_eq!(cache.current_bytes(), 6);
        let out = cache.wait_range(1, 0, 6).await.unwrap();
        assert_eq!(&out[..], b"AAABBB");
    }

    #[tokio::test]
    async fn gap_detected() {
        let cache = SparseCache::new(1024 * 1024);
        cache.write(1, 0, Bytes::from("AA")).await;
        cache.write(1, 4, Bytes::from("BB")).await;
        assert!(!cache.is_range_filled(1, 0, 6));
    }

    #[tokio::test]
    async fn evicts_when_over_limit() {
        let cache = SparseCache::new(100);
        for i in 0..10u64 {
            cache.write(1, i * 10, Bytes::from(vec![b'x'; 10])).await;
        }
        assert!(cache.current_bytes() <= 100);
    }

    #[tokio::test]
    async fn pinned_not_evicted() {
        let cache = SparseCache::new(60);
        cache.set_pin_window(1, 0, 10, 10);
        cache.write(1, 0, Bytes::from(vec![b'a'; 10])).await;
        cache.write(1, 10, Bytes::from(vec![b'b'; 10])).await;
        cache.write(1, 20, Bytes::from(vec![b'c'; 10])).await;
        cache.write(1, 30, Bytes::from(vec![b'd'; 10])).await;
        cache.write(1, 40, Bytes::from(vec![b'e'; 10])).await;
        cache.write(1, 50, Bytes::from(vec![b'f'; 10])).await;
        cache.unpin_file(1);
        cache.write(1, 60, Bytes::from(vec![b'g'; 10])).await;
        assert!(cache.current_bytes() <= 60);
    }



    #[tokio::test(flavor = "multi_thread")]
    async fn chunked_streaming() {
        let cache = Arc::new(SparseCache::new(1024 * 1024));
        let c2 = cache.clone();
        tokio::spawn(async move {
            for i in 0..4u64 {
                sleep(Duration::from_millis(10)).await;
                c2.write(1, i * 65536, Bytes::from(vec![b'A' as u8 + i as u8; 65536])).await;
            }
        });
        for i in 0..4u64 {
            let chunk = cache.wait_range(1, i * 65536, 65536).await.unwrap();
            assert_eq!(chunk.len(), 65536);
            assert_eq!(chunk[0], b'A' + i as u8);
        }
    }

    #[tokio::test]
    async fn two_files_independent() {
        let cache = SparseCache::new(1024 * 1024);
        cache.write(1, 0, Bytes::from("file1")).await;
        cache.write(2, 0, Bytes::from("file2")).await;
        assert!(cache.is_range_filled(1, 0, 5));
        assert!(cache.is_range_filled(2, 0, 5));
        let out1 = cache.wait_range(1, 0, 5).await.unwrap();
        let out2 = cache.wait_range(2, 0, 5).await.unwrap();
        assert_eq!(&out1[..], b"file1");
        assert_eq!(&out2[..], b"file2");
    }

    #[tokio::test]
    async fn clear_removes_all() {
        let cache = SparseCache::new(1024 * 1024);
        cache.write(1, 0, Bytes::from("data")).await;
        cache.clear();
        assert!(!cache.is_range_filled(1, 0, 4));
        assert_eq!(cache.current_bytes(), 0);
    }

}
