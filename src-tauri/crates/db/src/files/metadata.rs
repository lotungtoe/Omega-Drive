use rusqlite::{params, Connection, OptionalExtension, Result};

use crate::{drive_stats_cache, provider_quota_cache, upload_jobs};

use super::{ensure_media_child_row, infer_file_kind, set_file_kind, FileMetadata};

const FILE_SELECT_SHORT: &str = "SELECT \
    f.id, f.filename, f.size, f.thread_id, f.folder_id, \
    COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope, \
    f.checksum, f.status, f.starred, \
    f.created_at, f.deleted_at, NULL AS local_path, f.kind, NULL AS duration_sec, \
    f.is_hidden, f.last_accessed_at \
    FROM files f";

const FILE_SELECT_LONG: &str = "SELECT \
    f.id, f.filename, f.size, f.thread_id, f.folder_id, \
    COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope, \
    f.checksum, f.status, f.starred, \
    f.created_at, f.deleted_at, uj.source_path AS local_path, f.kind, vf.duration_sec, \
    f.is_hidden, f.last_accessed_at \
    FROM files f \
    LEFT JOIN video_files vf ON vf.file_id = f.id \
    LEFT JOIN upload_jobs uj ON uj.file_id = f.id";

fn map_file(row: &rusqlite::Row) -> rusqlite::Result<FileMetadata> {
    Ok(FileMetadata {
        id: row.get(0)?,
        filename: row.get(1)?,
        size: row.get(2)?,
        thread_id: row.get(3)?,
        folder_id: row.get(4)?,
        drive_scope: row.get(5)?,
        checksum: row.get(6)?,
        status: row.get(7)?,
        starred: row.get(8)?,
        created_at: row.get(9)?,
        deleted_at: row.get(10)?,
        local_path: row.get(11)?,
        kind: row.get(12)?,
        duration_sec: row.get(13)?,
        is_hidden: row.get::<_, i64>(14)? != 0,
        last_accessed_at: row.get(15)?,
    })
}

fn tokenize_search_query(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for ch in query.chars() {
        if ch.is_alphanumeric() {
            current.extend(ch.to_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn build_fts_prefix_query(query: &str) -> Option<String> {
    let tokens = tokenize_search_query(query);
    if tokens.is_empty() {
        None
    } else {
        Some(
            tokens
                .into_iter()
                .map(|token| format!("{token}*"))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }
}

fn recent_active_files_limited(conn: &Connection, limit: i64) -> Result<Vec<FileMetadata>> {
    let sql = format!(
        "{FILE_SELECT_SHORT} WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0 AND f.last_accessed_at IS NOT NULL AND f.last_accessed_at >= CAST(strftime('%s', 'now', '-3 days') AS INTEGER) ORDER BY f.last_accessed_at DESC, f.id DESC LIMIT ?"
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![limit], map_file)?;
    rows.collect()
}

pub fn get_recent_files_limited(
    conn: &Connection,
    limit: i64,
    role_filter: Option<&str>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let mut sql = format!(
        "{FILE_SELECT_SHORT} WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0 AND f.last_accessed_at IS NOT NULL AND f.last_accessed_at >= CAST(strftime('%s', 'now', '-3 days') AS INTEGER)"
    );
    sql.push_str(" ORDER BY f.last_accessed_at DESC, f.id DESC LIMIT ?");
    let mut stmt = conn.prepare_cached(&sql)?;
    let _ = (role_filter, drive_scope);
    let rows = stmt.query_map(params![limit], map_file)?;
    rows.collect()
}

pub fn get_recent_files_paginated(
    conn: &Connection,
    cursor: Option<i64>,
    limit: i64,
    role_filter: Option<&str>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut sql = format!(
        "{FILE_SELECT_SHORT} WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0 AND f.last_accessed_at IS NOT NULL AND f.last_accessed_at >= CAST(strftime('%s', 'now', '-3 days') AS INTEGER)"
    );

    if let Some(cur) = cursor {
        let cursor_accessed_at: Option<i64> = conn
            .query_row(
                "SELECT last_accessed_at FROM files WHERE id = ?",
                params![cur],
                |row| row.get(0),
            )
            .optional()?
            .flatten();

        if let Some(accessed_at) = cursor_accessed_at {
            sql.push_str(" AND (f.last_accessed_at < ? OR (f.last_accessed_at = ? AND f.id < ?))");
            params_vec.push(Box::new(accessed_at));
            params_vec.push(Box::new(accessed_at));
            params_vec.push(Box::new(cur));
        } else {
            return Ok(Vec::new());
        }
    }

    sql.push_str(" ORDER BY f.last_accessed_at DESC, f.id DESC LIMIT ?");
    params_vec.push(Box::new(limit));
    let _ = (role_filter, drive_scope);
    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), map_file)?;
    rows.collect()
}

pub fn insert_file(
    conn: &Connection,
    filename: &str,
    size: i64,
    thread_id: &str,
    folder_id: Option<i64>,
    drive_scope: &str,
    checksum: Option<&str>,
    local_path: Option<&str>,
) -> Result<i64> {
    let kind = infer_file_kind(filename);
    conn.execute(
        "INSERT INTO files (
            filename, size, thread_id, folder_id, checksum, status, is_hidden, starred, kind
         ) VALUES (?, ?, ?, ?, ?, 'uploading', 0, 0, ?)",
        params![filename, size, thread_id, folder_id, checksum, kind],
    )?;
    let file_id = conn.last_insert_rowid();
    if let Some(source_path) = local_path {
        upload_jobs::upsert_job(conn, file_id, source_path, "uploading", 0)?;
    }
    let _ = drive_scope;
    ensure_media_child_row(conn, file_id, kind)?;
    let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    Ok(file_id)
}

pub fn update_file_checksum(conn: &Connection, file_id: i64, checksum: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET checksum = ? WHERE id = ?",
        params![checksum, file_id],
    )?;
    Ok(())
}

pub fn insert_attachment_file(
    conn: &Connection,
    filename: &str,
    size: i64,
    thread_id: &str,
    folder_id: Option<i64>,
    drive_scope: &str,
    checksum: Option<&str>,
    local_path: Option<&str>,
) -> Result<i64> {
    insert_file(conn, filename, size, thread_id, folder_id, drive_scope, checksum, local_path)
}

pub fn get_file_by_id(conn: &Connection, id: i64) -> Result<Option<FileMetadata>> {
    let sql = format!("{FILE_SELECT_LONG} WHERE f.id = ?");
    conn.query_row(&sql, params![id], map_file).optional()
}

pub fn get_all_files(conn: &Connection) -> Result<Vec<FileMetadata>> {
    let sql = format!("{FILE_SELECT_SHORT} ORDER BY f.id DESC LIMIT 1000");
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map([], map_file)?;
    rows.collect()
}

pub fn get_all_file_count(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM files WHERE status != 'trashed' AND is_hidden = 0",
        [],
        |row| row.get(0),
    )
}

