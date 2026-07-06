use rusqlite::{params, Connection, OptionalExtension, Result};

use omega_drive_gateway::upload::upload_plan::{UploadPlan, UploadProfile};

use crate::services;

fn map_profile(row: &rusqlite::Row) -> rusqlite::Result<UploadProfile> {
    let plan_json: String = row.get(2)?;
    let mut plan: UploadPlan = serde_json::from_str(&plan_json).unwrap_or_else(|e| {
        tracing::error!("Failed to parse UploadPlan JSON: {}. Using default.", e);
        UploadPlan::default()
    });
    plan.apply_defaults();
    Ok(UploadProfile {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        plan,
    })
}

pub fn get_upload_profiles(conn: &Connection) -> Result<Vec<UploadProfile>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, plan_json
         FROM upload_profiles
         ORDER BY id ASC",
    )?;
    let rows = stmt.query_map([], map_profile)?;
    let profiles: Vec<UploadProfile> = rows.collect::<Result<Vec<_>>>()?;

    if profiles.is_empty() {
        tracing::info!("No upload profiles found. Restoring defaults.");
        return restore_default_profiles(conn);
    }

    Ok(profiles)
}

pub fn save_upload_profile(conn: &Connection, profile: &UploadProfile) -> Result<UploadProfile> {
    let mut normalized_profile = profile.clone();
    normalized_profile.plan.apply_defaults();

    let plan_json = serde_json::to_string(&normalized_profile.plan)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    if let Some(id) = normalized_profile.id {
        conn.execute(
            "UPDATE upload_profiles
             SET name = ?, plan_json = ?
             WHERE id = ?",
            params![
                normalized_profile.name,
                plan_json,
                id
            ],
        )?;
        get_profile_by_id(conn, id).map(|opt| opt.unwrap_or(normalized_profile))
    } else {
        conn.execute(
            "INSERT INTO upload_profiles (name, plan_json)
             VALUES (?, ?)",
            params![
                normalized_profile.name,
                plan_json,
            ],
        )?;
        let id = conn.last_insert_rowid();
        get_profile_by_id(conn, id).map(|opt| {
            opt.unwrap_or(UploadProfile {
                id: Some(id),
                ..normalized_profile
            })
        })
    }
}

pub fn delete_upload_profile(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM upload_profiles WHERE id = ?", params![id])?;
    Ok(())
}

pub fn restore_default_profiles(conn: &Connection) -> Result<Vec<UploadProfile>> {
    conn.execute("DELETE FROM upload_profiles", [])?;
    let mut inserted = Vec::new();
    for mut profile in services::system_profiles().default_system_profiles() {
        profile.id = None;
        let saved = save_upload_profile(conn, &profile)?;
        inserted.push(saved);
    }
    Ok(inserted)
}

pub fn get_profile_by_id(conn: &Connection, id: i64) -> Result<Option<UploadProfile>> {
    conn.query_row(
        "SELECT id, name, plan_json
         FROM upload_profiles WHERE id = ?",
        params![id],
        map_profile,
    )
    .optional()
}

