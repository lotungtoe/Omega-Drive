use std::sync::OnceLock;
use std::time::Duration;

use chrono::Utc;

use omega_drive_gateway::provider::provider_types::MediaSource;
use omega_drive_gateway::provider::storage::PartMetadata;

use crate::DownloadContext;

// ============================================================
// Bandwidth Tracker
// ============================================================

pub struct BwStats {
    pub avg_mbps: f64,
    pub min_mbps: f64,
    pub max_mbps: f64,
    pub total_bytes: u64,
}

pub struct BandwidthTracker {
    total_bytes: u64,
    prev_time: Duration,
    last_elapsed: Duration,
    min_rate: f64,
    max_rate: f64,
    initialized: bool,
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            prev_time: Duration::ZERO,
            last_elapsed: Duration::ZERO,
            min_rate: 0.0,
            max_rate: 0.0,
            initialized: false,
        }
    }

    pub fn record(&mut self, bytes: usize, elapsed: Duration) {
        self.total_bytes += bytes as u64;
        self.last_elapsed = elapsed;
        let chunk_time = elapsed.saturating_sub(self.prev_time);
        if chunk_time >= Duration::from_millis(1) {
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

    pub fn finalize(&self) -> BwStats {
        let avg_mbps = if self.last_elapsed > Duration::from_micros(1) && self.total_bytes > 0 {
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

// ============================================================
// HTTP Client
// ============================================================

pub fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(0)
            .pool_idle_timeout(Duration::from_secs(30))
            .http1_only()
            .build()
            .expect("Failed to build reqwest client")
    })
}

// ============================================================
// Download helpers
// ============================================================



// ============================================================
// Discord URL resolution
// ============================================================

pub async fn resolve_cached_discord_url(
    state: &DownloadContext,
    file_id: i64,
    part_num: u32,
    part: &PartMetadata,
) -> Result<String, String> {
    let cache_key = format!("{}:{}", file_id, part_num);
    if let Some(url) = {
        let cache = state.cdn_link_cache.read().await;
        cache.get(&cache_key).and_then(|(u, e)| {
            if *e > Utc::now() {
                Some(u.clone())
            } else {
                None
            }
        })
    } {
        return Ok(url);
    }

    let gateway = state.provider_runtime.stream_registry
        .get("discord")
        .ok_or_else(|| "Discord stream gateway unavailable".to_string())?;
    let source = gateway
        .resolve_media_source(part)
        .await
        .map_err(|e| format!("Loi Discord: {e}"))?;
    match source {
        MediaSource::ResolvedUrl { url, expiry } => {
            if let Some(expiry) = expiry {
                let mut cache = state.cdn_link_cache.write().await;
                cache.insert(cache_key, (url.clone(), expiry));
            }
            Ok(url)
        }
        MediaSource::ProviderOwned => Err("Discord stream gateway did not return URL".to_string()),
    }
}


