use rusqlite::{params, Connection, OptionalExtension, Result};

use omega_drive_gateway::core::file_types::FileType;
use omega_drive_gateway::upload::upload_rules::UploadProfileRule;

use crate::services;

fn map_rule(row: &rusqlite::Row) -> rusqlite::Result<UploadProfileRule> {
    let file_type_text: Option<String> = row.get(3)?;
    let file_type = file_type_text.and_then(|t| match t.as_str() {
        "video" => Some(FileType::Video),
        "image" => Some(FileType::Image),
        "audio" => Some(FileType::Audio),
        "document" => Some(FileType::Document),
        "archive" => Some(FileType::Archive),
        "code" => Some(FileType::Code),
        "sheet" | "spreadsheet" => Some(FileType::Sheet),
        "other" | "unknown" => Some(FileType::Unknown),
        _ => None,
    });

    let ext_text: Option<String> = row.get(4)?;
    let mut extensions = Vec::new();
    if let Some(text) = ext_text {
        for part in text.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                extensions.push(trimmed.to_string());
            }
        }
    }

    Ok(UploadProfileRule {
        id: Some(row.get(0)?),
        profile_id: row.get(1)?,
        priority: row.get(2)?,
        file_type,
        extensions,
        min_size_bytes: row.get(5)?,
        max_size_bytes: row.get(6)?,
    })
}

fn file_type_to_text(file_type: Option<FileType>) -> Option<String> {
    file_type.map(|t| match t {
        FileType::Video => "video".to_string(),
        FileType::Image => "image".to_string(),
        FileType::Audio => "audio".to_string(),
        FileType::Document => "document".to_string(),
        FileType::Archive => "archive".to_string(),
        FileType::Code => "code".to_string(),
        FileType::Sheet => "sheet".to_string(),
        FileType::Unknown => "unknown".to_string(),
    })
}

pub fn get_upload_profile_rules(
    conn: &Connection,
    profile_id: Option<i64>,
) -> Result<Vec<UploadProfileRule>> {
    let mut rules = Vec::new();
    if let Some(pid) = profile_id {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, priority, file_type, extensions, min_size_bytes, max_size_bytes
             FROM upload_profile_rules
             WHERE profile_id = ?
             ORDER BY priority DESC, id ASC",
        )?;
        let rows = stmt.query_map(params![pid], map_rule)?;
        for row in rows {
            rules.push(row?);
        }
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, profile_id, priority, file_type, extensions, min_size_bytes, max_size_bytes
             FROM upload_profile_rules
             ORDER BY priority DESC, id ASC",
        )?;
        let rows = stmt.query_map([], map_rule)?;
        for row in rows {
            rules.push(row?);
        }
    }
    Ok(rules)
}

pub fn get_rule_by_id(conn: &Connection, id: i64) -> Result<Option<UploadProfileRule>> {
    conn.query_row(
        "SELECT id, profile_id, priority, file_type, extensions, min_size_bytes, max_size_bytes
         FROM upload_profile_rules WHERE id = ?",
        params![id],
        map_rule,
    )
    .optional()
}

pub fn save_upload_profile_rule(
    conn: &Connection,
    rule: &UploadProfileRule,
) -> Result<UploadProfileRule> {
    let ext_list = services::ext_normalizer().normalize_extensions(&rule.extensions);
    let ext_text = if ext_list.is_empty() {
        None
    } else {
        Some(ext_list.join(","))
    };
    let file_type_text = file_type_to_text(rule.file_type);

    if let Some(id) = rule.id {
        conn.execute(
            "UPDATE upload_profile_rules
             SET profile_id = ?, priority = ?, file_type = ?, extensions = ?, min_size_bytes = ?, max_size_bytes = ?
             WHERE id = ?",
            params![
                rule.profile_id,
                rule.priority,
                file_type_text,
                ext_text,
                rule.min_size_bytes,
                rule.max_size_bytes,
                id
            ],
        )?;
        get_rule_by_id(conn, id).map(|opt| opt.unwrap_or_else(|| rule.clone()))
    } else {
        conn.execute(
            "INSERT INTO upload_profile_rules (profile_id, priority, file_type, extensions, min_size_bytes, max_size_bytes)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                rule.profile_id,
                rule.priority,
                file_type_text,
                ext_text,
                rule.min_size_bytes,
                rule.max_size_bytes,
            ],
        )?;
        let id = conn.last_insert_rowid();
        get_rule_by_id(conn, id).map(|opt| {
            opt.unwrap_or_else(|| UploadProfileRule {
                id: Some(id),
                extensions: ext_list,
                ..rule.clone()
            })
        })
    }
}

pub fn delete_upload_profile_rule(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM upload_profile_rules WHERE id = ?", params![id])?;
    Ok(())
}

pub fn save_upload_profile_rules_bulk(
    conn: &Connection,
    profile_id: i64,
    ordered_rule_ids: &[i64],
) -> Result<Vec<UploadProfileRule>> {
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let total = ordered_rule_ids.len() as i64;
    let result = (|| {
        for (idx, rule_id) in ordered_rule_ids.iter().enumerate() {
            let priority = total - idx as i64 - 1;
            conn.execute(
                "UPDATE upload_profile_rules SET priority = ?
                 WHERE id = ? AND profile_id = ?",
                params![priority, rule_id, profile_id],
            )?;
        }
        Ok(())
    })();

    match result {
        Ok(()) => {
            conn.execute_batch("COMMIT")?;
            get_upload_profile_rules(conn, Some(profile_id))
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

