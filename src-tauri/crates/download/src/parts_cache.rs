use std::collections::{HashMap, VecDeque};

use omega_drive_gateway::provider::storage::PartMetadata;

pub const PARTS_CACHE_MAX_BYTES: usize = 10 * 1024 * 1024;

fn part_heap_bytes(p: &PartMetadata) -> usize {
    p.platform.capacity()
        + p.message_id.capacity()
}

fn entry_bytes(parts: &[PartMetadata]) -> usize {
    std::mem::size_of::<PartMetadata>() * parts.len()
        + parts.iter().map(part_heap_bytes).sum::<usize>()
}

pub struct PartsCacheInner {
    entries: HashMap<i64, Vec<PartMetadata>>,
    lru: VecDeque<i64>,
    total_bytes: usize,
    max_bytes: usize,
}

impl PartsCacheInner {
    pub fn new(max_bytes: usize) -> Self {
        Self { entries: HashMap::new(), lru: VecDeque::new(), total_bytes: 0, max_bytes }
    }

    pub fn get_cloned(&mut self, file_id: i64) -> Option<Vec<PartMetadata>> {
        if self.entries.contains_key(&file_id) {
            if let Some(pos) = self.lru.iter().position(|k| *k == file_id) {
                self.lru.remove(pos);
                self.lru.push_back(file_id);
            }
            self.entries.get(&file_id).cloned()
        } else {
            None
        }
    }

    pub fn insert(&mut self, file_id: i64, parts: Vec<PartMetadata>) {
        let new_bytes = entry_bytes(&parts);
        if new_bytes > self.max_bytes {
            return;
        }

        if let Some(old) = self.entries.remove(&file_id) {
            if let Some(pos) = self.lru.iter().position(|k| *k == file_id) {
                self.lru.remove(pos);
            }
            self.total_bytes = self.total_bytes.saturating_sub(entry_bytes(&old));
        }

        while self.total_bytes + new_bytes > self.max_bytes {
            match self.lru.pop_front() {
                Some(evict_id) => {
                    if let Some(evicted) = self.entries.remove(&evict_id) {
                        self.total_bytes = self.total_bytes.saturating_sub(entry_bytes(&evicted));
                    }
                }
                None => break,
            }
        }

        self.total_bytes += new_bytes;
        self.entries.insert(file_id, parts);
        self.lru.push_back(file_id);
    }
}
