use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use bytes::Bytes;
use parking_lot::Mutex;

const EVICT_WATERMARK_RATIO: f64 = 0.9;

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

struct PartitionData {
    max_bytes: Option<usize>,
    current_bytes: usize,
    entries: HashMap<(i64, u64), Entry>,
    lru: VecDeque<(i64, u64)>,
    pinned: HashSet<(i64, u64)>,
}

struct Inner {
    total_max_bytes: usize,
    partitions: HashMap<String, PartitionData>,
    overflow: HashMap<String, usize>,
}

pub struct PartitionedMemCache {
    inner: Arc<Mutex<Inner>>,
}

impl PartitionedMemCache {
    pub fn new(total_bytes: usize, partitions: HashMap<String, PartitionConfig>) -> Self {
        let mut inner = Inner {
            total_max_bytes: total_bytes,
            partitions: HashMap::new(),
            overflow: HashMap::new(),
        };
        for (name, config) in partitions {
            inner.partitions.insert(
                name,
                PartitionData {
                    max_bytes: config.max_bytes,
                    current_bytes: 0,
                    entries: HashMap::new(),
                    lru: VecDeque::new(),
                    pinned: HashSet::new(),
                },
            );
        }
        Self {
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub async fn write(&self, file_id: i64, offset: u64, data: Bytes, namespace: &str) {
        if data.is_empty() {
            return;
        }
        let mut inner = self.inner.lock();
        let partition = inner
            .partitions
            .entry(namespace.to_string())
            .or_insert_with(|| PartitionData {
                max_bytes: None,
                current_bytes: 0,
                entries: HashMap::new(),
                lru: VecDeque::new(),
                pinned: HashSet::new(),
            });

        let data_len = data.len();
        let key = (file_id, offset);
        let old_len = partition
            .entries
            .get(&key)
            .map(|e| e.len as usize)
            .unwrap_or(0);
        let net_change = (data_len as isize) - (old_len as isize);

        partition.entries.insert(
            key,
            Entry {
                data,
                len: data_len as u64,
                namespace: namespace.to_string(),
            },
        );
        partition.current_bytes = (partition.current_bytes as isize + net_change) as usize;

        partition.lru.retain(|&k| k != key);
        partition.lru.push_front(key);

        let over_watermark = partition.max_bytes.map_or(false, |max| {
            partition.current_bytes as f64 > max as f64 * EVICT_WATERMARK_RATIO
        });
        if !over_watermark {
            return;
        }
        let max = partition.max_bytes;
        let _ = partition;

        Self::evict_watermark(&mut *inner, namespace);
        let still_over = max.is_some_and(|m| {
            inner.partitions.get(namespace).map_or(false, |p| {
                p.current_bytes as f64 > m as f64 * EVICT_WATERMARK_RATIO
            })
        });
        if still_over {
            Self::try_overflow(&mut *inner, namespace);
            let still_over2 = max.is_some_and(|m| {
                inner.partitions.get(namespace).map_or(false, |p| {
                    p.current_bytes as f64 > m as f64 * EVICT_WATERMARK_RATIO
                })
            });
            if still_over2 {
                Self::evict_watermark(&mut *inner, namespace);
            }
        }
    }

    pub async fn read(&self, file_id: i64, offset: u64, len: u64) -> Option<Bytes> {
        let mut inner = self.inner.lock();
        let key = (file_id, offset);
        for partition in inner.partitions.values_mut() {
            if let Some(entry) = partition.entries.get(&key) {
                if entry.len < len {
                    return None;
                }
                partition.lru.retain(|&k| k != key);
                partition.lru.push_front(key);
                return Some(entry.data.slice(..len as usize));
            }
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
        let mut inner = self.inner.lock();
        let partition = match inner.partitions.get_mut(namespace) {
            Some(p) => p,
            None => return,
        };
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

    fn evict_watermark(inner: &mut Inner, namespace: &str) {
        let partition = match inner.partitions.get_mut(namespace) {
            Some(p) => p,
            None => return,
        };
        let max = match partition.max_bytes {
            Some(m) => m,
            None => return,
        };
        let watermark = (max as f64 * EVICT_WATERMARK_RATIO) as usize;
        while partition.current_bytes > watermark {
            let victim = partition
                .lru
                .iter()
                .rev()
                .find(|&&key| partition.entries.contains_key(&key) && !partition.pinned.contains(&key))
                .copied();
            match victim {
                Some(key) => {
                    partition.lru.retain(|&k| k != key);
                    if let Some(entry) = partition.entries.remove(&key) {
                        partition.current_bytes =
                            partition.current_bytes.saturating_sub(entry.len as usize);
                    }
                }
                None => break,
            }
        }
    }

    fn try_overflow(inner: &mut Inner, namespace: &str) {
        let total_used: usize = inner.partitions.values().map(|p| p.current_bytes).sum();
        if total_used <= inner.total_max_bytes {
            let partition = inner.partitions.get(namespace).unwrap();
            if let Some(max) = partition.max_bytes {
                if partition.current_bytes > max {
                    inner
                        .overflow
                        .insert(namespace.to_string(), partition.current_bytes - max);
                }
            }
        } else {
            inner.overflow.remove(namespace);
        }
    }
}