pub fn get_file_by_thread_id(conn: &Connection, thread_id: &str) -> Result<Option<FileMetadata>> {
    let sql = format!("{FILE_SELECT_LONG} WHERE f.thread_id = ?");
    conn.query_row(&sql, params![thread_id], map_file)
        .optional()
}

pub fn get_files_by_parent(
    conn: &Connection,
    folder_id: Option<i64>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let _ = drive_scope;
    if let Some(fid) = folder_id {
        let mut sql =
            format!("{FILE_SELECT_SHORT} WHERE f.folder_id = ? AND f.status IN ('ready', 'error') AND f.is_hidden = 0");
        sql.push_str(" ORDER BY f.id DESC");
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![fid], map_file)?;
        rows.collect()
    } else {
        let mut sql = format!(
            "{FILE_SELECT_SHORT} WHERE f.folder_id IS NULL AND f.status IN ('ready', 'error') AND f.is_hidden = 0"
        );
        sql.push_str(" ORDER BY f.id DESC");
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map([], map_file)?;
        rows.collect()
    }
}

pub fn get_file_by_name(
    conn: &Connection,
    name: &str,
    folder_id: Option<i64>,
) -> Result<Option<FileMetadata>> {
    if let Some(fid) = folder_id {
        let sql = format!(
            "{FILE_SELECT_SHORT} WHERE f.filename = ? AND f.folder_id = ? AND f.status IN ('ready', 'error')"
        );
        conn.query_row(&sql, params![name, fid], map_file)
            .optional()
    } else {
        let sql = format!(
            "{FILE_SELECT_SHORT} WHERE f.filename = ? AND f.folder_id IS NULL AND f.status IN ('ready', 'error')"
        );
        conn.query_row(&sql, params![name], map_file).optional()
    }
}

pub fn update_file_status(conn: &Connection, id: i64, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET status = ? WHERE id = ?",
        params![status, id],
    )?;
    let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    Ok(())
}

pub fn move_file_to_trash(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE files SET status = 'trashed', deleted_at = CURRENT_TIMESTAMP WHERE id = ?",
        params![id],
    )?;
    let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    Ok(())
}

pub fn restore_trashed_file(conn: &Connection, id: i64) -> Result<bool> {
    let changed = conn.execute(
        "UPDATE files SET status = 'ready', deleted_at = NULL WHERE id = ? AND status = 'trashed'",
        params![id],
    )?;
    if changed > 0 {
        let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    }
    Ok(changed > 0)
}

