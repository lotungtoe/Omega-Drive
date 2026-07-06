use std::collections::HashMap;

use omega_drive_gateway::core::events::OmegaEvent;
use omega_drive_gateway::core::filemeta::FullMetadata;
use omega_drive_gateway::provider::provider_types::{RemoteFolderRef, RemoteObjectRef, RemoteUploadTarget};
use omega_drive_gateway::provider::storage::PartMetadata;
use omega_drive_gateway::core::scope::DriveScope;
pub use omega_drive_gateway::upload::upload_types::{UploadRecordContext, UploadedPart};

use tracing::warn;

use crate::context::UploadContext;
use crate::error::{UploadError, UploadResult};

fn upload_job_state_for_file_status(status: &str) -> &str {
    match status {
        "ready" => "done",
        "processing" => "processing",
        "error" => "error",
        "uploading" => "uploading",
        other => other,
    }
}

fn upload_error_code(err: &UploadError) -> &'static str {
    match err {
        UploadError::Validation { .. } => "validation",
        UploadError::Conflict { .. } => "conflict",
        UploadError::Db { .. } => "db",
        UploadError::Io { .. } => "io",
        UploadError::Provider { .. } => "provider",
        UploadError::Timeout { .. } => "timeout",
        UploadError::Internal { .. } => "internal",
    }
}

fn upload_error_message(err: &UploadError) -> String {
    match err {
        UploadError::Validation { message }
        | UploadError::Conflict { message }
        | UploadError::Timeout { message } => message.clone(),
        UploadError::Db { message, source }
        | UploadError::Io { message, source }
        | UploadError::Provider { message, source }
        | UploadError::Internal { message, source } => match source {
            Some(ref s) if !s.is_empty() => format!("{message} ({s})"),
            _ => message.clone(),
        },
    }
}

fn cleanup_provider_id(object: &RemoteObjectRef) -> &'static str {
    match object {
        RemoteObjectRef::DiscordThread { .. }
        | RemoteObjectRef::DiscordChannel { .. }
        | RemoteObjectRef::DiscordMessage { .. } => "discord",
        RemoteObjectRef::TelegramMessages { .. } => "telegram",
    }
}

fn build_cleanup_part_metadata(file_sqlite_id: i64, uploaded_part: &UploadedPart) -> PartMetadata {
    PartMetadata {
        id: 0,
        file_id: file_sqlite_id,
        platform: uploaded_part.platform.clone(),
        message_id: uploaded_part.message_id.to_string(),
        attachment_name: uploaded_part.attachment_name.clone(),
        part_index: uploaded_part.part_index,
        size: uploaded_part.size as i64,
        part_type: "chunk".to_string(),
        duration: None,
        checksum: uploaded_part.checksum.clone(),
    }
}

pub(crate) fn cleanup_target_for_record(record: &UploadRecordContext) -> Option<RemoteObjectRef> {
    match &record.upload_target {
        RemoteUploadTarget::DiscordThread { thread_id, .. } => {
            Some(RemoteObjectRef::DiscordThread { thread_id: *thread_id })
        }
    }
}

