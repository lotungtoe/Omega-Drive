use omega_drive_gateway::core::scope::DriveScope;
use serde_json::{json, Value};

use crate::context::DriveQueryContext;
use crate::error::DriveError;
use crate::presenters::{file_to_client_value, folder_to_client_value, map_files_with_progress};

pub async fn get_files(
    ctx: &DriveQueryContext,
    folder_id: Option<i64>,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let files = ctx.file_repo
        .get_files_by_parent(folder_id, drive_scope.map(DriveScope::as_str)).await
        .map_err(|e| DriveError::db("DB error when loading file list.", e))?;
    let file_ids: Vec<i64> = files.iter().map(|f| f.id).collect();
    let parts_counts = ctx.file_repo
        .get_part_counts_for_files(&file_ids).await
        .unwrap_or_default();
    Ok(json!({ "files": map_files_with_progress(files, &*ctx.file_classifier, &parts_counts) }))
}

pub async fn get_files_paginated(
    ctx: &DriveQueryContext,
    folder_id: Option<i64>,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let limit = limit.unwrap_or(50).min(200);
    let files = ctx.file_repo
        .get_files_paginated(folder_id, cursor, limit, Some("main"), drive_scope.map(DriveScope::as_str)).await
        .map_err(|e| DriveError::db("DB error when loading paginated file list.", e))?;
    let file_ids: Vec<i64> = files.iter().map(|f| f.id).collect();
    let parts_counts = ctx.file_repo
        .get_part_counts_for_files(&file_ids).await
        .unwrap_or_default();
    let has_more = files.len() as i64 == limit;
    let next_cursor = files.last().map(|f| f.id);
    Ok(json!({
        "files": map_files_with_progress(files, &*ctx.file_classifier, &parts_counts),
        "next_cursor": next_cursor,
        "has_more": has_more,
    }))
}

pub async fn get_all_files(ctx: &DriveQueryContext) -> Result<Value, DriveError> {
    let files = ctx.file_repo
        .get_all_files().await
        .map_err(|e| DriveError::db("DB error when loading file list.", e))?
        .into_iter()
        .filter(|f| f.status == "ready" || f.status == "error")
        .collect::<Vec<_>>();
    let file_ids: Vec<i64> = files.iter().map(|f| f.id).collect();
    let parts_counts = ctx.file_repo
        .get_part_counts_for_files(&file_ids).await
        .unwrap_or_default();
    Ok(json!({ "files": map_files_with_progress(files, &*ctx.file_classifier, &parts_counts) }))
}

pub async fn get_all_files_paginated(
    ctx: &DriveQueryContext,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let limit = limit.unwrap_or(50).min(200);
    let files = ctx.file_repo
        .get_all_files_paginated(cursor, limit, Some("main"), drive_scope.map(DriveScope::as_str)).await
        .map_err(|e| DriveError::db("DB error when loading paginated file list.", e))?;
    let file_ids: Vec<i64> = files.iter().map(|f| f.id).collect();
    let parts_counts = ctx.file_repo
        .get_part_counts_for_files(&file_ids).await
        .unwrap_or_default();
    let has_more = files.len() as i64 == limit;
    let next_cursor = files.last().map(|f| f.id);
    Ok(json!({
        "files": map_files_with_progress(files, &*ctx.file_classifier, &parts_counts),
        "next_cursor": next_cursor,
        "has_more": has_more,
    }))
}

pub async fn get_recent_files_paginated(
    ctx: &DriveQueryContext,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let limit = limit.unwrap_or(50).min(200);
    let files = ctx.file_repo
        .get_recent_files_paginated(cursor, limit, Some("main"), drive_scope.map(DriveScope::as_str)).await
        .map_err(|e| DriveError::db("DB error when loading recent files.", e))?;
    let file_ids: Vec<i64> = files.iter().map(|f| f.id).collect();
    let parts_counts = ctx.file_repo
        .get_part_counts_for_files(&file_ids).await
        .unwrap_or_default();
    let has_more = files.len() as i64 == limit;
    let next_cursor = files.last().map(|f| f.id);
    Ok(json!({
        "files": map_files_with_progress(files, &*ctx.file_classifier, &parts_counts),
        "next_cursor": next_cursor,
        "has_more": has_more,
    }))
}

pub async fn get_trash_paginated(
    ctx: &DriveQueryContext,
    cursor: Option<i64>,
    limit: Option<i64>,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let limit = limit.unwrap_or(50).min(200);
    let files = ctx.file_repo
        .get_trash_paginated(cursor, limit, Some("main"), drive_scope.map(DriveScope::as_str)).await
        .map_err(|e| DriveError::db("DB error when loading paginated trash.", e))?;
    let has_more = files.len() as i64 == limit;
    let next_cursor = files.last().map(|f| f.id);
    let files_val: Vec<Value> = files.into_iter().map(|f| file_to_client_value(&f)).collect();
    Ok(json!({
        "files": files_val,
        "next_cursor": next_cursor,
        "has_more": has_more,
    }))
}