pub fn mark_file_accessed(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE files SET last_accessed_at = CAST(strftime('%s', 'now') AS INTEGER) WHERE id = ?",
        params![id],
    )?;
    Ok(())
}

pub fn update_file_name(conn: &Connection, id: i64, filename: &str) -> Result<()> {
    conn.execute(
        "UPDATE files SET filename = ? WHERE id = ?",
        params![filename, id],
    )?;
    let kind = infer_file_kind(filename);
    set_file_kind(conn, id, kind)?;
    Ok(())
}

pub fn update_file_folder(conn: &Connection, id: i64, folder_id: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE files SET folder_id = ? WHERE id = ?",
        params![folder_id, id],
    )?;
    Ok(())
}

pub fn update_file_local_path(conn: &Connection, id: i64, local_path: Option<&str>) -> Result<()> {
    if let Some(path) = local_path {
        upload_jobs::update_source_path(conn, id, path)?;
    } else {
        upload_jobs::delete_job_by_file_id(conn, id)?;
    }
    Ok(())
}

pub fn delete_file(conn: &Connection, id: i64) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT platform, COALESCE(SUM(size), 0) FROM parts WHERE file_id = ? GROUP BY platform",
    )?;
    let deleted_sizes = stmt
        .query_map(params![id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>>>()?;
    conn.execute("DELETE FROM files WHERE id = ?", params![id])?;
    for (platform, size) in deleted_sizes {
        let _ = provider_quota_cache::adjust_provider_quota_cache(conn, &platform, -size);
    }
    let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    Ok(())
}

pub fn toggle_file_hidden(conn: &Connection, id: i64, is_hidden: bool) -> Result<()> {
    conn.execute(
        "UPDATE files SET is_hidden = ? WHERE id = ?",
        params![is_hidden, id],
    )?;
    Ok(())
}

pub fn toggle_file_star(conn: &Connection, id: i64, starred: bool) -> Result<()> {
    conn.execute(
        "UPDATE files SET starred = ? WHERE id = ?",
        params![starred, id],
    )?;
    Ok(())
}

pub fn get_files_paginated(
    conn: &Connection,
    folder_id: Option<i64>,
    cursor: Option<i64>,
    limit: i64,
    role_filter: Option<&str>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut sql = format!("{FILE_SELECT_SHORT} WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0");
    let _ = (role_filter, drive_scope);

    if let Some(fid) = folder_id {
        sql.push_str(" AND f.folder_id = ?");
        params_vec.push(Box::new(fid));
    } else {
        sql.push_str(" AND f.folder_id IS NULL");
    }

    if let Some(cur) = cursor {
        sql.push_str(" AND f.id < ?");
        params_vec.push(Box::new(cur));
    }

    sql.push_str(" ORDER BY f.id DESC LIMIT ?");
    params_vec.push(Box::new(limit));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), map_file)?;
    rows.collect()
}

pub fn get_all_files_paginated(
    conn: &Connection,
    cursor: Option<i64>,
    limit: i64,
    role_filter: Option<&str>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut sql = format!("{FILE_SELECT_SHORT} WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0");
    let _ = (role_filter, drive_scope);

    if let Some(cur) = cursor {
        sql.push_str(" AND f.id < ?");
        params_vec.push(Box::new(cur));
    }

    sql.push_str(" ORDER BY f.id DESC LIMIT ?");
    params_vec.push(Box::new(limit));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), map_file)?;
    rows.collect()
}

pub fn get_trash_paginated(
    conn: &Connection,
    cursor: Option<i64>,
    limit: i64,
    role_filter: Option<&str>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut sql = format!("{FILE_SELECT_SHORT} WHERE f.status = 'trashed'");
    let _ = (role_filter, drive_scope);

    if let Some(cur) = cursor {
        sql.push_str(" AND f.id < ?");
        params_vec.push(Box::new(cur));
    }

    sql.push_str(" ORDER BY f.id DESC LIMIT ?");
    params_vec.push(Box::new(limit));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), map_file)?;
    rows.collect()
}

pub fn get_transfers_paginated(
    conn: &Connection,
    cursor: Option<i64>,
    limit: i64,
) -> Result<Vec<FileMetadata>> {
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut sql = format!("{FILE_SELECT_SHORT} WHERE f.status IN ('uploading', 'processing')");

    if let Some(cur) = cursor {
        sql.push_str(" AND f.id < ?");
        params_vec.push(Box::new(cur));
    }

    sql.push_str(" ORDER BY f.id DESC LIMIT ?");
    params_vec.push(Box::new(limit));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params_refs.as_slice(), map_file)?;
    rows.collect()
}

