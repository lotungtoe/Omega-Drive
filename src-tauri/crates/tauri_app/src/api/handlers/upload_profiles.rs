use crate::{
    app_runtime::AppState,
    core::{
        error::AppResult,
        upload_plan::UploadProfile,
    },
    features::upload::profiles,
};

use super::upload_common::{ctx, map_upload_error};

#[tauri::command]
pub async fn get_upload_profiles(st: tauri::State<'_, AppState>) -> AppResult<Vec<UploadProfile>> {
    let upload = ctx(&st);
    profiles::get_upload_profiles(&upload)
        .await
        .map_err(|e| map_upload_error("get_upload_profiles", serde_json::json!({}), e))
}

#[tauri::command]
pub async fn save_upload_profile(
    st: tauri::State<'_, AppState>,
    profile: UploadProfile,
) -> AppResult<UploadProfile> {
    let upload = ctx(&st);
    profiles::save_upload_profile(&upload, profile.clone())
        .await
        .map_err(|e| {
            map_upload_error(
                "save_upload_profile",
                serde_json::json!({ "profile_id": profile.id }),
                e,
            )
        })
}

#[tauri::command]
pub async fn delete_upload_profile(
    st: tauri::State<'_, AppState>,
    id: i64,
) -> AppResult<serde_json::Value> {
    let upload = ctx(&st);
    profiles::delete_upload_profile(&upload, id)
        .await
        .map_err(|e| {
            map_upload_error(
                "delete_upload_profile",
                serde_json::json!({ "profile_id": id }),
                e,
            )
        })
}

#[tauri::command]
pub async fn restore_default_profiles(
    st: tauri::State<'_, AppState>,
) -> AppResult<Vec<UploadProfile>> {
    let upload = ctx(&st);
    profiles::restore_default_profiles(&upload)
        .await
        .map_err(|e| map_upload_error("restore_default_profiles", serde_json::json!({}), e))
}

