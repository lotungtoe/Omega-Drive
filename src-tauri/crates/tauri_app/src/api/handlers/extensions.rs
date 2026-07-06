use std::sync::Arc;

use serde_json::Value;

use crate::{
    app_runtime::AppState, core::error::AppResult, extensions::registry::ExtensionRegistry,
};

#[tauri::command]
pub async fn invoke_feature(
    st: tauri::State<'_, AppState>,
    window: tauri::Window,
    extension_id: String,
    command_id: String,
    payload: Value,
) -> AppResult<Value> {
    let registry = ExtensionRegistry::global()?;
    registry
        .dispatch(
            Arc::new(st.inner().clone()),
            &extension_id,
            &command_id,
            payload,
            Some(window.label().to_string()),
        )
        .await
}
