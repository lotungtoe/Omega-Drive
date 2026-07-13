use chrono::Utc;
use rusqlite::{params, Connection, Result};

pub use omega_drive_gateway::core::data::DownloadJob;

pub fn create_job(
    conn: &Connection,
    file_id: i64,
    target_path: &str,
    total_parts: i64,
) -> Result<i64> {
    let now = Utc::now().timestamp();
    conn.execute(
        "INSERT INTO download_jobs (file_id, target_path, state, total_parts, done_parts, created_at, updated_at)
         VALUES (?1, ?2, 'queued', ?3, 0, ?4, ?4)",
        params![file_id, target_path, total_parts, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_progress(conn: &Connection, id: i64, done_parts: i64) -> Result<()> {
    conn.execute(
        "UPDATE download_jobs SET done_parts = ?, updated_at = ? WHERE id = ?",
        params![done_parts, Utc::now().timestamp(), id],
    )?;
    Ok(())
}

pub fn update_state(
    conn: &Connection,
    id: i64,
    state: &str,
    error: Option<&str>,
    error_code: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE download_jobs
         SET state = ?1, error = ?2, error_code = ?3, updated_at = ?4
         WHERE id = ?5",
        params![state, error, error_code, Utc::now().timestamp(), id],
    )?;
    Ok(())
}

pub fn get_job(conn: &Connection, id: i64) -> Result<Option<DownloadJob>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_id, target_path, state, error, error_code, total_parts, done_parts, created_at, updated_at
         FROM download_jobs WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    if let Some(row) = rows.next()? {
        Ok(Some(DownloadJob {
            id: row.get(0)?,
            file_id: row.get(1)?,
            target_path: row.get(2)?,
            state: row.get(3)?,
            error: row.get(4)?,
            error_code: row.get(5)?,
            total_parts: row.get(6)?,
            done_parts: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn list_jobs_by_state(conn: &Connection, states: &[&str]) -> Result<Vec<DownloadJob>> {
    if states.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = states.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id, file_id, target_path, state, error, error_code, total_parts, done_parts, created_at, updated_at
         FROM download_jobs
         WHERE state IN ({})
         ORDER BY created_at ASC",
        placeholders
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(rusqlite::params_from_iter(states.iter()), |row| {
        Ok(DownloadJob {
            id: row.get(0)?,
            file_id: row.get(1)?,
            target_path: row.get(2)?,
            state: row.get(3)?,
            error: row.get(4)?,
            error_code: row.get(5)?,
            total_parts: row.get(6)?,
            done_parts: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    let mut results = Vec::new();
    for r in rows {
        results.push(r?);
    }
    Ok(results)
}

pub fn get_next_queued(conn: &Connection) -> Result<Option<DownloadJob>> {
    let mut stmt = conn.prepare(
        "SELECT id, file_id, target_path, state, error, error_code, total_parts, done_parts, created_at, updated_at
         FROM download_jobs
         WHERE state = 'queued'
         ORDER BY created_at ASC
         LIMIT 1",
    )?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some(DownloadJob {
            id: row.get(0)?,
            file_id: row.get(1)?,
            target_path: row.get(2)?,
            state: row.get(3)?,
            error: row.get(4)?,
            error_code: row.get(5)?,
            total_parts: row.get(6)?,
            done_parts: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn exists_active_job_for_file(conn: &Connection, file_id: i64) -> Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT 1 FROM download_jobs
         WHERE file_id = ?1 AND state IN ('queued', 'downloading')
         LIMIT 1",
    )?;
    let mut rows = stmt.query(params![file_id])?;
    Ok(rows.next()?.is_some())
}

pub fn delete_job(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM download_jobs WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn pause_all_active_jobs(conn: &Connection, error_code: &str) -> Result<()> {
    conn.execute(
        "UPDATE download_jobs
         SET state = 'paused', error_code = ?1, updated_at = ?2
         WHERE state IN ('queued', 'downloading')",
        params![error_code, Utc::now().timestamp()],
    )?;
    Ok(())
}

pub fn resume_shutdown_jobs(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE download_jobs
         SET state = 'queued', error_code = NULL, updated_at = ?1
         WHERE state = 'paused' AND error_code = 'shutdown'",
        params![Utc::now().timestamp()],
    )?;
    Ok(())
}

pub fn purge_old_jobs(conn: &Connection, days: i64, states: &[&str]) -> Result<usize> {
    if states.is_empty() {
        return Ok(0);
    }
    let placeholders = states.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "DELETE FROM download_jobs
         WHERE state IN ({})
           AND updated_at < ?",
        placeholders
    );
    let mut params_vec: Vec<rusqlite::types::Value> = Vec::with_capacity(states.len() + 1);
    for s in states {
        params_vec.push((*s).to_string().into());
    }
    let cutoff = Utc::now().timestamp() - days.saturating_mul(86_400);
    params_vec.push(cutoff.into());
    let params = rusqlite::params_from_iter(params_vec);
    let affected = conn.execute(&sql, params)?;
    Ok(affected)
}
