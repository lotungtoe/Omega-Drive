use serde_json::{json, Value};

use crate::{
    app_runtime::AppState,
    core::error::{report, AppError, AppResult},
    core::scope::parse_scope,
    features::drive::{commands, error::DriveError, queries},
};

fn drive_context(action: &str, extra: Value) -> Value {
    let mut context = serde_json::Map::from_iter([
        ("feature".to_string(), json!("drive")),
        ("action".to_string(), json!(action)),
    ]);

    if let Value::Object(extra) = extra {
        context.extend(extra);
    }

    Value::Object(context)
}

fn map_drive_error(action: &str, extra: Value, err: DriveError) -> AppError {
    report(
        "drive",
        AppError::from(err).with_context(drive_context(action, extra)),
    )
}

#[tauri::command]
pub async fn get_folders(
    st: tauri::State<'_, AppState>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref()).map_err(|message| {
        map_drive_error(
            "get_folders",
            json!({ "drive_scope": drive_scope }),
            DriveError::validation(message),
        )
    })?;
    queries::get_folders(&ctx, scope)
        .await
        .map_err(|e| map_drive_error("get_folders", json!({ "drive_scope": drive_scope }), e))
}

#[tauri::command]
pub async fn create_folder(
    st: tauri::State<'_, AppState>,
    name: String,
    parent_id: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| {
            map_drive_error(
                "create_folder",
                json!({ "parent_id": parent_id, "name": name, "drive_scope": drive_scope }),
                DriveError::validation(message),
            )
        })?
        .unwrap_or_default();
    commands::create_folder(&ctx, name.clone(), parent_id, scope)
        .await
        .map_err(|e| {
            map_drive_error(
                "create_folder",
                json!({
                    "parent_id": parent_id,
                    "name": name,
                    "drive_scope": drive_scope,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn rename_folder(
    st: tauri::State<'_, AppState>,
    folder_id: i64,
    new_name: String,
) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::rename_folder(&ctx, folder_id, new_name.clone())
        .await
        .map_err(|e| {
            map_drive_error(
                "rename_folder",
                json!({
                    "folder_id": folder_id,
                    "new_name": new_name,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn delete_folder(st: tauri::State<'_, AppState>, folder_id: i64) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::delete_folder(&ctx, folder_id)
        .await
        .map_err(|e| map_drive_error("delete_folder", json!({ "folder_id": folder_id }), e))
}

#[tauri::command]
pub async fn move_folder(
    st: tauri::State<'_, AppState>,
    folder_id: i64,
    parent_id: Option<i64>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::move_folder(&ctx, folder_id, parent_id)
        .await
        .map_err(|e| {
            map_drive_error(
                "move_folder",
                json!({
                    "folder_id": folder_id,
                    "parent_id": parent_id,
                }),
                e,
            )
        })
}
