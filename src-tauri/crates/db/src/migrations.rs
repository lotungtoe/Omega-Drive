use rusqlite::{Connection, OptionalExtension, Result};
use tracing::info;

const RESET_SQL: &str = r#"
PRAGMA foreign_keys = OFF;

DROP TRIGGER IF EXISTS files_fts_after_insert;
DROP TRIGGER IF EXISTS files_fts_after_delete;
DROP TRIGGER IF EXISTS files_fts_after_update_filename;

DROP TABLE IF EXISTS files_fts;
DROP TABLE IF EXISTS upload_jobs;
DROP TABLE IF EXISTS video_keyframes;
DROP TABLE IF EXISTS provider_quota_cache;
DROP TABLE IF EXISTS drive_stats_cache;
DROP INDEX IF EXISTS idx_files_folder_active;
DROP INDEX IF EXISTS idx_files_folder_status_id;
DROP INDEX IF EXISTS idx_files_folder_active_id;
DROP INDEX IF EXISTS idx_files_folder_deleted_id;
DROP INDEX IF EXISTS idx_files_deleted_id;
DROP TABLE IF EXISTS image_files;
DROP TABLE IF EXISTS audio_files;
DROP TABLE IF EXISTS video_files;
DROP TABLE IF EXISTS upload_profile_rules;
DROP TABLE IF EXISTS download_jobs;
DROP TABLE IF EXISTS upload_profiles;
DROP TABLE IF EXISTS parts;
DROP TABLE IF EXISTS files;
DROP TABLE IF EXISTS folders;
DROP TABLE IF EXISTS tenant_meta;

PRAGMA foreign_keys = ON;
"#;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS tenant_meta (
    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
    scope TEXT NOT NULL,
    discord_guild_id TEXT,
    telegram_group_id TEXT,
    display_name TEXT
);
INSERT OR IGNORE INTO tenant_meta (singleton, scope, discord_guild_id, telegram_group_id)
VALUES (1, 'my', NULL, NULL);

CREATE TABLE IF NOT EXISTS folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    parent_id INTEGER,
    starred BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY(parent_id) REFERENCES folders(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_folders_parent_name ON folders(parent_id, name);

CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    size INTEGER NOT NULL,
    thread_id TEXT NOT NULL UNIQUE,
    folder_id INTEGER,
    checksum TEXT,
    status TEXT NOT NULL DEFAULT 'uploading',
    is_hidden BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME,
    starred BOOLEAN NOT NULL DEFAULT 0,
    kind TEXT NOT NULL DEFAULT 'other',
    last_accessed_at INTEGER,
    FOREIGN KEY(folder_id) REFERENCES folders(id) ON DELETE SET NULL
);
CREATE INDEX IF NOT EXISTS idx_files_status_id ON files(status, id DESC);
CREATE INDEX IF NOT EXISTS idx_files_all_active_id ON files(id DESC) WHERE status != 'trashed';
CREATE INDEX IF NOT EXISTS idx_files_kind_status_id ON files(kind, status, id DESC);
CREATE INDEX IF NOT EXISTS idx_files_last_accessed_id ON files(last_accessed_at DESC, id DESC) WHERE last_accessed_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_files_folder_active ON files(folder_id, is_hidden, id DESC) WHERE status IN ('ready', 'error');

CREATE TABLE IF NOT EXISTS parts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    platform TEXT NOT NULL,
    message_id TEXT NOT NULL,
    part_index INTEGER NOT NULL,
    size INTEGER NOT NULL,
    checksum TEXT,
    FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_parts_file_partidx_id ON parts(file_id, part_index, id);

