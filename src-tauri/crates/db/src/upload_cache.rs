use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Result};

pub use omega_drive_gateway::core::data::UploadJob;

fn map_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<UploadJob> {
    Ok(UploadJob {
        id: row.get(0)?,
        file_id: row.get(1)?,
        source_path: row.get(2)?,
        state: row.get(3)?,
        error: row.get(4)?,
        error_code: row.get(5)?,
        done_parts: row.get(6)?,
        total_parts: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

pub fn upsert_job(
    conn: &Connection,
    file_id: i64,
    source_path: &str,
    state: &str,
    total_parts: i64,
) -> Result<i64> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO upload_jobs (file_id, source_path, state, error, error_code, done_parts, total_parts, created_at, updated_at)
         VALUES (?1, ?2, ?3, NULL, NULL, 0, ?4, ?5, ?5)
         ON CONFLICT(file_id) DO UPDATE SET
            source_path = excluded.source_path,
            state = excluded.state,
            total_parts = excluded.total_parts,
            updated_at = excluded.updated_at",
        params![file_id, source_path, state, total_parts, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_active_job_by_source_path(
    conn: &Connection,
    source_path: &str,
) -> Result<Option<UploadJob>> {
    conn.query_row(
        "SELECT id, file_id, source_path, state, error, error_code, done_parts, total_parts, created_at, updated_at
         FROM upload_jobs
         WHERE source_path = ? AND state IN ('queued', 'uploading', 'processing')
         ORDER BY id DESC
         LIMIT 1",
        params![source_path],
        map_job,
    )
    .optional()
}

pub fn get_job_by_file_id(conn: &Connection, file_id: i64) -> Result<Option<UploadJob>> {
    conn.query_row(
        "SELECT id, file_id, source_path, state, error, error_code, done_parts, total_parts, created_at, updated_at
         FROM upload_jobs
         WHERE file_id = ?",
        params![file_id],
        map_job,
    )
    .optional()
}

pub fn update_source_path(conn: &Connection, file_id: i64, source_path: &str) -> Result<()> {
    conn.execute(
        "UPDATE upload_jobs
         SET source_path = ?, updated_at = ?
         WHERE file_id = ?",
        params![source_path, Utc::now().timestamp(), file_id],
    )?;
    Ok(())
}

pub fn update_progress(
    conn: &Connection,
    file_id: i64,
    done_parts: i64,
    total_parts: i64,
) -> Result<()> {
    conn.execute(
        "UPDATE upload_jobs
         SET done_parts = ?, total_parts = ?, updated_at = ?
         WHERE file_id = ?",
        params![done_parts, total_parts, Utc::now().timestamp(), file_id],
    )?;
    Ok(())
}

pub fn update_state(
    conn: &Connection,
    file_id: i64,
    state: &str,
    error: Option<&str>,
    error_code: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE upload_jobs
         SET state = ?, error = ?, error_code = ?, updated_at = ?
         WHERE file_id = ?",
        params![state, error, error_code, Utc::now().timestamp(), file_id],
    )?;
    Ok(())
}

pub fn delete_job_by_file_id(conn: &Connection, file_id: i64) -> Result<()> {
    conn.execute("DELETE FROM upload_jobs WHERE file_id = ?", params![file_id])?;
    Ok(())
}
