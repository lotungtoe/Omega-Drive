use std::path::PathBuf;

use rusqlite::Connection;

use omega_drive_gateway::core::backup::{FilePayload, Op};
use crate::backup_repo::BackupRepository;

use crate::DbWriteQueue;

impl BackupRepository for DbWriteQueue {
    fn capture_file_state(
        &self,
        conn: &Connection,
        file_id: i64,
    ) -> Result<FilePayload, rusqlite::Error> {
        capture_file_state(conn, file_id)
    }
}

pub fn capture_file_state(
    conn: &Connection,
    file_id: i64,
) -> Result<FilePayload, rusqlite::Error> {
    let files = query_rows_json(conn, "files", "id = ?1", &[&file_id])?;
    let parts = query_rows_json(conn, "parts", "file_id = ?1", &[&file_id])?;
    let video_files = query_rows_json(conn, "video_files", "file_id = ?1", &[&file_id])?;
    let audio_files = query_rows_json(conn, "audio_files", "file_id = ?1", &[&file_id])?;
    let image_files = query_rows_json(conn, "image_files", "file_id = ?1", &[&file_id])?;

    Ok(FilePayload {
        file: files.into_iter().next(),
        parts,
        video_file: video_files.into_iter().next(),
        audio_file: audio_files.into_iter().next(),
        image_files,
    })
}

fn query_rows_json(
    conn: &Connection,
    table: &str,
    where_clause: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<serde_json::Value>, rusqlite::Error> {
    let sql = format!("SELECT * FROM {table} WHERE {where_clause}");
    let mut stmt = conn.prepare(&sql)?;
    let col_names: Vec<String> = stmt.column_names().iter().map(|c| c.to_string()).collect();
    let rows = stmt.query_map(params, |row| {
        let mut map = serde_json::Map::new();
        for (i, name) in col_names.iter().enumerate() {
            let val = match row.get_ref(i)? {
                rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                rusqlite::types::ValueRef::Integer(v) => serde_json::json!(v),
                rusqlite::types::ValueRef::Real(v) => serde_json::json!(v),
                rusqlite::types::ValueRef::Text(v) => {
                    serde_json::Value::String(String::from_utf8_lossy(v).into_owned())
                }
                rusqlite::types::ValueRef::Blob(v) => serde_json::Value::Array(
                    v.iter().map(|b| serde_json::Number::from(*b).into()).collect(),
                ),
            };
            map.insert(name.clone(), val);
        }
        Ok(serde_json::Value::Object(map))
    })?;
    rows.collect()
}

pub fn create_snapshot(conn: &Connection, base_dir: &PathBuf) -> Result<Vec<(Vec<u8>, String)>, String> {
    let snapshot_path = base_dir.join(format!("backup_snapshot_{}.db", chrono::Utc::now().format("%Y%m%d_%H%M%S")));
    conn.execute_batch(&format!("VACUUM INTO '{}'", snapshot_path.to_str().expect("snapshot_path must be UTF-8")))
        .map_err(|e| format!("VACUUM INTO failed: {e}"))?;

    let data = std::fs::read(&snapshot_path).map_err(|e| format!("read snapshot failed: {e}"))?;
    let _ = std::fs::remove_file(&snapshot_path);

    let compressed = zstd::encode_all(&data[..], 3)
        .map_err(|e| format!("zstd compression failed: {e}"))?;

    let max_chunk: usize = 10 * 1024 * 1024;
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");

    if compressed.len() <= max_chunk {
        Ok(vec![(compressed, format!("backup_snapshot_{ts}.zst"))])
    } else {
        let chunks: Vec<(Vec<u8>, String)> = compressed.chunks(max_chunk).enumerate().map(|(i, chunk)| {
            (chunk.to_vec(), format!("backup_snapshot_{ts}.part{:04}.zst", i + 1))
        }).collect();
        Ok(chunks)
    }
}

pub fn apply_op(conn: &Connection, op: &Op) -> Result<(), String> {
    match op {
        Op::FileSnapshot { file_id, payload, .. } => {
            if let Some(ref file_val) = payload.file {
                let cols = json_obj_to_insert("files", file_val);
                conn.execute_batch(&cols).map_err(|e| format!("INSERT file {file_id}: {e}"))?;
            }
            for part in &payload.parts {
                let cols = json_obj_to_insert("parts", part);
                conn.execute_batch(&cols).map_err(|e| format!("INSERT part: {e}"))?;
            }
            if let Some(ref vf) = payload.video_file {
                let cols = json_obj_to_insert("video_files", vf);
                conn.execute_batch(&cols).map_err(|e| format!("INSERT video_file: {e}"))?;
            }
            if let Some(ref af) = payload.audio_file {
                let cols = json_obj_to_insert("audio_files", af);
                conn.execute_batch(&cols).map_err(|e| format!("INSERT audio_file: {e}"))?;
            }
            for img in &payload.image_files {
                let cols = json_obj_to_insert("image_files", img);
                conn.execute_batch(&cols).map_err(|e| format!("INSERT image_file: {e}"))?;
            }
        }
        Op::Mutation { table, action, row_id, .. } => {
            match action.as_str() {
                "delete" => {
                    conn.execute(&format!("DELETE FROM {table} WHERE id = ?1"), [*row_id])
                        .map_err(|e| format!("DELETE {table} {row_id}: {e}"))?;
                }
                _ => {
                    tracing::warn!("Backup restore: skipping Mutation {action} {table} {row_id} (no data captured)");
                }
            }
        }
    }
    Ok(())
}

fn json_obj_to_insert(table: &str, val: &serde_json::Value) -> String {
    let obj = match val {
        serde_json::Value::Object(m) => m,
        _ => return format!("-- cannot parse JSON for {table}"),
    };
    let cols: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    let placeholders: Vec<String> = obj.values().map(|v| match v {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        serde_json::Value::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            let json_str = serde_json::to_string(v).unwrap_or_default();
            format!("'{}'", json_str.replace('\'', "''"))
        }
    }).collect();
    let cols_str = cols.join(", ");
    let vals_str = placeholders.join(", ");
    format!("INSERT OR REPLACE INTO {table} ({cols_str}) VALUES ({vals_str});")
}
