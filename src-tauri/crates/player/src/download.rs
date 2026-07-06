use bytes::Bytes;
use futures_util::StreamExt;

use omega_drive_gateway::provider::storage::PartMetadata;

use crate::PlayerContext;

pub(crate) struct BwStats {
    pub avg_mbps: f64,
    pub min_mbps: f64,
    pub max_mbps: f64,
    pub total_bytes: u64,
}

pub(crate) struct BandwidthTracker {
    total_bytes: u64,
    prev_time: std::time::Duration,
    last_elapsed: std::time::Duration,
    min_rate: f64,
    max_rate: f64,
    initialized: bool,
}

impl BandwidthTracker {
    pub(crate) fn new() -> Self {
        Self {
            total_bytes: 0,
            prev_time: std::time::Duration::ZERO,
            last_elapsed: std::time::Duration::ZERO,
            min_rate: 0.0,
            max_rate: 0.0,
            initialized: false,
        }
    }

    pub(crate) fn record(&mut self, bytes: usize, elapsed: std::time::Duration) {
        self.total_bytes += bytes as u64;
        self.last_elapsed = elapsed;
        let chunk_time = elapsed.saturating_sub(self.prev_time);
        if chunk_time >= std::time::Duration::from_millis(1) {
            let rate = bytes as f64 / chunk_time.as_secs_f64() / 1024.0 / 1024.0;
            if self.initialized {
                self.min_rate = self.min_rate.min(rate);
                self.max_rate = self.max_rate.max(rate);
            } else {
                self.min_rate = rate;
                self.max_rate = rate;
                self.initialized = true;
            }
        }
        self.prev_time = elapsed;
    }

    pub(crate) fn finalize(&self) -> BwStats {
        let avg_mbps = if self.last_elapsed > std::time::Duration::from_micros(1) && self.total_bytes > 0 {
            self.total_bytes as f64 / self.last_elapsed.as_secs_f64() / 1024.0 / 1024.0
        } else {
            0.0
        };
        BwStats {
            avg_mbps,
            min_mbps: if self.initialized { self.min_rate } else { 0.0 },
            max_mbps: if self.initialized { self.max_rate } else { 0.0 },
            total_bytes: self.total_bytes,
        }
    }
}

pub(crate) fn http_client() -> &'static reqwest::Client {
    static CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(10))
            .pool_max_idle_per_host(0)
            .pool_idle_timeout(std::time::Duration::from_secs(30))
            .http1_only()
            .build()
            .expect("Failed to build reqwest client")
    })
}

pub(crate) async fn download_url(url: &str) -> Result<Vec<u8>, String> {
    let client = http_client();
    let res = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    debug_log!("cdn", "{} {}", status, url);
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("HTTP_403".to_string());
    }
    if !status.is_success() {
        return Err(format!("HTTP_STATUS_{}", status.as_u16()));
    }
    res.bytes()
        .await
        .map_err(|e| e.to_string())
        .map(|b| b.to_vec())
}

pub(crate) async fn download_url_stream(
    url: &str,
    file_id: i64,
    part_num: u32,
    file_offset: u64,
    sparse_cache: &crate::sparse::SparseCache,
) -> Result<(), String> {
    if let Some(t1) = crate::bridge::take_t1_mark(file_id) {
        debug_log!("t1", "file={} t1={}µs", file_id, t1.as_micros());
    }
    let client = http_client();
    let t_req = std::time::Instant::now();
    let res = client.get(url).send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    let ttfb = t_req.elapsed();
    debug_log!("cdn_bw", "file={} part={} discord coord ttfb={:?} status={}", file_id, part_num, ttfb, status);
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err("HTTP_403".to_string());
    }
    if !status.is_success() {
        return Err(format!("HTTP_STATUS_{}", status.as_u16()));
    }
    let mut stream = res.bytes_stream();
    let bw_start = std::time::Instant::now();
    let mut bw = BandwidthTracker::new();
    let mut pos: u64 = 0;
    let mut write_time = std::time::Duration::ZERO;
    let mut chunk_count: u32 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        let len = chunk.len();
        let data = Bytes::from(chunk);
        bw.record(len, bw_start.elapsed());
        let w_start = std::time::Instant::now();
        sparse_cache.write(file_id, file_offset + pos, data).await;
        write_time += w_start.elapsed();
        chunk_count += 1;
        pos += len as u64;
        if bw_start.elapsed() > std::time::Duration::from_secs(10) {
            return Err("SLOW_DOWNLOAD".to_string());
        }
    }
    let s = bw.finalize();
    let net_time = bw_start.elapsed().checked_sub(write_time).unwrap_or_default();
    debug_log!("cdn_bw", "file={} part={} discord coord avg={:.1}MB/s min={:.1}MB/s max={:.1}MB/s size={} elapsed={:?}",
        file_id, part_num, s.avg_mbps, s.min_mbps, s.max_mbps, s.total_bytes, bw_start.elapsed());
    debug_log!("cdn_bw", "file={} part={} discord coord write_time={:?} chunk_count={} net_time={:?}",
        file_id, part_num, write_time, chunk_count, net_time);
    Ok(())
}

#[derive(Clone)]
pub(crate) struct ChunkMeta {
    pub part_id: i64,
    pub platform: String,
    pub message_id: String,
    pub attachment_name: Option<String>,
    pub part_type: String,
    pub size: u64,
}

pub(crate) fn fetch_part_metadata(meta: &ChunkMeta, file_id: i64, part_num: u32) -> PartMetadata {
    PartMetadata {
        id: meta.part_id,
        file_id,
        platform: meta.platform.clone(),
        message_id: meta.message_id.clone(),
        attachment_name: meta.attachment_name.clone(),
        part_index: part_num,
        size: meta.size as i64,
        part_type: meta.part_type.clone(),
        duration: None,
        checksum: None,
    }
}

pub(crate) async fn load_chunk_meta(st: &PlayerContext, file_id: i64, part_num: u32) -> Result<ChunkMeta, String> {
    if let Some(selected) = st.player_runtime.get_original_part(file_id, part_num).await {
        let meta = ChunkMeta {
            part_id: selected.id,
            platform: selected.platform.clone(),
            message_id: selected.message_id.clone(),
            attachment_name: selected.attachment_name.clone(),
            part_type: selected.part_type.clone(),
            size: selected.size.max(0) as u64,
        };
        debug_log!("meta", "load_chunk_meta[cached]: file={} part={} platform={} type={} size={}",
            file_id, part_num, meta.platform, meta.part_type, meta.size);
        return Ok(meta);
    }

    let _file = st.file_repo.get_file_by_id(file_id)
        .await
        .map_err(|e| format!("Loi DB: {e}"))?
        .ok_or_else(|| "Khong tim thay file".to_string())?;
    let selected = st.file_repo.get_part_by_index(file_id, part_num)
        .await
        .map_err(|e| format!("DB chunk error: {e}"))?
        .ok_or_else(|| format!("Chunk {part_num} not found"))?;
    st.player_runtime
        .cache_original_parts(file_id, vec![selected.clone()])
        .await;
    let meta = ChunkMeta {
        part_id: selected.id,
        platform: selected.platform.clone(),
        message_id: selected.message_id,
        attachment_name: selected.attachment_name,
        part_type: selected.part_type,
        size: selected.size.max(0) as u64,
    };
    debug_log!("meta", "load_chunk_meta[db]: file={} part={} platform={} type={} size={}",
        file_id, part_num, meta.platform, meta.part_type, meta.size);
    Ok(meta)
}




