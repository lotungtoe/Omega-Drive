use std::sync::OnceLock;
use std::time::Duration;

use chrono::Utc;

use omega_drive_gateway::provider::provider_types::MediaSource;
use omega_drive_gateway::provider::storage::PartMetadata;

use crate::DownloadContext;

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
    part: &PartMetadata,
) -> Result<String, String> {
    let cache_key = format!("{}:{}", file_id, part.part_index);
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