pub fn get_file_stats(conn: &Connection, drive_scope: Option<&str>) -> Result<(i64, i64, i64)> {
    let sql = String::from(
        "SELECT \
           COUNT(CASE WHEN status IN ('ready', 'error') AND is_hidden = 0 THEN 1 END), \
           COALESCE(SUM(CASE WHEN status IN ('ready', 'error') AND is_hidden = 0 THEN size END), 0), \
           COUNT(CASE WHEN status = 'trashed' THEN 1 END) \
         FROM files",
    );
    let _ = drive_scope;
    conn.query_row(&sql, [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
}

pub fn get_files_with_part_type_paginated(
    conn: &Connection,
    part_type: &str,
    cursor: Option<i64>,
    limit: i64,
    role_filter: Option<&str>,
    drive_scope: Option<&str>,
) -> Result<Vec<FileMetadata>> {
    let _ = (role_filter, drive_scope);
    if part_type != "chunk" {
        return Ok(Vec::new());
    }
    get_all_files_paginated(conn, cursor, limit, None, None)
}

pub fn search_files_limited(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<FileMetadata>> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return recent_active_files_limited(conn, limit);
    }

    if let Some(fts_query) = build_fts_prefix_query(trimmed) {
        let sql = format!(
            "{FILE_SELECT_SHORT} \
             JOIN files_fts ON files_fts.rowid = f.id \
             WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0 AND files_fts MATCH ? \
             ORDER BY bm25(files_fts), f.id DESC LIMIT ?"
        );
        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params![fts_query, limit], map_file)?;
        let matches: Vec<FileMetadata> = rows.collect::<Result<Vec<_>>>()?;
        if !matches.is_empty() {
            return Ok(matches);
        }
    }

    let pattern = format!("%{}%", query);
    let sql = format!(
        "{FILE_SELECT_SHORT} WHERE f.status IN ('ready', 'error') AND f.is_hidden = 0 AND f.filename LIKE ? ORDER BY f.id DESC LIMIT ?"
    );
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(params![pattern, limit], map_file)?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{
        get_file_by_id, insert_file, mark_file_accessed, search_files_limited,
        update_file_checksum, update_file_name, update_file_status,
    };
    use crate::migrations::run_migrations;

    fn open_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    #[test]
    fn search_files_uses_fts_and_falls_back_to_substring_matching() {
        let conn = open_conn();

        let phoenix_id = insert_file(
            &conn,
            "Project Phoenix Final Cut.mkv",
            10,
            "channel-1",
            None,
            "my",
            None,
            None,
        )
        .expect("insert phoenix");
        update_file_status(&conn, phoenix_id, "ready").expect("mark phoenix ready");

        let notes_id = insert_file(
            &conn,
            "Project Notes.txt",
            10,
            "channel-2",
            None,
            "my",
            None,
            None,
        )
        .expect("insert notes");
        update_file_status(&conn, notes_id, "ready").expect("mark notes ready");
        mark_file_accessed(&conn, phoenix_id).expect("mark phoenix accessed");
        mark_file_accessed(&conn, notes_id).expect("mark notes accessed");

        let trashed_id = insert_file(
            &conn,
            "Phoenix Trash.tmp",
            10,
            "channel-3",
            None,
            "my",
            None,
            None,
        )
        .expect("insert trashed");
        update_file_status(&conn, trashed_id, "trashed").expect("trash file");

        let fts_matches = search_files_limited(&conn, "phoenix final", 10).expect("fts search");
        assert_eq!(fts_matches.len(), 1);
        assert_eq!(fts_matches[0].id, phoenix_id);

        let fallback_matches =
            search_files_limited(&conn, "enix Final", 10).expect("fallback search");
        assert_eq!(fallback_matches.len(), 1);
        assert_eq!(fallback_matches[0].id, phoenix_id);

        update_file_name(&conn, notes_id, "Roadmap Alpha.txt").expect("rename file");
        let renamed_matches =
            search_files_limited(&conn, "roadmap", 10).expect("search renamed file");
        assert_eq!(renamed_matches.len(), 1);
        assert_eq!(renamed_matches[0].id, notes_id);

        let recent_matches = search_files_limited(&conn, "   ", 10).expect("blank search");
        assert_eq!(recent_matches.len(), 2);
    }

    #[test]
    fn update_file_checksum_persists_final_hash() {
        let conn = open_conn();

        let file_id = insert_file(
            &conn,
            "archive.bin",
            10,
            "channel-1",
            None,
            "my",
            None,
            None,
        )
        .expect("insert file");

        update_file_checksum(&conn, file_id, "final-hash").expect("update checksum");

        let file = get_file_by_id(&conn, file_id)
            .expect("load file")
            .expect("file exists");
        assert_eq!(file.checksum.as_deref(), Some("final-hash"));
    }
}