pub(crate) async fn cleanup_failed_upload_artifacts(
    state: &UploadContext,
    file_sqlite_id: i64,
    cleanup_target: Option<RemoteObjectRef>,
) -> UploadResult<()> {
    let parts = state.file_repo.get_original_parts_for_file(file_sqlite_id).await
        .map_err(|e| UploadError::db("Failed to load existing upload parts", e))?;
    let mut cleanup_errors = Vec::new();
    let skip_discord_parts = matches!(cleanup_target, Some(RemoteObjectRef::DiscordThread { .. }));

    if let Some(target) = cleanup_target.as_ref() {
        let provider_id = cleanup_provider_id(target);
        match state.provider_runtime.remote_object_registry.get(provider_id) {
            Some(gateway) => {
                if let Err(err) = gateway.delete_object(target).await {
                    cleanup_errors.push(err.to_string());
                }
            }
            None => cleanup_errors.push(format!("Missing remote cleanup gateway '{}'", provider_id)),
        }
    }

    let mut parts_by_platform = HashMap::<String, Vec<PartMetadata>>::new();
    for part in parts {
        parts_by_platform.entry(part.platform.clone()).or_default().push(part);
    }

    for (platform, provider_parts) in parts_by_platform {
        if skip_discord_parts && platform == "discord" { continue; }
        let Some(gateway) = state.provider_runtime.remote_object_registry.get(&platform) else {
            cleanup_errors.push(format!("Missing remote cleanup gateway '{}' for file {}", platform, file_sqlite_id));
            continue;
        };
        if let Err(err) = gateway.delete_file_artifacts(file_sqlite_id, &provider_parts).await {
            cleanup_errors.push(format!("Provider cleanup '{}' failed for file {}: {}", platform, file_sqlite_id, err));
        }
    }

    if !cleanup_errors.is_empty() {
        return Err(UploadError::provider_message(cleanup_errors.join(" | ")));
    }

    state.file_repo.delete_parts_by_type(file_sqlite_id, "chunk").await
        .map_err(|e| UploadError::db("Failed to clear local upload parts after cleanup", e))?;
    Ok(())
}

pub(crate) async fn cleanup_failed_shared_batch_group(
    state: &UploadContext,
    file_ids: &[i64],
    cleanup_target: &RemoteObjectRef,
) -> UploadResult<()> {
    let provider_id = cleanup_provider_id(cleanup_target);
    let gateway = state.provider_runtime.remote_object_registry.get(provider_id)
        .ok_or_else(|| UploadError::provider_message(format!("Missing remote cleanup gateway '{}' for shared batch", provider_id)))?;
    gateway.delete_object(cleanup_target).await
        .map_err(|e| UploadError::provider("Failed to delete shared batch target", e))?;
    for file_id in file_ids {
        state.file_repo.delete_parts_by_type(*file_id, "chunk").await
            .map_err(|e| UploadError::db(format!("Failed to clear shared-batch parts for file {}", file_id), e))?;
    }
    Ok(())
}

pub(crate) async fn cleanup_uploaded_parts_without_db(
    state: &UploadContext,
    file_sqlite_id: i64,
    uploaded_parts: &[UploadedPart],
) -> UploadResult<()> {
    let mut by_platform = HashMap::<String, Vec<PartMetadata>>::new();
    for uploaded_part in uploaded_parts {
        by_platform.entry(uploaded_part.platform.clone()).or_default()
            .push(build_cleanup_part_metadata(file_sqlite_id, uploaded_part));
    }
    let mut cleanup_errors = Vec::new();
    for (platform, provider_parts) in by_platform {
        if platform == "discord" { continue; }
        let Some(gateway) = state.provider_runtime.remote_object_registry.get(&platform) else {
            cleanup_errors.push(format!("Missing remote cleanup gateway '{}' for inline cleanup", platform));
            continue;
        };
        if let Err(err) = gateway.delete_parts(&provider_parts).await {
            cleanup_errors.push(format!("Inline cleanup for provider '{}' failed: {}", platform, err));
        }
    }
    if cleanup_errors.is_empty() { Ok(()) } else { Err(UploadError::provider_message(cleanup_errors.join(" | "))) }
}

