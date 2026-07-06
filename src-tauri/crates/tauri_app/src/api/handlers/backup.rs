use serde_json::{json, Value};

use crate::app_wiring::app_runtime::AppState;
use omega_drive_db::db_executor::DbExecutor;
use omega_drive_gateway::core::error::{AppError, AppResult};

fn read_discord_token() -> String {
    std::env::var("DISCORD_TOKEN").unwrap_or_default().trim().to_string()
}

#[tauri::command]
pub async fn trigger_backup_snapshot(
    st: tauri::State<'_, AppState>,
) -> AppResult<Value> {
    // Precondition: Discord token must be set
    if read_discord_token().is_empty() {
        return Err(AppError::from("Discord token not configured"));
    }

    // Precondition: Discord must be connected
    let provider_runtime = st.provider_runtime();
    let discord_ok = match provider_runtime.provider_admin_registry.get("discord") {
        Some(gateway) => gateway.connection_status().await.ok()
            .map(|s| s.connected).unwrap_or(false),
        None => false,
    };
    if !discord_ok {
        return Err(AppError::from("Discord is not connected"));
    }

    // Precondition: must have an active tenant (DB)
    let _tenant = st
        .active_tenant
        .lock()
        .map_err(|e| AppError::from(format!("Failed to read active tenant: {e}")))?
        .clone();

    let backup_service = st
        .backup_service
        .as_ref()
        .ok_or_else(|| AppError::from("Backup service not initialized"))?
        .clone();

    let dc = crate::providers::discord_provider::discord_backup_gateway()
        .ok_or_else(|| AppError::from("Discord not initialized"))?;
    let base_dir = st.base_dir.clone();
    let chunks = {
        let mut result = None;
        st.db_write.read(&mut |c| {
            result = Some(omega_drive_db::backup::create_snapshot(c, &base_dir));
        });
        result.ok_or_else(|| AppError::from("Failed to create snapshot"))?
    }.map_err(AppError::from)?;
    let thread_id = crate::features::backup::run_snapshot_and_upload(
        &dc, chunks, &backup_service,
    )
    .await
    .map_err(AppError::from)?;

    Ok(json!({ "status": "ok", "thread_id": thread_id }))
}

#[tauri::command]
pub async fn list_backup_snapshots(
    st: tauri::State<'_, AppState>,
) -> AppResult<Value> {
    let thread_id = backup_thread_id(&st)?;
    let dc = crate::providers::discord_provider::discord_backup_gateway()
        .ok_or_else(|| format!("Discord not initialized"))?;
    let snapshots = crate::features::backup::list_snapshots(&dc, thread_id)
        .await
        .map_err(|e| format!("Failed to list snapshots: {e}"))?;
    Ok(json!({ "snapshots": snapshots }))
}

#[tauri::command]
pub async fn restore_backup_snapshot(
    st: tauri::State<'_, AppState>,
    snapshot_timestamp: String,
) -> AppResult<Value> {
    let thread_id = backup_thread_id(&st)?;
    let dc = crate::providers::discord_provider::discord_backup_gateway()
        .ok_or_else(|| AppError::from("Discord not initialized"))?;

    let snapshots = crate::features::backup::list_snapshots(&dc, thread_id)
        .await
        .map_err(|e| format!("Failed to list snapshots: {e}"))?;
    let snapshot = snapshots
        .into_iter()
        .find(|s| s.timestamp == snapshot_timestamp)
        .ok_or_else(|| format!("Snapshot '{snapshot_timestamp}' not found"))?;

    let (db_bytes, db_filename) =
        crate::features::backup::download_snapshot(&dc, &snapshot)
            .await
            .map_err(|e| format!("Failed to download snapshot: {e}"))?;

    let base_dir = st.base_dir.clone();
    let tenant = st
        .active_tenant
        .lock()
        .map(|t| t.clone())
        .map_err(|e| format!("Failed to read active tenant: {e}"))?;
    let db_path = omega_drive_core::tenant_registry::tenant_db_path(&base_dir, &tenant);

    let backup_path = base_dir.join(format!(
        "omega_drive_before_restore_{}.db",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    ));
    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| format!("Failed to backup current DB: {e}"))?;
    std::fs::write(&db_path, &db_bytes)
        .map_err(|e| format!("Failed to write restored DB: {e}"))?;

    st.db_write
        .reopen(&db_path)
        .await
        .map_err(|e| format!("Failed to reopen write DB: {e}"))?;
    st.drive_db_read
        .reopen(&db_path)
        .await
        .map_err(|e| format!("Failed to reopen read pool: {e}"))?;

    let replayed = crate::features::backup::replay_ops(
        &dc,
        |op: &omega_drive_gateway::core::backup::Op| -> Result<(), String> {
            let mut err: Option<String> = None;
            st.db_write.write(&mut |conn| {
                if let Err(e) = omega_drive_db::backup::apply_op(conn, op) {
                    err = Some(e);
                }
            });
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        },
        thread_id, 0,
    )
    .await
    .map_err(|e| format!("Failed to replay ops: {e}"))?;

    Ok(json!({
        "restored": db_filename,
        "backup_path": backup_path.to_str(),
        "ops_replayed": replayed,
    }))
}

fn backup_thread_id(st: &AppState) -> AppResult<u64> {
    let backup = st
        .backup_service
        .as_ref()
        .ok_or_else(|| AppError::from("Backup service not initialized"))?;
    backup
        .latest_backup_thread_id()
        .ok_or_else(|| AppError::from("No active backup thread"))
}