pub async fn get_transfers_paginated(
    ctx: &DriveQueryContext,
    cursor: Option<i64>,
    limit: Option<i64>,
) -> Result<Value, DriveError> {
    let limit = limit.unwrap_or(50).min(200);
    let files = ctx.file_repo
        .get_transfers_paginated(cursor, limit).await
        .map_err(|e| DriveError::db("DB error when loading transfer list.", e))?;
    let file_ids: Vec<i64> = files.iter().map(|f| f.id).collect();
    let parts_counts = ctx.file_repo
        .get_part_counts_for_files(&file_ids).await
        .unwrap_or_default();
    let has_more = files.len() as i64 == limit;
    let next_cursor = files.last().map(|f| f.id);
    Ok(json!({
        "files": map_files_with_progress(files, &*ctx.file_classifier, &parts_counts),
        "next_cursor": next_cursor,
        "has_more": has_more,
    }))
}

pub async fn search_files(ctx: &DriveQueryContext, q: String) -> Result<Value, DriveError> {
    let files = ctx.file_repo
        .search_files(&q, 100).await
        .map_err(|e| DriveError::db("DB error when searching files.", e))?;
    let files: Vec<Value> = files.into_iter().map(|f| file_to_client_value(&f)).collect();
    Ok(json!({ "files": files }))
}

pub async fn get_stats(
    ctx: &DriveQueryContext,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let scope = drive_scope.map(DriveScope::as_str);

    if let Some(scope) = scope {
        if let Some(stats) = ctx.stats_cache_repo
            .get_drive_stats_cache(scope).await
            .map_err(|e| DriveError::db("DB error when loading file stats cache.", e))?
        {
            return Ok(json!({
                "total_files": stats.total_files,
                "total_folders": stats.total_folders,
                "total_size": stats.total_size,
                "trash_count": stats.trash_count,
            }));
        }
    }

    let (total_files, total_size, trash_count) = ctx.file_repo
        .get_file_stats(scope).await
        .map_err(|e| DriveError::db("DB error when loading file stats.", e))?;
    let folders = ctx.folder_repo
        .get_all_folders(scope).await
        .map_err(|e| DriveError::db("DB error when loading folder stats.", e))?;

    Ok(json!({
        "total_files": total_files,
        "total_folders": folders.len(),
        "total_size": total_size,
        "trash_count": trash_count,
    }))
}

pub async fn get_trash(ctx: &DriveQueryContext) -> Result<Value, DriveError> {
    let all = ctx.file_repo
        .get_all_files().await
        .map_err(|e| DriveError::db("DB error when loading trash.", e))?;
    let files: Vec<Value> = all
        .into_iter()
        .filter(|f| f.status == "trashed")
        .map(|f| file_to_client_value(&f))
        .collect();
    Ok(json!({ "files": files }))
}

pub async fn retrieve_thumbnail(
    ctx: &DriveQueryContext,
    file_id: i64,
) -> Result<Value, DriveError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::io::Read;

    let thumb_path = ctx.thumbnail_dir.join(format!("{file_id}.png"));
    if !thumb_path.exists() {
        return Ok(json!({ "thumbnail": null }));
    }

    let mut buf = Vec::new();
    std::fs::File::open(&thumb_path)
        .and_then(|mut f| f.read_to_end(&mut buf))
        .map_err(|e| DriveError::io("Cannot read thumbnail.", e))?;

    let b64 = STANDARD.encode(&buf);
    Ok(json!({ "thumbnail": format!("data:image/png;base64,{b64}") }))
}

pub async fn get_video_full_metadata(
    ctx: &DriveQueryContext,
    file_id: i64,
) -> Result<Value, DriveError> {
    if let Some(video) = ctx.file_repo
        .get_video_file(file_id).await
        .map_err(|e| DriveError::db("DB error when fetching video metadata.", e))?
    {
        return serde_json::to_value(video)
            .map_err(|e| DriveError::internal("Cannot serialize video metadata.", e));
    }
    if let Some(audio) = ctx.file_repo
        .get_audio_file(file_id).await
        .map_err(|e| DriveError::db("DB error when fetching audio metadata.", e))?
    {
        return serde_json::to_value(audio)
            .map_err(|e| DriveError::internal("Cannot serialize audio metadata.", e));
    }
    Ok(json!(null))
}

pub async fn get_folders(
    ctx: &DriveQueryContext,
    drive_scope: Option<DriveScope>,
) -> Result<Value, DriveError> {
    let folders = ctx.folder_repo
        .get_all_folders(drive_scope.map(DriveScope::as_str)).await
        .map_err(|e| DriveError::db("DB error when loading folder list.", e))?;
    let folders_val: Vec<Value> = folders.iter().map(folder_to_client_value).collect();
    Ok(json!({ "folders": folders_val }))
}
