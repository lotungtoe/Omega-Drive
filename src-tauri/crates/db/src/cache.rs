use rusqlite::{Connection, Result};
use std::path::Path;
use tracing::info;

pub fn open_cache_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    run_migrations(&conn)?;
    info!("Cache database opened at: {}", path.display());
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS upload_jobs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL UNIQUE,
            source_path TEXT NOT NULL,
            state       TEXT NOT NULL,
            error       TEXT,
            error_code  TEXT,
            done_parts  INTEGER NOT NULL DEFAULT 0,
            total_parts INTEGER NOT NULL DEFAULT 0,
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_upload_jobs_state_updated
            ON upload_jobs(state, updated_at);
        CREATE INDEX IF NOT EXISTS idx_upload_jobs_source_path
            ON upload_jobs(source_path);
        CREATE TABLE IF NOT EXISTS download_jobs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_id     INTEGER NOT NULL,
            target_path TEXT NOT NULL,
            state       TEXT NOT NULL,
            error       TEXT,
            error_code  TEXT,
            total_parts INTEGER NOT NULL DEFAULT 0,
            done_parts  INTEGER NOT NULL DEFAULT 0,
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_download_jobs_file_id
            ON download_jobs(file_id);
        CREATE INDEX IF NOT EXISTS idx_download_jobs_state_created
            ON download_jobs(state, created_at);
        CREATE INDEX IF NOT EXISTS idx_download_jobs_state_updated
            ON download_jobs(state, updated_at);",
    )?;
    Ok(())
}
