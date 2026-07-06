use rusqlite::{params, Connection, OptionalExtension, Result};

pub fn adjust_provider_quota_cache(
    conn: &Connection,
    provider: &str,
    delta_bytes: i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO provider_quota_cache (provider, used_bytes, updated_at)
         VALUES (?1, CASE WHEN ?2 > 0 THEN ?2 ELSE 0 END, CAST(strftime('%s', 'now') AS INTEGER))
         ON CONFLICT(provider) DO UPDATE SET
            used_bytes = MAX(provider_quota_cache.used_bytes + ?2, 0),
            updated_at = excluded.updated_at",
        params![provider, delta_bytes],
    )?;
    Ok(())
}

pub fn rebuild_provider_quota_cache(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM provider_quota_cache", [])?;
    conn.execute(
        "INSERT INTO provider_quota_cache (provider, used_bytes, updated_at)
         SELECT platform, COALESCE(SUM(size), 0), CAST(strftime('%s', 'now') AS INTEGER)
         FROM parts
         GROUP BY platform",
        [],
    )?;
    Ok(())
}

pub fn get_provider_usage_cache(conn: &Connection, provider: &str) -> Result<Option<u64>> {
    conn.query_row(
        "SELECT used_bytes FROM provider_quota_cache WHERE provider = ?",
        params![provider],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map(|value| value.map(|bytes| bytes.max(0) as u64))
}
