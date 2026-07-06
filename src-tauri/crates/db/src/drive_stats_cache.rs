use rusqlite::{Connection, OptionalExtension, Result};

pub use omega_drive_gateway::core::data::DriveStats;

pub fn refresh_drive_stats_cache(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM drive_stats_cache", [])?;
    conn.execute(
        "INSERT INTO drive_stats_cache (
            drive_scope, total_files, total_folders, total_size, trash_count, updated_at
         )
         SELECT
            COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my'),
            COUNT(CASE WHEN status IN ('ready', 'error') THEN 1 END),
            (SELECT COUNT(*) FROM folders),
            COALESCE(SUM(CASE WHEN status IN ('ready', 'error') THEN size END), 0),
            COUNT(CASE WHEN status = 'trashed' THEN 1 END),
            CAST(strftime('%s', 'now') AS INTEGER)
         FROM files",
        [],
    )?;
    Ok(())
}

pub fn get_drive_stats_cache(conn: &Connection, drive_scope: &str) -> Result<Option<DriveStats>> {
    let _ = drive_scope;
    conn.query_row(
        "SELECT total_files, total_folders, total_size, trash_count
         FROM drive_stats_cache
         LIMIT 1",
        [],
        |row| {
            Ok(DriveStats {
                total_files: row.get(0)?,
                total_folders: row.get(1)?,
                total_size: row.get(2)?,
                trash_count: row.get(3)?,
            })
        },
    )
    .optional()
}

