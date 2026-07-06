use omega_drive_extractor as kreuzberg_extractor;
use omega_drive_extractor::KreuzbergResult;

use crate::core::error::wrap_error;
use rxing::multi::MultipleBarcodeReader;
use rxing::{
    common::HybridBinarizer, BinaryBitmap, BufferedImageLuminanceSource, DecodeHintType,
    DecodeHintValue, DecodingHintDictionary, MultiFormatReader, RXingResult,
};
use serde_json::{json, Value};
use tokio::task;
use tracing::info;

use crate::{
    app_runtime::AppState,
    core::{
        error::{report, AppError, AppResult},
        error_codes as codes,
    },
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

pub(crate) fn map_drive_error(action: &str, extra: Value, err: DriveError) -> AppError {
    report(
        "drive",
        AppError::from(err).with_context(drive_context(action, extra)),
    )
}

#[tauri::command]
pub async fn get_files(
    st: tauri::State<'_, AppState>,
    folder_id: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref()).map_err(|message| {
        map_drive_error(
            "get_files",
            json!({ "folder_id": folder_id, "drive_scope": drive_scope }),
            DriveError::validation(message),
        )
    })?;
    queries::get_files(&ctx, folder_id, scope)
        .await
        .map_err(|e| {
            map_drive_error(
                "get_files",
                json!({ "folder_id": folder_id, "drive_scope": drive_scope }),
                e,
            )
        })
}

