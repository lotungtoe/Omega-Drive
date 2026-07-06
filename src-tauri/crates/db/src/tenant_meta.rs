use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Result};

use omega_drive_gateway::core::tenant::TenantDescriptor;

pub fn upsert_tenant_meta(conn: &Connection, tenant: &TenantDescriptor) -> Result<()> {
    conn.execute(
        "INSERT INTO tenant_meta (singleton, scope, discord_guild_id, telegram_group_id)
         VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(singleton) DO UPDATE SET
            scope = excluded.scope,
            discord_guild_id = excluded.discord_guild_id,
            telegram_group_id = excluded.telegram_group_id",
        params![
            tenant.scope,
            tenant.discord_guild_id,
            tenant.telegram_group_id
        ],
    )?;
    Ok(())
}

pub fn get_tenant_display_name(conn: &Connection) -> Result<Option<String>> {
    ensure_display_name_column(conn)?;
    conn.query_row(
        "SELECT display_name
         FROM tenant_meta
         WHERE singleton = 1",
        [],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
    .map(|value| value.flatten())
}

pub fn set_tenant_display_name(conn: &Connection, display_name: Option<&str>) -> Result<()> {
    ensure_display_name_column(conn)?;
    conn.execute(
        "UPDATE tenant_meta
         SET display_name = ?1
         WHERE singleton = 1",
        params![normalize_display_name(display_name)],
    )?;
    Ok(())
}

pub fn get_tenant_meta(conn: &Connection) -> Result<TenantDescriptor> {
    conn.query_row(
        "SELECT scope, discord_guild_id, telegram_group_id
         FROM tenant_meta
         WHERE singleton = 1",
        [],
        |row| {
            Ok(TenantDescriptor::new(
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get(2)?,
            ))
        },
    )
    .optional()
    .map(|tenant| tenant.unwrap_or_default())
}

pub fn read_tenant_display_name_from_db(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }

    let conn = Connection::open(path).ok()?;
    get_tenant_display_name(&conn).ok().flatten()
}

fn normalize_display_name(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ensure_display_name_column(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(tenant_meta)")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        if row? == "display_name" {
            return Ok(());
        }
    }
    conn.execute("ALTER TABLE tenant_meta ADD COLUMN display_name TEXT", [])?;
    Ok(())
}

