use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use parking_lot::RwLock;

const EVICT_WATERMARK_RATIO: f64 = 0.9;
const WINDOW_STALE_DURATION: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct PartitionConfig {
    pub max_bytes: Option<usize>,
}

struct Entry {
    data: Bytes,
    len: u64,
    #[allow(dead_code)]
    namespace: String,
}

struct FileWindow {
    start: u64,
    end: u64,
    last_active: Instant,
}

impl FileWindow {
    fn expand(&mut self, offset: u64, len: u64) {
        self.start = self.start.min(offset);
        self.end = self.end.max(offset + len.saturating_sub(1));
        self.last_active = Instant::now();
    }
}

struct PartitionData {
    max_bytes: Option<usize>,
    current_bytes: usize,
    entries: BTreeMap<(i64, u64), Entry>,
    windows: HashMap<i64, FileWindow>,
    pinned: HashSet<(i64, u64)>,
}

impl PartitionData {
    fn update_window(&mut self, file_id: i64, offset: u64, len: u64) {
        self.windows
            .entry(file_id)
            .and_modify(|w| w.expand(offset, len))
            .or_insert_with(|| FileWindow {
                start: offset,
                end: offset + len.saturating_sub(1),
                last_active: Instant::now(),
            });
    }

    fn prune_stale_windows(&mut self) {
        self.windows.retain(|_, w| w.last_active.elapsed() < WINDOW_STALE_DURATION);
    }

    fn evict_score(&self, file_id: i64, offset: u64) -> u64 {
        match self.windows.get(&file_id) {
            Some(w) if offset < w.start => w.start - offset,
            Some(w) if offset > w.end => offset - w.end,
            Some(_) => 0,
            None => u64::MAX,
        }
    }
}

#[derive(Clone)]
pub struct PartitionedMemCache {
    partitions: Arc<RwLock<HashMap<String, Arc<RwLock<PartitionData>>>>>,
}

impl PartitionedMemCache {
    pub fn new(partitions: HashMap<String, PartitionConfig>) -> Self {
        let map: HashMap<_, _> = partitions
            .into_iter()
            .map(|(name, config)| {
                let pd = PartitionData {
                    max_bytes: config.max_bytes,
                    current_bytes: 0,
                    entries: BTreeMap::new(),
                    windows: HashMap::new(),
                    pinned: HashSet::new(),
                };
                (name, Arc::new(RwLock::new(pd)))
            })
            .collect();
        Self {
            partitions: Arc::new(RwLock::new(map)),
        }
    }

    pub async fn write(&self, file_id: i64, offset: u64, data: Bytes, namespace: &str) {
        if data.is_empty() {
            return;
        }
        let arc_lock = {
            let map = self.partitions.read();
            match map.get(namespace) {
                Some(p) => p.clone(),
                None => {
                    drop(map);
                    let mut map = self.partitions.write();
                    map.entry(namespace.to_string())
                        .or_insert_with(|| {
                            Arc::new(RwLock::new(PartitionData {
                                max_bytes: None,
                                current_bytes: 0,
                                entries: BTreeMap::new(),
                                windows: HashMap::new(),
                                pinned: HashSet::new(),
                            }))
                        })
                        .clone()
                }
            }
        };
        let mut partition = arc_lock.write();
        let max_bytes = partition.max_bytes;

        let data_len = data.len() as u64;
        let key = (file_id, offset);
        let old_len = partition
            .entries
            .get(&key)
            .map(|e| e.len)
            .unwrap_or(0);
        let net_change = (data_len as isize) - (old_len as isize);

        partition.entries.insert(
            key,
            Entry {
                data,
                len: data_len,
                namespace: namespace.to_string(),
            },
        );
        partition.current_bytes = (partition.current_bytes as isize + net_change) as usize;

        // Merge adjacent entries (max 1MB per entry)
        let mut block_off = offset;
        let mut total_len = data_len;

        // Backward merge: gộp với entry liền trước nếu adjacent
        let pk = partition.entries
            .range(..(file_id, block_off))
            .next_back()
            .and_then(|(&k, e)| {
                if k.0 == file_id && k.1 + e.len >= block_off { Some(k) } else { None }
            });
        if let Some(pk) = pk {
            let prev = partition.entries.remove(&pk).unwrap();
            let cur = partition.entries.remove(&(file_id, block_off)).unwrap();
            let combined_len = prev.len + cur.len;
            if combined_len <= 1_048_576 {
                let mut combined = Vec::with_capacity(combined_len as usize);
                combined.extend_from_slice(&prev.data);
                combined.extend_from_slice(&cur.data);
                partition.entries.insert(pk, Entry {
                    data: Bytes::from(combined),
                    len: combined_len,
                    namespace: namespace.to_string(),
                });
                block_off = pk.1;
                total_len = combined_len;
            } else {
                partition.entries.insert(pk, prev);
                partition.entries.insert((file_id, block_off), cur);
            }
        }

        // Forward merge: gộp với các entry liền sau
        loop {
            if total_len >= 1_048_576 { break; }
            let next_key = (file_id, block_off + total_len);
            let Some((&k, _)) = partition.entries.range(next_key..).next() else { break };
            if k.0 != file_id { break; }
            if block_off + total_len < k.1 { break; }

            let next_entry = partition.entries.remove(&k).unwrap();
            let mut cur_entry = partition.entries.remove(&(file_id, block_off)).unwrap();
            let mut combined = Vec::with_capacity(cur_entry.data.len() + next_entry.data.len());
            combined.extend_from_slice(&cur_entry.data);
            combined.extend_from_slice(&next_entry.data);
            cur_entry.data = Bytes::from(combined);
            cur_entry.len = total_len + next_entry.len;
            partition.entries.insert((file_id, block_off), cur_entry);
            total_len += next_entry.len;
        }

        partition.update_window(file_id, offset, data_len);

        let over_watermark = max_bytes.map_or(false, |max| {
            partition.current_bytes as f64 > max as f64 * EVICT_WATERMARK_RATIO
        });
        if !over_watermark {
            return;
        }

        Self::evict_watermark(&mut partition);
        let still_over = max_bytes.is_some_and(|m| {
            partition.current_bytes as f64 > m as f64 * EVICT_WATERMARK_RATIO
        });
        if still_over {
            Self::evict_watermark(&mut partition);
        }
    }