#[tauri::command]
pub async fn get_files_paginated(
    st: tauri::State<'_, AppState>,
    folder_id: Option<i64>,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| map_drive_error("get_files_paginated", json!({ "folder_id": folder_id, "cursor": cursor, "limit": limit.unwrap_or(50).min(200), "drive_scope": drive_scope }), DriveError::validation(message)))?;
    queries::get_files_paginated(&ctx, folder_id, cursor, limit, scope)
        .await
        .map_err(|e| {
            map_drive_error(
                "get_files_paginated",
                json!({
                    "folder_id": folder_id,
                    "cursor": cursor,
                    "limit": limit.unwrap_or(50).min(200),
                    "drive_scope": drive_scope,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn get_all_files(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    info!("[IPC Audit] Calling handler: get_all_files");
    let ctx = st.inner().drive_query_context();
    queries::get_all_files(&ctx)
        .await
        .map_err(|e| map_drive_error("get_all_files", json!({}), e))
}

#[tauri::command]
pub async fn get_all_files_paginated(
    st: tauri::State<'_, AppState>,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| map_drive_error("get_all_files_paginated", json!({ "cursor": cursor, "limit": limit.unwrap_or(50).min(200), "drive_scope": drive_scope }), DriveError::validation(message)))?;
    queries::get_all_files_paginated(&ctx, cursor, limit, scope)
        .await
        .map_err(|e| {
            map_drive_error(
                "get_all_files_paginated",
                json!({
                    "cursor": cursor,
                    "limit": limit.unwrap_or(50).min(200),
                    "drive_scope": drive_scope,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn get_recent_files_paginated(
    st: tauri::State<'_, AppState>,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| map_drive_error("get_recent_files_paginated", json!({ "cursor": cursor, "limit": limit.unwrap_or(50).min(200), "drive_scope": drive_scope }), DriveError::validation(message)))?;
    queries::get_recent_files_paginated(&ctx, cursor, limit, scope)
        .await
        .map_err(|e| {
            map_drive_error(
                "get_recent_files_paginated",
                json!({
                    "cursor": cursor,
                    "limit": limit.unwrap_or(50).min(200),
                    "drive_scope": drive_scope,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn get_trash_paginated(
    st: tauri::State<'_, AppState>,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref())
        .map_err(|message| map_drive_error("get_trash_paginated", json!({ "cursor": cursor, "limit": limit.unwrap_or(50).min(200), "drive_scope": drive_scope }), DriveError::validation(message)))?;
    queries::get_trash_paginated(&ctx, cursor, limit, scope)
        .await
        .map_err(|e| {
            map_drive_error(
                "get_trash_paginated",
                json!({
                    "cursor": cursor,
                    "limit": limit.unwrap_or(50).min(200),
                    "drive_scope": drive_scope,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn get_transfers_paginated(
    st: tauri::State<'_, AppState>,
    cursor: Option<i64>,
    limit: Option<i64>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    queries::get_transfers_paginated(&ctx, cursor, limit)
        .await
        .map_err(|e| {
            map_drive_error(
                "get_transfers_paginated",
                json!({
                    "cursor": cursor,
                    "limit": limit.unwrap_or(50).min(200),
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn delete_file(st: tauri::State<'_, AppState>, file_id: i64) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::delete_file(&ctx, file_id)
        .await
        .map_err(|e| map_drive_error("delete_file", json!({ "file_id": file_id }), e))
}

#[tauri::command]
pub async fn rename_file(
    st: tauri::State<'_, AppState>,
    file_id: i64,
    new_name: String,
) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::rename_file(&ctx, file_id, new_name.clone())
        .await
        .map_err(|e| {
            map_drive_error(
                "rename_file",
                json!({
                    "file_id": file_id,
                    "new_name": new_name,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn restore_file(st: tauri::State<'_, AppState>, file_id: i64) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::restore_file(&ctx, file_id)
        .await
        .map_err(|e| map_drive_error("restore_file", json!({ "file_id": file_id }), e))
}

#[tauri::command]
pub async fn move_file(
    st: tauri::State<'_, AppState>,
    file_id: i64,
    folder_id: Option<i64>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::move_file(&ctx, file_id, folder_id)
        .await
        .map_err(|e| {
            map_drive_error(
                "move_file",
                json!({
                    "file_id": file_id,
                    "folder_id": folder_id,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn purge_file(st: tauri::State<'_, AppState>, file_id: i64) -> AppResult<Value> {
    let u_ctx = st.inner().upload_context();
    let _ = crate::features::upload::transfer::cancel_transfer_by_file_id(&u_ctx, file_id).await;

    let ctx = st.inner().drive_command_context();
    commands::purge_file(&ctx, file_id)
        .await
        .map_err(|e| map_drive_error("purge_file", json!({ "file_id": file_id }), e))
}

#[tauri::command]
pub async fn toggle_star(
    st: tauri::State<'_, AppState>,
    id: i64,
    is_folder: bool,
    starred: bool,
) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::toggle_star(&ctx, id, is_folder, starred)
        .await
        .map_err(|e| {
            map_drive_error(
                "toggle_star",
                json!({
                    "id": id,
                    "is_folder": is_folder,
                    "starred": starred,
                }),
                e,
            )
        })
}

#[tauri::command]
pub async fn search_files(st: tauri::State<'_, AppState>, q: String) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    queries::search_files(&ctx, q.clone())
        .await
        .map_err(|e| map_drive_error("search_files", json!({ "query": q }), e))
}

#[tauri::command]
pub async fn get_stats(
    st: tauri::State<'_, AppState>,
    drive_scope: Option<String>,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    let scope = parse_scope(drive_scope.as_deref()).map_err(|message| {
        map_drive_error(
            "get_stats",
            json!({ "drive_scope": drive_scope }),
            DriveError::validation(message),
        )
    })?;
    queries::get_stats(&ctx, scope)
        .await
        .map_err(|e| map_drive_error("get_stats", json!({ "drive_scope": drive_scope }), e))
}

#[tauri::command]
pub async fn get_trash(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    queries::get_trash(&ctx)
        .await
        .map_err(|e| map_drive_error("get_trash", json!({}), e))
}

#[tauri::command]
pub async fn empty_trash(st: tauri::State<'_, AppState>) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    commands::empty_trash(&ctx)
        .await
        .map_err(|e| map_drive_error("empty_trash", json!({}), e))
}

#[tauri::command]
pub async fn retrieve_thumbnail(st: tauri::State<'_, AppState>, file_id: i64) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    queries::retrieve_thumbnail(&ctx, file_id)
        .await
        .map_err(|e| map_drive_error("retrieve_thumbnail", json!({ "file_id": file_id }), e))
}

#[tauri::command]
pub async fn get_video_full_metadata(
    st: tauri::State<'_, AppState>,
    file_id: i64,
) -> AppResult<Value> {
    let ctx = st.inner().drive_query_context();
    queries::get_video_full_metadata(&ctx, file_id)
        .await
        .map_err(|e| map_drive_error("get_video_full_metadata", json!({ "file_id": file_id }), e))
}

#[tauri::command]
pub async fn retrieve_full_file(
    st: tauri::State<'_, AppState>,
    file_id: i64,
) -> AppResult<Vec<u8>> {
    let ctx = st.inner().drive_command_context();
    commands::retrieve_full_file(&ctx, file_id)
        .await
        .map_err(|e| map_drive_error("retrieve_full_file", json!({ "file_id": file_id }), e))
}

async fn scan_qr_internal(bytes: Vec<u8>) -> AppResult<Value> {
    // Use spawn_blocking to avoid blocking async runtime since image decoding and QR scanning are CPU-intensive
    let results: Result<Vec<Value>, String> = task::spawn_blocking(move || {
        let img =
            image::load_from_memory(&bytes).map_err(|e| format!("Error loading image from bytes: {e}"))?;

        // Convert to DynamicImage (rxing BufferedImageLuminanceSource needs DynamicImage or Luma Image)
        let luminance_source = BufferedImageLuminanceSource::new(img);
        let binarizer = HybridBinarizer::new(luminance_source);
        let mut bitmap = BinaryBitmap::new(binarizer);

        let reader = MultiFormatReader::default();
        // Wrap reader to scan multiple barcodes at once
        let mut multi_reader = rxing::multi::GenericMultipleBarcodeReader::new(reader);

        // Configure hint for more thorough scanning (try_harder)
        let mut hints = DecodingHintDictionary::new();
        hints.insert(DecodeHintType::TRY_HARDER, DecodeHintValue::TryHarder(true));

        // Scan all detectable barcodes
        match multi_reader.decode_multiple_with_hints(&mut bitmap, &hints) {
            Ok(scan_results) => {
                let mapped: Vec<Value> = scan_results
                    .into_iter()
                    .map(|r: RXingResult| {
                        json!({
                            "text": r.getText(),
                            "format": format!("{:?}", r.getBarcodeFormat()),
                        })
                    })
                    .collect();
                Ok::<Vec<Value>, String>(mapped)
            }
            Err(_) => {
                // If no barcodes found, return empty array
                Ok::<Vec<Value>, String>(vec![])
            }
        }
    })
    .await
    .map_err(|e| AppError::new(codes::E_UNKNOWN, format!("System error scanning QR: {}", e)))?;

    results
        .map(|v| json!(v))
        .map_err(|e| AppError::new(codes::E_UNKNOWN, e))
}

#[tauri::command]
pub async fn scan_qr_from_bytes(bytes: Vec<u8>) -> AppResult<Value> {
    scan_qr_internal(bytes).await
}

#[tauri::command]
pub async fn scan_qr_by_file_id(st: tauri::State<'_, AppState>, file_id: i64) -> AppResult<Value> {
    let ctx = st.inner().drive_command_context();
    let bytes = ctx
        .service
        .retrieve_full_file(file_id)
        .await
        .map_err(|e| AppError::new(codes::E_UNKNOWN, e))?;

    scan_qr_internal(bytes).await
}

#[tauri::command]
pub async fn extract_file_text(
    st: tauri::State<'_, AppState>,
    file_id: i64,
    filename: String,
) -> AppResult<KreuzbergResult> {
    let ctx = st.inner().drive_command_context();
    let bytes = ctx
        .service
        .retrieve_full_file(file_id)
        .await
        .map_err(|e| AppError::new(codes::E_UNKNOWN, e))?;

    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let result = kreuzberg_extractor::extract_text(&bytes, &ext)
        .await
        .map_err(|e| {
            AppError::new(
                codes::E_UNKNOWN,
                format!("Text extraction failed: {}", e),
            )
        })?;

    Ok(result)
}

#[tauri::command]
pub async fn open_external_url(url: String) -> AppResult<()> {
    // Only allow http and https
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AppError::new(
            codes::E_INVALID_INPUT,
            "Only supports opening http or https links",
        ));
    }

    let ctx = json!({
        "feature": "drive",
        "action": "open_external_url",
        "url": url
    });

    open::that(&url).map_err(|e| {
        wrap_error(
            "drive",
            crate::core::error_codes::E_IO,
            "Cannot open external link.",
            ctx,
            e,
        )
    })
}
