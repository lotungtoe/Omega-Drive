use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::Path;
use omega_drive_gateway::provider::storage::PartMetadata;

const HIT_BUFFER_MINUTES: i64 = 5;
const ANTI_FLAP_WINDOW_SECS: i64 = 30;

pub enum CacheLookup {
    Hit(String, DateTime<Utc>),
    Stale(String, DateTime<Utc>),
    Miss,
}

pub fn check_cache(
    cache: &HashMap<String, (String, DateTime<Utc>)>,
    key: &str,
    now: DateTime<Utc>,
) -> CacheLookup {
    match cache.get(key) {
        Some((url, expiry)) if *expiry > now + chrono::Duration::minutes(HIT_BUFFER_MINUTES) =>
            CacheLookup::Hit(url.clone(), *expiry),
        Some((url, expiry)) => CacheLookup::Stale(url.clone(), *expiry),
        None => CacheLookup::Miss,
    }
}

pub fn anti_flap_filter(
    last_expiry: Option<DateTime<Utc>>,
    new_expiry: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    cached: Option<(String, DateTime<Utc>)>,
) -> Option<(String, DateTime<Utc>)> {
    let prev = last_expiry?;
    if prev <= now { return None; }
    let fresh_expiry = new_expiry?;
    if fresh_expiry <= prev + chrono::Duration::seconds(ANTI_FLAP_WINDOW_SECS) {
        return cached;
    }
    None
}

pub fn list_expired_parts(
    parts_map: &HashMap<u32, PartMetadata>,
    cache: &HashMap<String, (String, DateTime<Utc>)>,
    file_id: i64,
    _now: DateTime<Utc>,
    threshold: DateTime<Utc>,
) -> Vec<u32> {
    let mut expired = Vec::new();
    for (&pn, meta) in parts_map.iter() {
        if meta.platform != "discord" { continue; }
        let key = format!("{}:{}", file_id, pn);
        match cache.get(&key) {
            None => expired.push(pn),
            Some((_, expiry)) if *expiry <= threshold => expired.push(pn),
            _ => {}
        }
    }
    expired
}

// ponytail: part number can end with '.' (legacy) or end of string (new)
pub fn parse_part_index(filename: &str) -> Option<u32> {
    let marker = ".part";
    let start = filename.find(marker)?;
    let rest = &filename[start + marker.len()..];
    let end = rest.find('.').unwrap_or(rest.len());
    rest[..end].parse::<u32>().ok()
}

pub fn read_disk_cache(path: &Path) -> HashMap<String, (String, DateTime<Utc>)> {
    let file = match std::fs::read_to_string(path) {
        Ok(f) => f,
        Err(_) => return HashMap::new(),
    };
    let now = Utc::now().timestamp();
    serde_json::from_str::<Vec<[String; 3]>>(&file)
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e[2].parse::<i64>().unwrap_or(0) > now)
        .filter_map(|e| {
            let ts = e[2].parse::<i64>().ok()?;
            let dt = DateTime::from_timestamp(ts, 0)?;
            Some((e[0].clone(), (e[1].clone(), dt)))
        })
        .collect()
}

pub fn build_snapshot(
    cache: &HashMap<String, (String, DateTime<Utc>)>,
    file_id: i64,
) -> Vec<[String; 3]> {
    let prefix = format!("{}:", file_id);
    let now = Utc::now().timestamp();
    cache.iter()
        .filter(|(k, (_, exp))| k.starts_with(&prefix) && exp.timestamp() > now)
        .map(|(k, (url, exp))| [k.clone(), url.clone(), exp.timestamp().to_string()])
        .collect()
}

pub fn persist_cache_to_disk(
    cache: &HashMap<String, (String, DateTime<Utc>)>,
    file_id: i64,
    base_dir: &Path,
) {
    let snapshot = build_snapshot(cache, file_id);
    if snapshot.is_empty() { return; }
    let path = base_dir.join("cache").join("discord").join(format!("{}.json", file_id));
    let _ = std::fs::create_dir_all(path.parent().expect("cache file path has parent dir"));
    if let Ok(json) = serde_json::to_string(&snapshot) {
        let _ = std::fs::write(&path, json);
    }
}
