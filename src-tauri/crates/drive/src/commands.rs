use omega_drive_gateway::core::events::OmegaEvent;
use omega_drive_gateway::core::scope::DriveScope;
use serde_json::{json, Value};

use crate::context::DriveCommandContext;
use crate::error::DriveError;

fn map_move_folder_error(err: String) -> DriveError {
    match err.as_str() {
        "cannot move folder into itself" => DriveError::validation("Cannot move folder into itself."),
        "cannot move folder into descendant" => DriveError::validation("Cannot move folder into its descendant."),
        "folder not found" => DriveError::not_found("Folder not found."),
        _ => {
            if let Some(source) = err.strip_prefix("db: ") {
                DriveError::db("DB error when updating folder.", source)
            } else {
                DriveError::internal("Cannot move folder.", err)
            }
        }
    }
}

fn map_delete_folder_error(err: String) -> DriveError {
    if let Some(source) = err.strip_prefix("db: ") {
        DriveError::db("DB error when deleting folder.", source)
    } else {
        DriveError::provider("Cannot delete folder.", err)
    }
}

pub async fn create_folder(
    ctx: &DriveCommandContext,
    name: String,
    parent_id: Option<i64>,
    drive_scope: DriveScope,
) -> Result<Value, DriveError> {
    let folder_id = ctx
        .service
        .create_folder(&name, parent_id, drive_scope)
        .await
        .map_err(|e| DriveError::provider("Cannot create folder.", e))?;

    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "id": folder_id, "name": name }))
}

pub async fn rename_folder(
    ctx: &DriveCommandContext,
    folder_id: i64,
    new_name: String,
) -> Result<Value, DriveError> {
    ctx.service
        .rename_folder(folder_id, &new_name)
        .await
        .map_err(|e| DriveError::provider("Cannot rename folder.", e))?;

    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn delete_folder(ctx: &DriveCommandContext, folder_id: i64) -> Result<Value, DriveError> {
    ctx.service
        .delete_folder(folder_id)
        .await
        .map_err(map_delete_folder_error)?;
    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn move_folder(
    ctx: &DriveCommandContext,
    folder_id: i64,
    parent_id: Option<i64>,
) -> Result<Value, DriveError> {
    ctx.service
        .move_folder(folder_id, parent_id)
        .await
        .map_err(map_move_folder_error)?;
    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn delete_file(ctx: &DriveCommandContext, file_id: i64) -> Result<Value, DriveError> {
    ctx.file_repo
        .move_to_trash(file_id).await
        .map_err(|e| DriveError::db("DB error when trashing file.", e))?;
    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn rename_file(
    ctx: &DriveCommandContext,
    file_id: i64,
    new_name: String,
) -> Result<Value, DriveError> {
    let new_name = new_name.trim().to_string();
    if new_name.is_empty() {
        return Err(DriveError::validation("Filename cannot be empty."));
    }
    ctx.file_repo
        .update_file_name(file_id, &new_name).await
        .map_err(|e| DriveError::db("DB error when renaming file.", e))?;
    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn restore_file(ctx: &DriveCommandContext, file_id: i64) -> Result<Value, DriveError> {
    let restored = ctx.file_repo
        .restore_trash(file_id).await
        .map_err(|e| DriveError::db("DB error when restoring file.", e))?;
    if !restored {
        return Err(DriveError::not_found("File no longer in trash or being cleaned up."));
    }
    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn move_file(
    ctx: &DriveCommandContext,
    file_id: i64,
    folder_id: Option<i64>,
) -> Result<Value, DriveError> {
    let file = ctx.file_repo
        .get_file_by_id(file_id).await
        .map_err(|e| DriveError::db("DB error when loading file.", e))?
        .ok_or_else(|| DriveError::not_found("File not found."))?;

    if let Some(target_folder_id) = folder_id {
        let target_folder = ctx.folder_repo
            .get_folder_by_id(target_folder_id).await
            .map_err(|e| DriveError::db("DB error when loading destination folder.", e))?
            .ok_or_else(|| DriveError::not_found("Destination folder not found."))?;

        if target_folder.drive_scope != file.drive_scope {
            return Err(DriveError::validation("Cannot move file to a different drive scope."));
        }
    }

    ctx.file_repo
        .update_file_folder(file_id, folder_id).await
        .map_err(|e| DriveError::db("DB error when moving file.", e))?;

    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn purge_file(ctx: &DriveCommandContext, file_id: i64) -> Result<Value, DriveError> {
    ctx.service
        .purge_file(file_id).await
        .map_err(|e| DriveError::provider("Cannot permanently delete file.", e))?;

    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn toggle_star(
    ctx: &DriveCommandContext,
    id: i64,
    is_folder: bool,
    starred: bool,
) -> Result<Value, DriveError> {
    if is_folder {
        ctx.folder_repo
            .toggle_folder_star(id, starred).await
            .map_err(|e| DriveError::db("DB error when updating folder star.", e))?;
    } else {
        ctx.file_repo
            .toggle_star(id, starred).await
            .map_err(|e| DriveError::db("DB error when updating file star.", e))?;
    }
    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn empty_trash(ctx: &DriveCommandContext) -> Result<Value, DriveError> {
    let all = ctx.file_repo
        .get_all_files().await
        .map_err(|e| DriveError::db("DB error when loading trash.", e))?;
    let ids: Vec<_> = all.into_iter()
        .filter(|f| f.status == "trashed")
        .map(|f| f.id)
        .collect();

    let mut failures = Vec::new();
    for id in ids {
        if let Err(e) = ctx.service.purge_file(id).await {
            failures.push(format!("#{id}: {e}"));
        }
    }

    if !failures.is_empty() {
        return Err(DriveError::provider(
            format!("Cannot empty all trash: {}", failures.join("; ")),
            "bulk purge failed",
        ));
    }

    ctx.events.emit(OmegaEvent::FilesTableChanged);
    Ok(json!({ "success": true }))
}

pub async fn retrieve_full_file(
    ctx: &DriveCommandContext,
    file_id: i64,
) -> Result<Vec<u8>, DriveError> {
    ctx.service
        .retrieve_full_file(file_id).await
        .map_err(|e| DriveError::provider("Cannot load file into RAM", e))
}
