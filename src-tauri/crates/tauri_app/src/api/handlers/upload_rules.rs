use serde_json::Value;

use crate::{
    app_runtime::AppState,
    core::{error::AppResult, upload_rules::UploadProfileRule},
    features::upload::{resolution, rules},
};

use super::upload_common::{ctx, map_upload_error};

#[tauri::command]
pub async fn get_upload_profile_rules(
    st: tauri::State<'_, AppState>,
    profile_id: Option<i64>,
) -> AppResult<Vec<UploadProfileRule>> {
    let upload = ctx(&st);
    rules::get_upload_profile_rules(&upload, profile_id)
        .await
        .map_err(|e| {
            map_upload_error(
                "get_upload_profile_rules",
                serde_json::json!({ "profile_id": profile_id }),
                e,
            )
        })
}

#[tauri::command]
pub async fn save_upload_profile_rule(
    st: tauri::State<'_, AppState>,
    rule: UploadProfileRule,
) -> AppResult<UploadProfileRule> {
    let upload = ctx(&st);
    rules::save_upload_profile_rule(&upload, rule.clone())
        .await
        .map_err(|e| {
            map_upload_error(
                "save_upload_profile_rule",
                serde_json::json!({ "rule_id": rule.id, "profile_id": rule.profile_id }),
                e,
            )
        })
}

#[tauri::command]
pub async fn delete_upload_profile_rule(
    st: tauri::State<'_, AppState>,
    id: i64,
) -> AppResult<Value> {
    let upload = ctx(&st);
    rules::delete_upload_profile_rule(&upload, id)
        .await
        .map_err(|e| {
            map_upload_error(
                "delete_upload_profile_rule",
                serde_json::json!({ "rule_id": id }),
                e,
            )
        })
}

#[tauri::command]
pub async fn save_upload_profile_rules_bulk(
    st: tauri::State<'_, AppState>,
    profile_id: i64,
    ordered_rule_ids: Vec<i64>,
) -> AppResult<Vec<UploadProfileRule>> {
    let upload = ctx(&st);
    rules::save_upload_profile_rules_bulk(&upload, profile_id, ordered_rule_ids)
        .await
        .map_err(|e| {
            map_upload_error(
                "save_upload_profile_rules_bulk",
                serde_json::json!({ "profile_id": profile_id }),
                e,
            )
        })
}

#[tauri::command]
pub async fn resolve_upload_profile_for_batch(
    st: tauri::State<'_, AppState>,
    items: Vec<resolution::UploadBatchRequestItem>,
) -> AppResult<Vec<resolution::UploadBatchResolvedItem>> {
    let upload = ctx(&st);
    resolution::resolve_upload_profile_for_batch(&upload, items)
        .await
        .map_err(|e| map_upload_error("resolve_upload_profile_for_batch", serde_json::json!({}), e))
}
