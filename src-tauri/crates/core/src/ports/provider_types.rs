pub use omega_drive_gateway::provider::provider_types::*;

use chrono::{DateTime, Utc};

pub fn parse_discord_expires(url: &str) -> Option<DateTime<Utc>> {
    let query = url.split('?').nth(1)?;
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        let key = it.next()?;
        let value = it.next().unwrap_or("");
        if key == "ex" && !value.is_empty() {
            if let Ok(ts) = u64::from_str_radix(value, 16) {
                let ts_i64 = i64::try_from(ts).ok()?;
                return DateTime::from_timestamp(ts_i64, 0);
            }
        }
    }
    None
}

pub fn expiry_from_discord_url(url: &str, fallback_minutes: i64) -> DateTime<Utc> {
    parse_discord_expires(url)
        .unwrap_or_else(|| Utc::now() + chrono::Duration::minutes(fallback_minutes))
}