pub async fn ensure_upload_target(
    state: &UploadContext,
    file_path_str: &str,
    filename: &str,
    total_bytes: u64,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    video_duration_sec: Option<f64>,
    video_container: Option<String>,
) -> UploadResult<UploadRecordContext> {
    let (file_sqlite_id, upload_target, thread_id_str, thread_to_archive) = {
        let existing = state.upload_job_repo.get_active_job_by_source_path(file_path_str).await
            .map_err(|e| UploadError::db("Failed to query upload record", e))?;
        if let Some(job) = existing {
            match state.file_repo.get_file_by_id(job.file_id).await {
                Ok(Some(file)) => {
                    let thread_id = file.thread_id.parse::<u64>()
                        .map_err(|_| UploadError::conflict("Invalid channel id stored for upload record"))?;
                    (job.file_id, RemoteUploadTarget::DiscordThread { thread_id, archive_on_finalize: true }, file.thread_id, Some(thread_id))
                }
                _ => return Err(UploadError::conflict("Upload job exists but file record not found")),
            }
        } else {
            let gateway = state.provider_runtime.remote_folder_registry
                .get(drive_scope.remote_folder_provider_id())
                .ok_or_else(|| UploadError::provider_message("Discord upload target gateway not available"))?;
            let target = gateway.ensure_upload_target(filename, None::<&RemoteFolderRef>).await
                .map_err(|e| UploadError::provider("Failed to create upload target", e))?;
            let (tid, to_archive) = match &target {
                RemoteUploadTarget::DiscordThread { thread_id, archive_on_finalize } => {
                    (thread_id.to_string(), archive_on_finalize.then_some(*thread_id))
                }
            };
            let fid = state.file_repo.insert_file(filename, total_bytes as i64, &tid, folder_id, drive_scope.as_str(), None, Some(file_path_str)).await
                .map_err(|e| UploadError::db("Failed to insert upload record", e))?;
            if let Some(dur) = video_duration_sec {
                let _ = state.file_repo.upsert_video_file(
                    fid, Some(dur), None, None, None, None, None, None,
                    video_container.as_deref(),
                ).await.map_err(|e| warn!("Failed to persist video metadata for {}: {e}", fid));
            }
            state.upload_job_repo.upsert_job(fid, file_path_str, "uploading", 0).await
                .map_err(|e| UploadError::db("Failed to persist upload job", e))?;
            state.file_repo.update_file_status(fid, "uploading").await
                .map_err(|e| UploadError::db("Failed to mark upload as running", e))?;
            state.events.emit(OmegaEvent::FilesTableChanged);
            (fid, target, tid, to_archive)
        }
    };

    let existing_parts = state.file_repo.get_original_parts_for_file(file_sqlite_id).await
        .map_err(|e| UploadError::db("Failed to load existing upload parts", e))?;

    Ok(UploadRecordContext { file_sqlite_id, upload_target, thread_id_str, thread_to_archive, existing_parts })
}

pub(crate) async fn ensure_upload_target_with_existing_remote(
    state: &UploadContext,
    file_path_str: &str,
    filename: &str,
    total_bytes: u64,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    upload_target: RemoteUploadTarget,
    thread_id_str: String,
    thread_to_archive: Option<u64>,
) -> UploadResult<UploadRecordContext> {
    let file_sqlite_id = {
        let existing = state.upload_job_repo.get_active_job_by_source_path(file_path_str).await
            .map_err(|e| UploadError::db("Failed to query upload record", e))?;
        if let Some(job) = existing {
            job.file_id
        } else {
            let fid = state.file_repo.insert_file(filename, total_bytes as i64, &thread_id_str, folder_id, drive_scope.as_str(), None, Some(file_path_str)).await
                .map_err(|e| UploadError::db("Failed to insert upload record", e))?;
            state.upload_job_repo.upsert_job(fid, file_path_str, "uploading", 0).await
                .map_err(|e| UploadError::db("Failed to persist upload job", e))?;
            state.file_repo.update_file_status(fid, "uploading").await
                .map_err(|e| UploadError::db("Failed to mark upload as running", e))?;
            state.events.emit(OmegaEvent::FilesTableChanged);
            fid
        }
    };

    let existing_parts = state.file_repo.get_original_parts_for_file(file_sqlite_id).await
        .map_err(|e| UploadError::db("Failed to load existing upload parts", e))?;
    Ok(UploadRecordContext { file_sqlite_id, upload_target, thread_id_str, thread_to_archive, existing_parts })
}

