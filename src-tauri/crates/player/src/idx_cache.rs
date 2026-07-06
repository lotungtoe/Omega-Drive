use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use bytes::Bytes;
use parking_lot::RwLock;

#[derive(Clone, Debug)]
pub struct IdxEntry {
    pub offset: u64,
    pub size: u64,
    pub bytes: Bytes,
}

impl IdxEntry {
    /// Check if byte range [req_start, req_start+req_len) overlaps with idx range
    pub fn overlaps(&self, req_start: u64, req_len: u64) -> bool {
        let req_end = req_start.saturating_add(req_len);
        let idx_end = self.offset.saturating_add(self.size);
        req_start < idx_end && req_end > self.offset
    }

    /// Slice bytes for the overlapping portion
    pub fn slice_for(&self, req_start: u64, req_len: u64) -> Option<Bytes> {
        if !self.overlaps(req_start, req_len) {
            return None;
        }
        let req_end = req_start.saturating_add(req_len);
        let idx_end = self.offset.saturating_add(self.size);
        let overlap_start = req_start.max(self.offset);
        let overlap_end = req_end.min(idx_end);
        if overlap_end <= overlap_start {
            return None;
        }
        let cache_start = (overlap_start - self.offset) as usize;
        let cache_end = (overlap_end - self.offset) as usize;
        Some(self.bytes.slice(cache_start..cache_end))
    }
}

#[derive(Clone)]
pub struct IdxCache {
    base_dir: PathBuf,
    inner: Arc<RwLock<HashMap<i64, IdxEntry>>>,
}

impl IdxCache {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load idx entry for a file. Checks RAM cache first, then disk.
    pub async fn load(&self, file_id: i64) -> Option<IdxEntry> {
        if let Some(entry) = self.inner.read().get(&file_id).cloned() {
            return Some(entry);
        }
        let path = self.base_dir.join(format!("{}.idx", file_id));
        let data = tokio::fs::read(&path).await.ok()?;
        if data.len() < 20 || &data[0..4] != b"IDXC" {
            return None;
        }
        let offset = u64::from_be_bytes(data[4..12].try_into().ok()?);
        let size = u64::from_be_bytes(data[12..20].try_into().ok()?);
        let entry = IdxEntry {
            offset,
            size,
            bytes: Bytes::from(data[20..].to_vec()),
        };
        self.inner.write().insert(file_id, entry.clone());
        Some(entry)
    }
}
