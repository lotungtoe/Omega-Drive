use serde_json::{json, Value};
use std::path::PathBuf;

use omega_drive_gateway::core::data::DownloadJob;

use crate::{
    app_runtime::AppState,
    core::{
        error::{wrap_error, AppResult},
        error_codes as codes,
    },
};

#[tauri::command]
#[tracing::instrument(skip(st), fields(file_id = file_id, session_id = %session_id, save_path = %save_path))]
pub async fn download_file_to_disk(
    st: tauri::State<'_, AppState>,
    file_id: i64,
    save_path: String,
    session_id: String,
) -> AppResult<Value> {
    let job = st
        .download_manager
        .queue_download(st.inner().download_context(), file_id, save_path)
        .await?;
    Ok(json!({ "status": "queued", "jobId": job.id, "sessionId": session_id }))
}

#[tauri::command]
pub async fn queue_download(
    st: tauri::State<'_, AppState>,
    file_id: i64,
    target_path: String,
) -> AppResult<DownloadJob> {
    st.download_manager
        .queue_download(st.inner().download_context(), file_id, target_path)
        .await
}

#[tauri::command]
pub async fn pause_download(st: tauri::State<'_, AppState>, job_id: i64) -> AppResult<Value> {
    st.download_manager
        .pause_download(st.inner().download_context(), job_id)
        .await?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn resume_download(st: tauri::State<'_, AppState>, job_id: i64) -> AppResult<Value> {
    st.download_manager
        .resume_download(st.inner().download_context(), job_id)
        .await?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn cancel_download(st: tauri::State<'_, AppState>, job_id: i64) -> AppResult<Value> {
    st.download_manager
        .cancel_download(st.inner().download_context(), job_id)
        .await?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn retry_download(st: tauri::State<'_, AppState>, job_id: i64) -> AppResult<Value> {
    st.download_manager
        .retry_download(st.inner().download_context(), job_id)
        .await?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn list_download_jobs(st: tauri::State<'_, AppState>) -> AppResult<Vec<DownloadJob>> {
    st.download_manager
        .list_download_jobs(st.inner().download_context())
        .await
}

#[tauri::command]
pub async fn open_download_file(path: String) -> AppResult<Value> {
    let ctx = json!({
        "feature": "download",
        "action": "open_download_file",
    });
    open::that(&path).map_err(|e| {
        wrap_error(
            "download",
            codes::E_IO,
            "Cannot open download file.",
            ctx,
            e,
        )
    })?;
    Ok(json!({ "success": true }))
}

#[tauri::command]
pub async fn open_download_folder(path: String) -> AppResult<Value> {
    let ctx = json!({
        "feature": "download",
        "action": "open_download_folder",
    });
    let path = PathBuf::from(path);
    let target = path.parent().unwrap_or(&path);
    open::that(target).map_err(|e| {
        wrap_error(
            "download",
            codes::E_IO,
            "Cannot open download folder.",
            ctx,
            e,
        )
    })?;
    Ok(json!({ "success": true }))
}
