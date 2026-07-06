use serde_json::{json, Value};

use crate::{
    app_runtime::AppState,
    core::{error::AppResult, upload_plan::UploadPlan},
    core::scope::parse_scope,
    features::upload::transfer,
};

use super::upload_common::{ctx, map_upload_error};

#[tauri::command]
pub async fn upload_file_from_path(
    st: tauri::State<'_, AppState>,
    file_path: String,
    folder_id: Option<i64>,
    drive_scope: Option<String>,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> AppResult<Value> {
    let upload = ctx(&st);
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| {
            map_upload_error(
                "upload_file_from_path",
                json!({ "drive_scope": drive_scope }),
                crate::features::upload::UploadError::create_validation_error(message),
            )
        })?
        .unwrap_or_default();
    transfer::upload_file_from_path(
        &upload,
        file_path,
        folder_id,
        scope,
        session_id,
        profile_id,
        upload_plan,
    )
    .await
    .map_err(|e| map_upload_error("upload_file_from_path", json!({}), e))
}

#[tauri::command]
pub async fn upload_files_from_paths(
    st: tauri::State<'_, AppState>,
    file_paths: Vec<String>,
    folder_id: Option<i64>,
    drive_scope: Option<String>,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> AppResult<Value> {
    let upload = ctx(&st);
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| {
            map_upload_error(
                "upload_files_from_paths",
                json!({ "drive_scope": drive_scope }),
                crate::features::upload::UploadError::create_validation_error(message),
            )
        })?
        .unwrap_or_default();
    transfer::upload_files_from_paths(
        &upload,
        file_paths,
        folder_id,
        scope,
        session_id,
        profile_id,
        upload_plan,
    )
    .await
    .map_err(|e| map_upload_error("upload_files_from_paths", json!({}), e))
}

#[tauri::command]
pub async fn cancel_upload(st: tauri::State<'_, AppState>, session_id: String) -> AppResult<Value> {
    cancel_transfer(st, session_id).await
}

#[tauri::command]
pub async fn pause_upload(st: tauri::State<'_, AppState>, session_id: String) -> AppResult<Value> {
    let upload = ctx(&st);
    transfer::pause_upload(&upload, session_id.clone())
        .await
        .map_err(|e| map_upload_error("pause_upload", json!({ "session_id": session_id }), e))
}

#[tauri::command]
pub async fn resume_upload(
    st: tauri::State<'_, AppState>,
    session_id: String,
    file_id: i64,
    file_path: String,
    folder_id: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let upload = ctx(&st);
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| {
            map_upload_error(
                "resume_upload",
                json!({ "drive_scope": drive_scope }),
                crate::features::upload::UploadError::create_validation_error(message),
            )
        })?
        .unwrap_or_default();
    transfer::resume_upload(
        &upload,
        session_id.clone(),
        file_id,
        file_path,
        folder_id,
        scope,
    )
    .await
    .map_err(|e| {
        map_upload_error(
            "resume_upload",
            json!({ "file_id": file_id, "session_id": session_id }),
            e,
        )
    })
}

#[tauri::command]
#[tracing::instrument(
    skip(st, upload_plan),
    fields(
        file_path = %file_path,
        folder_id = ?folder_id,
        session_id = %session_id,
        profile_id = ?profile_id
    )
)]
pub async fn upload_file_native(
    st: tauri::State<'_, AppState>,
    file_path: String,
    folder_id: Option<i64>,
    drive_scope: Option<String>,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> AppResult<Value> {
    let upload = ctx(&st);
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| {
            map_upload_error(
                "upload_file_native",
                json!({ "drive_scope": drive_scope }),
                crate::features::upload::UploadError::create_validation_error(message),
            )
        })?
        .unwrap_or_default();
    transfer::upload_file_native(
        &upload,
        file_path,
        folder_id,
        scope,
        session_id,
        profile_id,
        upload_plan,
    )
    .await
    .map_err(|e| map_upload_error("upload_file_native", json!({}), e))
}

#[tauri::command]
pub async fn cancel_transfer(
    st: tauri::State<'_, AppState>,
    session_id: String,
) -> AppResult<Value> {
    let upload = ctx(&st);
    transfer::cancel_transfer(&upload, session_id.clone())
        .await
        .map_err(|e| map_upload_error("cancel_transfer", json!({ "session_id": session_id }), e))
}