    pub async fn read(&self, file_id: i64, offset: u64, len: u64) -> Option<Bytes> {
        let partitions: Vec<_> = {
            let map = self.partitions.read();
            map.values().map(|p| p.clone()).collect()
        };
        let end = offset + len;

        for p in &partitions {
            let data = p.read();
            let base = data.entries.range(..=(file_id, offset)).next_back()?;
            if base.0 .0 != file_id { continue; }
            let base_entry_off = base.0 .1;
            let base_entry_len = base.1.len;
            if base_entry_off > offset || base_entry_off + base_entry_len <= offset { continue; }

            let mut result = Vec::with_capacity(len as usize);
            let mut cur = offset;
            while cur < end {
                let Some((&k, e)) = data.entries.range(..=(file_id, cur)).next_back() else {
                    return None;
                };
                if k.0 != file_id || k.1 + e.len <= cur { return None; }
                let skip = (cur - k.1) as usize;
                let take = ((e.data.len() - skip) as usize).min((end - cur) as usize);
                result.extend_from_slice(&e.data[skip..][..take]);
                cur += take as u64;
            }

            drop(data);
            let mut w = p.write();
            w.update_window(file_id, offset, len);
            return Some(Bytes::from(result));
        }
        None
    }

    pub async fn set_pin_window(
        &self,
        file_id: i64,
        center: u64,
        half: u64,
        max: u64,
        namespace: &str,
    ) {
        let arc_lock = {
            let map = self.partitions.read();
            match map.get(namespace) {
                Some(p) => p.clone(),
                None => return,
            }
        };
        let mut partition = arc_lock.write();
        partition.pinned.retain(|&(fid, _)| fid != file_id);

        let start = center.saturating_sub(half);
        let end = center.saturating_add(half);
        let mut pinned_bytes: u64 = 0;

        let keys: Vec<(i64, u64)> = partition
            .entries
            .keys()
            .filter(|&&(fid, off)| fid == file_id && off >= start && off < end)
            .copied()
            .collect();

        for key in keys {
            if max > 0 {
                if let Some(entry) = partition.entries.get(&key) {
                    let new_total = pinned_bytes + entry.len;
                    if new_total > max {
                        continue;
                    }
                    pinned_bytes = new_total;
                }
            }
            partition.pinned.insert(key);
        }
    }

    fn evict_watermark(partition: &mut PartitionData) {
        let max = match partition.max_bytes {
            Some(m) => m,
            None => return,
        };
        let watermark = (max as f64 * EVICT_WATERMARK_RATIO) as usize;

        partition.prune_stale_windows();

        if partition.current_bytes <= watermark {
            return;
        }

        let mut candidates: Vec<((i64, u64), u64)> = partition
            .entries
            .keys()
            .filter(|k| !partition.pinned.contains(k))
            .map(|k| (*k, partition.evict_score(k.0, k.1)))
            .collect();

        candidates.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        for (key, _) in candidates {
            if partition.current_bytes <= watermark {
                break;
            }
            if let Some(entry) = partition.entries.remove(&key) {
                partition.current_bytes =
                    partition.current_bytes.saturating_sub(entry.len as usize);
            }
        }
    }

    pub async fn wait_range(&self, file_id: i64, offset: u64, len: u64) -> Result<Bytes, String> {
        loop {
            if let Some(data) = self.read(file_id, offset, len).await {
                return Ok(data);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }

    pub async fn clear(&self) {
        let mut map = self.partitions.write();
        map.clear();
    }
}

#[cfg(test)]
#[path = "partitioned_mem_cache_test.rs"]
mod partitioned_mem_cache_tests;