pub(crate) async fn reserve_attachment_upload_target(
    state: &UploadContext,
    file_path_str: &str,
    filename: &str,
    total_bytes: u64,
    parent_id: i64,
    attachment_type: &str,
) -> UploadResult<UploadRecordContext> {
    let (file_sqlite_id, upload_target, thread_id_str, thread_to_archive) = {
        let existing = state.upload_job_repo.get_active_job_by_source_path(file_path_str).await
            .map_err(|e| UploadError::db("Failed to query upload record", e))?;
        if let Some(job) = existing {
            match state.file_repo.get_file_by_id(job.file_id).await {
                Ok(Some(file)) => {
                    let thread_id = file.thread_id.parse::<u64>()
                        .map_err(|_| UploadError::conflict("Invalid channel id stored for upload record"))?;
                    (job.file_id, RemoteUploadTarget::DiscordThread { thread_id, archive_on_finalize: true }, file.thread_id, Some(thread_id))
                }
                _ => return Err(UploadError::conflict("Upload job exists but attachment file record not found")),
            }
        } else {
            let parent_file = state.file_repo.get_file_by_id(parent_id).await
                .map_err(|e| UploadError::db("Failed to load attachment parent", e))?
                .ok_or_else(|| UploadError::conflict("Attachment parent file not found"))?;
            let parent_folder_id = parent_file.folder_id;
            let parent_scope = parent_file.drive_scope.parse::<DriveScope>().unwrap_or_default();
            let gateway = state.provider_runtime.remote_folder_registry
                .get(parent_scope.remote_folder_provider_id())
                .ok_or_else(|| UploadError::provider_message("Discord upload target gateway not available"))?;
            let target = gateway.ensure_upload_target(filename, None::<&RemoteFolderRef>).await
                .map_err(|e| UploadError::provider("Failed to create upload target", e))?;
            let (tid, to_archive) = match &target {
                RemoteUploadTarget::DiscordThread { thread_id, archive_on_finalize } => {
                    (thread_id.to_string(), archive_on_finalize.then_some(*thread_id))
                }
            };
            let fid = state.file_repo.insert_attachment_file(filename, total_bytes as i64, &tid, parent_folder_id, parent_scope.as_str(), None).await
                .map_err(|e| UploadError::db("Failed to insert attachment upload record", e))?;
            state.upload_job_repo.upsert_job(fid, file_path_str, "uploading", 0).await
                .map_err(|e| UploadError::db("Failed to persist upload job", e))?;
            state.file_repo.update_file_status(fid, "uploading").await
                .map_err(|e| UploadError::db("Failed to mark upload as running", e))?;
            let _ = attachment_type;
            state.events.emit(OmegaEvent::FilesTableChanged);
            (fid, target, tid, to_archive)
        }
    };

    let existing_parts = state.file_repo.get_original_parts_for_file(file_sqlite_id).await
        .map_err(|e| UploadError::db("Failed to load existing upload parts", e))?;
    Ok(UploadRecordContext { file_sqlite_id, upload_target, thread_id_str, thread_to_archive, existing_parts })
}

pub async fn persist_part_results(
    state: &UploadContext,
    file_sqlite_id: i64,
    part_results: &[UploadedPart],
    part_type: &str,
) -> UploadResult<()> {
    let file_exists = state.file_repo.get_file_by_id(file_sqlite_id).await
        .map_err(|_| ())
        .ok()
        .flatten()
        .is_some();
    if !file_exists {
        warn!("File record with ID={} not found. Skipping part persistence.", file_sqlite_id);
        return Ok(());
    }
    for result in part_results {
        if let Err(err) = state.file_repo.insert_part(
            file_sqlite_id,
            &result.platform,
            &result.message_id.to_string(),
            result.attachment_name.as_deref(),
            result.part_index,
            result.size as i64,
            part_type,
            result.checksum.clone(),
        ).await {
            let file_exists = state.file_repo.get_file_by_id(file_sqlite_id).await.unwrap_or(None).is_some();
            if !file_exists {
                warn!("File record ID={} was deleted during upload. Ignoring part persistence error.", file_sqlite_id);
                return Ok(());
            }
            return Err(UploadError::db("Failed to persist upload part", err));
        }
    }
    Ok(())
}

