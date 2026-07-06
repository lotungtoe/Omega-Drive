use rusqlite::{params, Connection, OptionalExtension, Result};
use omega_drive_gateway::core::data::FileMetadata;

pub fn get_platform_usage(conn: &Connection, platform: &str) -> Result<u64> {
    let usage: Option<i64> = conn.query_row(
        "SELECT SUM(size) FROM parts WHERE platform = ?",
        params![platform],
        |row| row.get(0),
    )?;
    Ok(usage.unwrap_or(0) as u64)
}

pub fn get_file_by_id(conn: &Connection, file_id: i64) -> Result<Option<FileMetadata>> {
    conn.query_row(
        "SELECT f.id, f.filename, f.size, f.thread_id, f.folder_id,
                COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope,
                f.checksum, f.status, f.starred,
                f.created_at, f.deleted_at, uj.source_path AS local_path, f.kind, vf.duration_sec,
                f.last_accessed_at
         FROM files f
         LEFT JOIN video_files vf ON vf.file_id = f.id
         LEFT JOIN upload_jobs uj ON uj.file_id = f.id
         WHERE f.id = ?",
        params![file_id],
        |row| {
            Ok(FileMetadata {
                id: row.get(0)?,
                filename: row.get(1)?,
                size: row.get(2)?,
                thread_id: row.get::<_, Option<String>>(3)?.unwrap_or_default(),
                folder_id: row.get(4)?,
                drive_scope: row.get(5)?,
                checksum: row.get::<_, Option<String>>(6)?,
                status: row.get(7)?,
                starred: row.get(8)?,
                created_at: row.get(9)?,
                deleted_at: row.get::<_, Option<String>>(10)?,
                local_path: row.get::<_, Option<String>>(11)?,
                kind: row.get::<_, String>(12)?,
                duration_sec: row.get::<_, Option<f64>>(13)?,
                last_accessed_at: row.get(14)?,
            })
        },
    )
    .optional()
}
