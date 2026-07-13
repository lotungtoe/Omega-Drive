use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::provider_quota_cache;

use super::PartMetadata;

pub fn delete_parts_by_type(conn: &Connection, file_id: i64, part_type: &str) -> Result<()> {
    if part_type != "chunk" {
        return Ok(());
    }
    let mut stmt = conn.prepare(
        "SELECT platform, COALESCE(SUM(size), 0) FROM parts WHERE file_id = ? GROUP BY platform",
    )?;
    let deleted_sizes = stmt
        .query_map(params![file_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>>>()?;
    conn.execute("DELETE FROM parts WHERE file_id = ?", params![file_id])?;
    for (platform, size) in deleted_sizes {
        let _ = provider_quota_cache::adjust_provider_quota_cache(conn, &platform, -size);
    }
    Ok(())
}

pub fn insert_part(
    conn: &Connection,
    file_id: i64,
    platform: &str,
    message_id: &str,
    attachment_name: Option<&str>,
    part_index: u32,
    size: i64,
    part_type: &str,
    checksum: Option<String>,
) -> Result<i64> {
    tracing::debug!(
        "DB: Inserting part for file_id={}, index={}, size={}, type={}",
        file_id,
        part_index,
        size,
        part_type
    );
    conn.execute(
        "INSERT INTO parts (file_id, platform, message_id, part_index, size, checksum) VALUES (?, ?, ?, ?, ?, ?)",
        params![file_id, platform, message_id, part_index, size, checksum],
    )?;
    let _ = (attachment_name, part_type);
    let _ = provider_quota_cache::adjust_provider_quota_cache(conn, platform, size);
    Ok(conn.last_insert_rowid())
}

pub fn update_part_remote_id(
    conn: &Connection,
    file_id: i64,
    part_index: u32,
    new_message_id: &str,
    attachment_name: Option<&str>,
    new_size: i64,
    part_type: &str,
    checksum: Option<String>,
) -> Result<()> {
    conn.execute(
        "UPDATE parts SET message_id = ?, size = ?, checksum = ? WHERE file_id = ? AND part_index = ?",
        params![new_message_id, new_size, checksum, file_id, part_index],
    )?;
    let _ = (attachment_name, part_type);
    let _ = provider_quota_cache::rebuild_provider_quota_cache(conn);
    Ok(())
}

pub fn get_parts_for_file(conn: &Connection, file_id: i64) -> Result<Vec<PartMetadata>> {
    let mut stmt = conn.prepare(
        "SELECT p.file_id, p.platform, p.message_id, p.part_index, p.size
         FROM parts p
         WHERE p.file_id = ?
         ORDER BY p.part_index ASC",
    )?;
    let rows = stmt.query_map(params![file_id], |row| {
        Ok(PartMetadata {
            id: 0,
            file_id: row.get(0)?,
            platform: row.get(1)?,
            message_id: row.get(2)?,
            part_index: row.get(3)?,
            size: row.get(4)?,
            checksum: None,
        })
    })?;
    rows.collect()
}

pub fn get_part_by_index(
    conn: &Connection,
    file_id: i64,
    part_index: u32,
) -> Result<Option<PartMetadata>> {
    conn.query_row(
        "SELECT p.id, p.file_id, p.platform, p.message_id, p.part_index, p.size, p.checksum
         FROM parts p
         WHERE p.file_id = ? AND p.part_index = ?
         LIMIT 1",
        params![file_id, part_index],
        |row| {
            Ok(PartMetadata {
                id: 0,
                file_id: row.get(0)?,
                platform: row.get(1)?,
                message_id: row.get(2)?,
                part_index: row.get(3)?,
                size: row.get(4)?,
                checksum: None,
            })
        },
    )
    .optional()
}

pub fn get_parts_for_file_by_type(
    conn: &Connection,
    file_id: i64,
    part_type: &str,
) -> Result<Vec<PartMetadata>> {
    if part_type != "chunk" {
        return Ok(Vec::new());
    }
    get_parts_for_file(conn, file_id)
}

pub fn get_original_parts_for_file(conn: &Connection, file_id: i64) -> Result<Vec<PartMetadata>> {
    get_parts_for_file(conn, file_id)
}

pub fn get_platform_usage(conn: &Connection, platform: &str) -> Result<u64> {
    if let Some(usage) = provider_quota_cache::get_provider_usage_cache(conn, platform)? {
        return Ok(usage);
    }
    let usage: Option<i64> = conn.query_row(
        "SELECT SUM(size) FROM parts WHERE platform = ?",
        params![platform],
        |row| row.get(0),
    )?;
    Ok(usage.unwrap_or(0) as u64)
}

pub fn get_part_count_for_file(conn: &Connection, file_id: i64) -> Result<usize> {
    let count: usize = conn.query_row(
        "SELECT COUNT(DISTINCT part_index) FROM parts WHERE file_id = ?",
        params![file_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn get_part_count_for_file_and_platform(
    conn: &Connection,
    file_id: i64,
    platform: &str,
) -> Result<i64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT part_index) FROM parts WHERE file_id = ? AND platform = ?",
        params![file_id, platform],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub fn get_part_counts_for_files(
    conn: &Connection,
    file_ids: &[i64],
) -> Result<HashMap<i64, usize>> {
    if file_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = std::iter::repeat_n("?", file_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT file_id, COUNT(DISTINCT part_index) FROM parts WHERE file_id IN ({}) GROUP BY file_id",
        placeholders
    );

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(file_ids.iter()), |row| {
        let file_id: i64 = row.get(0)?;
        let count: usize = row.get(1)?;
        Ok((file_id, count))
    })?;

    let mut map = HashMap::with_capacity(file_ids.len());
    for row in rows {
        let (file_id, count) = row?;
        map.insert(file_id, count);
    }

    Ok(map)
}

