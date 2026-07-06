use std::time::Instant;

use crate::app_wiring::app_runtime::AppState;
use omega_drive_core::scope::parse_scope;
use omega_drive_gateway::core::error::AppResult;
use omega_drive_gateway::upload::upload_plan::UploadPlan;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use omega_drive_external_import::import_log;

#[tauri::command]
pub async fn get_available_browsers() -> AppResult<Vec<String>> {
    Ok(omega_drive_external_import::downloader::probe_installed_browsers())
}

#[tauri::command]
pub async fn get_url_metadata(url: String, cookies_browser: Option<String>) -> AppResult<Value> {
    let metadata = crate::features::external_import::service::get_metadata(url, cookies_browser)
        .await
        .map_err(|e| omega_drive_gateway::core::error::AppError::new("external_import", e))?;
    Ok(json!(metadata))
}

#[tauri::command]
pub async fn start_url_import(
    st: tauri::State<'_, AppState>,
    _app_handle: tauri::AppHandle,
    url: String,
    cookies_browser: Option<String>,
    folder_id: Option<i64>,
    drive_scope: Option<String>,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> AppResult<Value> {
    let t_start = Instant::now();
    let base_dir = st.base_dir.clone();

    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|e| omega_drive_gateway::core::error::AppError::new("external_import", e))?
        .unwrap_or_default();

    let upload_ctx = st.inner().upload_context();

    // Step 1: Download video + audio as separate files
    let import = match crate::features::external_import::streaming_importer::start_import_stream(
        &url, cookies_browser.as_deref(), &base_dir,
    ).await {
        Ok(d) => {
            import_log::log_event(&base_dir, &session_id, &url, "download", t_start.elapsed().as_millis() as u64, Some(d.total_bytes), None);
            d
        }
        Err(e) => {
            import_log::log_event(&base_dir, &session_id, &url, "download", t_start.elapsed().as_millis() as u64, None, Some(&e));
            return Err(omega_drive_gateway::core::error::AppError::new("external_import", e));
        }
    };
    let t_download = Instant::now();

    // Step 2: Resolve upload plan
    let plan = if let Some(plan) = upload_plan {
        plan
    } else if let Some(pid) = profile_id {
        let profile = upload_ctx.upload_profile_repo.get_profile_by_id(pid).await
            .map_err(|e| omega_drive_gateway::core::error::AppError::new("external_import", e.to_string()))?;
        profile.map(|p| p.plan).unwrap_or_else(omega_drive_gateway::upload::upload_plan::balanced_upload_plan)
    } else {
        let profiles = upload_ctx.upload_profile_repo.get_upload_profiles().await
            .map_err(|e| omega_drive_gateway::core::error::AppError::new("external_import", e.to_string()))?;
        profiles.into_iter().next().map(|p| p.plan)
            .unwrap_or_else(omega_drive_gateway::upload::upload_plan::balanced_upload_plan)
    };
    import_log::log_event(&base_dir, &session_id, &url, "plan", t_download.elapsed().as_millis() as u64, None, None);
    let t_plan = Instant::now();

    // Step 3: Upload video file
    let cancel_token = CancellationToken::new();
    let video_id = match omega_drive_upload::coordinator::run_upload(
        upload_ctx.clone(),
        omega_drive_upload::coordinator::UploadDataSource::File(import.video_path.clone()),
        folder_id,
        scope,
        session_id.clone(),
        plan,
        None,
        cancel_token.clone(),
        import.metadata.duration,
        import.metadata.ext.clone(),
    )
    .await
    {
        Ok(id) => {
            import_log::log_event(&base_dir, &session_id, &url, "upload", t_plan.elapsed().as_millis() as u64, Some(import.total_bytes), None);
            id
        }
        Err(e) => {
            import_log::log_event(&base_dir, &session_id, &url, "upload", t_plan.elapsed().as_millis() as u64, None, Some(&format!("Upload failed: {e}")));
            return Err(omega_drive_gateway::core::error::AppError::new("external_import", format!("Upload failed: {e}")));
        }
    };
    let _t_upload = Instant::now();

    // Step 4: Upload audio attachment if present
    if let Some(ref audio_path) = import.audio_path {
        let audio_plan = omega_drive_gateway::upload::upload_plan::balanced_upload_plan();
        match omega_drive_upload::coordinator::run_upload(
            upload_ctx,
            omega_drive_upload::coordinator::UploadDataSource::File(audio_path.clone()),
            folder_id,
            omega_drive_core::scope::DriveScope::default(),
            format!("{}-audio", session_id),
            audio_plan,
            Some((video_id, "audio".to_string())),
            cancel_token,
            None,
            None,
        )
        .await
        {
            Ok(audio_id) => {
                let _ = st.file_repo.toggle_hidden(audio_id, true).await;
                let existing_audio: Vec<i64> = st.file_repo.get_video_file(video_id).await
                    .ok().flatten()
                    .and_then(|vm| vm.audio.and_then(|json| serde_json::from_str(&json).ok()))
                    .unwrap_or_default();
                let mut all_audio = existing_audio;
                all_audio.push(audio_id);
                let _ = crate::features::upload::audio_attach::attach_audio_files(
                    &st.upload_context(),
                    video_id, all_audio, Some(audio_id),
                ).await;
            }
            Err(e) => {
                tracing::warn!("[import] Audio upload failed, video imported without audio: {}", e);
            }
        }
    }

    // Step 5: Cleanup temp files
    tokio::spawn(async move {
        let _ = tokio::fs::remove_file(&import.video_path).await;
        if let Some(a) = &import.audio_path {
            let _ = tokio::fs::remove_file(a).await;
        }
    });

    import_log::log_event(&base_dir, &session_id, &url, "total", t_start.elapsed().as_millis() as u64, Some(import.total_bytes), None);

    Ok(json!({ "id": video_id, "status": "started" }))
}
