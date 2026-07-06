use serde_json::{json, Value};

use crate::{
    app_runtime::AppState,
    core::error::{report, AppError},
    features::upload::{UploadContext, UploadError},
};

pub(super) fn upload_context(action: &str, extra: Value) -> Value {
    let mut context = serde_json::Map::from_iter([
        ("feature".to_string(), json!("upload")),
        ("action".to_string(), json!(action)),
    ]);

    if let Value::Object(extra) = extra {
        context.extend(extra);
    }

    Value::Object(context)
}

pub(super) fn map_upload_error(action: &str, extra: Value, err: UploadError) -> AppError {
    report(
        "upload",
        AppError::from(err).with_context(upload_context(action, extra)),
    )
}

pub(super) fn ctx(st: &tauri::State<'_, AppState>) -> UploadContext {
    st.inner().upload_context()
}
