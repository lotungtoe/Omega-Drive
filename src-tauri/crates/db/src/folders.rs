//! Folder metadata management (Discord Categories) in SQLite.
//!
//! Functions for manipulating the folders table.
//! Folders in this app map to Discord Category channels.

use rusqlite::{params, Connection, OptionalExtension, Result};

pub use omega_drive_gateway::core::data::FolderMetadata;

use crate::drive_stats_cache;

/// Convert a SQL result row into a FolderMetadata struct.
fn map_folder(row: &rusqlite::Row) -> rusqlite::Result<FolderMetadata> {
    Ok(FolderMetadata {
        id: row.get(0)?,
        name: row.get(1)?,
        parent_id: row.get(2)?,
        starred: row.get(3)?,
        drive_scope: row.get(4)?,
    })
}

/// Insert a new folder into the database.
pub fn insert_folder(
    conn: &Connection,
    name: &str,
    parent_id: Option<i64>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO folders (name, parent_id, starred) VALUES (?, ?, 0)",
        params![name, parent_id],
    )?;
    let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    Ok(conn.last_insert_rowid())
}

/// Get folder metadata by ID.
pub fn get_folder_by_id(conn: &Connection, id: i64) -> Result<Option<FolderMetadata>> {
    conn.query_row(
        "SELECT id, name, parent_id, starred,
            COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope
         FROM folders WHERE id = ?",
        params![id],
        map_folder,
    )
    .optional()
}

/// List all folders.
pub fn get_all_folders(
    conn: &Connection,
    drive_scope: Option<&str>,
) -> Result<Vec<FolderMetadata>> {
    let sql = String::from(
        "SELECT id, name, parent_id, starred,
            COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope
         FROM folders",
    );
    let _ = drive_scope;
    let mut stmt = conn.prepare(&sql)?;
    let x = stmt.query_map([], map_folder)?.collect();
    x
}

/// List children of a parent folder.
pub fn get_folders_by_parent(
    conn: &Connection,
    parent_id: Option<i64>,
    drive_scope: Option<&str>,
) -> Result<Vec<FolderMetadata>> {
    let _ = drive_scope;
    if let Some(pid) = parent_id {
        let sql =
            String::from("SELECT id, name, parent_id, starred, COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope FROM folders WHERE parent_id = ?");
        let mut stmt = conn.prepare(&sql)?;
        let x = stmt.query_map(params![pid], map_folder)?.collect();
        x
    } else {
        let sql =
            String::from("SELECT id, name, parent_id, starred, COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope FROM folders WHERE parent_id IS NULL");
        let mut stmt = conn.prepare(&sql)?;
        let x = stmt.query_map([], map_folder)?.collect();
        x
    }
}

/// Search folders by name under a given parent.
pub fn get_folder_by_name(
    conn: &Connection,
    name: &str,
    parent_id: Option<i64>,
    drive_scope: Option<&str>,
) -> Result<Option<FolderMetadata>> {
    let _ = drive_scope;
    if let Some(pid) = parent_id {
        conn.query_row(
            "SELECT id, name, parent_id, starred, COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope FROM folders WHERE name = ? AND parent_id = ?",
            params![name, pid], map_folder,
        ).optional()
    } else {
        conn.query_row(
            "SELECT id, name, parent_id, starred, COALESCE((SELECT scope FROM tenant_meta WHERE singleton = 1), 'my') AS drive_scope FROM folders WHERE name = ? AND parent_id IS NULL",
            params![name], map_folder,
        ).optional()
    }
}

/// Rename a folder.
pub fn update_folder_name(conn: &Connection, id: i64, name: &str) -> Result<()> {
    conn.execute(
        "UPDATE folders SET name = ? WHERE id = ?",
        params![name, id],
    )?;
    Ok(())
}

/// Move a folder to a new parent.
pub fn update_folder_parent(conn: &Connection, id: i64, parent_id: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE folders SET parent_id = ? WHERE id = ?",
        params![parent_id, id],
    )?;
    Ok(())
}

/// Delete a folder from the database.
pub fn delete_folder(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM folders WHERE id = ?", params![id])?;
    let _ = drive_stats_cache::refresh_drive_stats_cache(conn);
    Ok(())
}

/// Toggle the starred flag on a folder.
pub fn toggle_folder_star(conn: &Connection, id: i64, starred: bool) -> Result<()> {
    conn.execute(
        "UPDATE folders SET starred = ? WHERE id = ?",
        params![starred, id],
    )?;
    Ok(())
}