pub async fn mark_status(
    state: &UploadContext,
    file_sqlite_id: i64,
    status: &str,
) -> UploadResult<()> {
    state.file_repo.update_file_status(file_sqlite_id, status).await
        .map_err(|e| UploadError::db(format!("Failed to update upload status to {status}"), e))?;
    state.upload_job_repo.update_state(file_sqlite_id, upload_job_state_for_file_status(status), None, None).await
        .map_err(|e| UploadError::db("Failed to sync upload job state", e))?;
    state.events.emit(OmegaEvent::FilesTableChanged);
    if let Some(ref backup) = state.backup_service {
        backup.try_capture_file(file_sqlite_id, status);
    }
    Ok(())
}

pub async fn mark_failure(
    state: &UploadContext,
    file_sqlite_id: i64,
    err: &UploadError,
) -> UploadResult<()> {
    state.file_repo.update_file_status(file_sqlite_id, "error").await
        .map_err(|e| UploadError::db("Failed to mark upload as error", e))?;
    let message = upload_error_message(err);
    state.upload_job_repo.update_state(file_sqlite_id, "error", Some(&message), Some(upload_error_code(err))).await
        .map_err(|e| UploadError::db("Failed to sync upload error state", e))?;
    state.events.emit(OmegaEvent::FilesTableChanged);
    Ok(())
}

pub(crate) async fn persist_metadata_and_duration(
    state: &UploadContext,
    file_sqlite_id: i64,
    full_meta: Option<&FullMetadata>,
    duration: Option<f64>,
) -> UploadResult<()> {
    let mut parsed_summary = None;
    if let Some(metadata) = full_meta {
        let json = serde_json::to_string(metadata)
            .map_err(|e| UploadError::internal("Failed to serialize full metadata", e))?;
        parsed_summary = state.media_parser.parse_media_summary(&json);
    }

    let normalized_duration = duration
        .filter(|v| v.is_finite() && *v > 0.0)
        .or_else(|| parsed_summary.as_ref().and_then(|m| m.duration_sec));

    let kind = state.file_repo.get_file_kind(file_sqlite_id).await
        .map_err(|e| UploadError::db("Failed to query file kind for metadata persistence", e))?
        .unwrap_or_else(|| "unknown".to_string());

    match kind.as_str() {
        "audio" => {
            state.file_repo.upsert_audio_file(
                file_sqlite_id,
                normalized_duration,
                parsed_summary.as_ref().and_then(|m| m.audio_bitrate_bps),
                parsed_summary.as_ref().and_then(|m| m.sample_rate_hz),
                parsed_summary.as_ref().and_then(|m| m.channels),
                parsed_summary.as_ref().and_then(|m| m.audio_codec_only.as_deref()),
                parsed_summary.as_ref().and_then(|m| m.container.as_deref()),
            ).await
            .map_err(|e| UploadError::db("Failed to persist audio metadata", e))?;
        }
        "image" => {
            state.file_repo.upsert_image_file(
                file_sqlite_id,
                parsed_summary.as_ref().and_then(|m| m.width),
                parsed_summary.as_ref().and_then(|m| m.height),
                parsed_summary.as_ref().and_then(|m| m.container.as_deref()),
                None, None,
            ).await
            .map_err(|e| UploadError::db("Failed to persist image metadata", e))?;
        }
        _ => {
            state.file_repo.upsert_video_file(
                file_sqlite_id,
                normalized_duration,
                parsed_summary.as_ref().and_then(|m| m.width),
                parsed_summary.as_ref().and_then(|m| m.height),
                None,
                parsed_summary.as_ref().and_then(|m| m.bitrate_bps),
                parsed_summary.as_ref().and_then(|m| m.video_codec.as_deref()),
                parsed_summary.as_ref().and_then(|m| m.audio_codec.as_deref()),
                parsed_summary.as_ref().and_then(|m| m.container.as_deref()),
            ).await
            .map_err(|e| UploadError::db("Failed to persist video metadata", e))?;
        }
    }
    Ok(())
}

pub(crate) async fn update_file_checksum(
    state: &UploadContext,
    file_id: i64,
    checksum: &str,
) -> UploadResult<()> {
    state.file_repo.update_file_checksum(file_id, checksum).await
        .map_err(|e| UploadError::db("Failed to update final file checksum", e))?;
    Ok(())
}