CREATE TABLE IF NOT EXISTS upload_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    plan_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS upload_profile_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    file_type TEXT,
    extensions TEXT,
    min_size_bytes INTEGER,
    max_size_bytes INTEGER,
    FOREIGN KEY(profile_id) REFERENCES upload_profiles(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_upload_profile_rules_profile_priority
    ON upload_profile_rules(profile_id, priority DESC, id ASC);

CREATE TABLE IF NOT EXISTS video_files (
    file_id INTEGER PRIMARY KEY,
    duration_sec REAL,
    resolution TEXT,
    fps REAL,
    bitrate_bps INTEGER,
    video_codec TEXT,
    audio_codec TEXT,
    container TEXT,
    resume_position_sec REAL,
    resume_part_index INTEGER,
    audio TEXT,
    default_audio INTEGER,
    FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY(default_audio) REFERENCES files(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS audio_files (
    file_id INTEGER PRIMARY KEY,
    duration_sec REAL,
    bitrate_bps INTEGER,
    sample_rate_hz INTEGER,
    channels INTEGER,
    audio_codec TEXT,
    container TEXT,
    FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS image_files (
    file_id INTEGER PRIMARY KEY,
    resolution TEXT,
    format TEXT,
    color_space TEXT,
    orientation TEXT,
    FOREIGN KEY(file_id) REFERENCES files(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS drive_stats_cache (
    drive_scope TEXT PRIMARY KEY,
    total_files INTEGER NOT NULL DEFAULT 0,
    total_folders INTEGER NOT NULL DEFAULT 0,
    total_size INTEGER NOT NULL DEFAULT 0,
    trash_count INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS provider_quota_cache (
    provider TEXT PRIMARY KEY,
    used_bytes INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
    filename,
    content='files',
    content_rowid='id',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS files_fts_after_insert AFTER INSERT ON files BEGIN
    INSERT INTO files_fts(rowid, filename) VALUES (new.id, new.filename);
END;
CREATE TRIGGER IF NOT EXISTS files_fts_after_delete AFTER DELETE ON files BEGIN
    INSERT INTO files_fts(files_fts, rowid, filename) VALUES ('delete', old.id, old.filename);
END;
CREATE TRIGGER IF NOT EXISTS files_fts_after_update_filename AFTER UPDATE OF filename ON files BEGIN
    INSERT INTO files_fts(files_fts, rowid, filename) VALUES ('delete', old.id, old.filename);
    INSERT INTO files_fts(rowid, filename) VALUES (new.id, new.filename);
END;
"#;

pub fn run_migrations(conn: &Connection) -> Result<()> {
    if schema_is_compatible(conn)? {
        info!("Database schema is up to date");
        return Ok(());
    }

    info!("Resetting SQLite schema");
    conn.execute_batch(RESET_SQL)?;
    conn.execute_batch(SCHEMA_SQL)?;
    Ok(())
}

fn schema_is_compatible(conn: &Connection) -> Result<bool> {
    Ok(table_exists(conn, "tenant_meta")?
        && table_exists(conn, "files")?
        && table_exists(conn, "folders")?
        && table_exists(conn, "parts")?)
}

fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?",
        [name],
        |_| Ok(true),
    )
    .optional()
    .map(|value| value.unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rusqlite::Connection;

    use super::run_migrations;

    const EXPECTED_TABLES: &[&str] = &[
        "tenant_meta",
        "folders",
        "files",
        "parts",
        "upload_profiles",
        "upload_profile_rules",
        "video_files",
        "audio_files",
        "image_files",
        "drive_stats_cache",
        "provider_quota_cache",
        "files_fts",
    ];

    fn open_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    #[test]
    fn run_migrations_creates_expected_tables() {
        let conn = open_conn();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type IN ('table', 'view')")
            .expect("prepare table query");
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query table names");
        let names: BTreeSet<String> = rows.map(|row| row.expect("read table name")).collect();

        for table in EXPECTED_TABLES {
            assert!(names.contains(*table), "missing expected table {table}");
        }
        let tenant_columns: Vec<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(tenant_meta)")
                .expect("prepare tenant columns");
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .expect("query tenant columns");
            rows.map(|row| row.expect("read column")).collect()
        };
        assert!(tenant_columns.iter().any(|column| column == "display_name"));
    }

    #[test]
    fn run_migrations_is_idempotent() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("first run");
        run_migrations(&conn).expect("second run");
    }

    #[test]
    fn run_migrations_resets_legacy_schema() {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "
            CREATE TABLE files (
                id INTEGER PRIMARY KEY,
                filename TEXT NOT NULL,
                drive_scope TEXT NOT NULL,
                local_path TEXT,
                role TEXT
            );
            INSERT INTO files (id, filename, drive_scope, local_path, role)
            VALUES (1, 'legacy.bin', 'my', 'C:\\legacy.bin', 'main');
            ",
        )
        .expect("create legacy schema");

        run_migrations(&conn).expect("reset schema");

        let columns: Vec<String> = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(files)")
                .expect("prepare file columns");
            let rows = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .expect("query file columns");
            rows.map(|row| row.expect("read column")).collect()
        };

        assert!(!columns.iter().any(|column| column == "drive_scope"));
        assert!(!columns.iter().any(|column| column == "local_path"));
        assert!(!columns.iter().any(|column| column == "role"));
        assert!(columns.iter().any(|column| column == "kind"));
    }
}
