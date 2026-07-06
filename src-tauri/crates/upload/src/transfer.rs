use std::{collections::BTreeMap, ops::Range, path::PathBuf, time::Duration};

use serde_json::{json, Value};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};
use tracing::{error, info};

use omega_drive_gateway::{
    core::{
        config::DEFAULT_DISCORD_PARTS_PER_MESSAGE,
        engine_context::IntegrityService,
        file_types::FileType,
        scope::DriveScope,
    },
    provider::provider_types::{RemoteFolderRef, RemoteUploadTarget, UploadPartRequest},
    upload::upload_plan::{ProviderType, UploadPlan},
};

use tokio_util::sync::CancellationToken;

use crate::{
    context::UploadContext,
    coordinator::{run_upload, UploadDataSource},
    error::{UploadError, UploadResult},
    metadata::extract_full_metadata,
    persistence, plan,
    provider_dispatch::{self, UploadedPart},
    types::{SenderEntry, TransferType},
};

const PART_TYPE_ORIGINAL_CHUNK: &str = "chunk";
const SHARED_BATCH_INLINE_ENTRY_BYTES_CAP: u64 = 8 * 1024 * 1024;
const SHARED_BATCH_INLINE_AGGREGATE_BYTES_CAP: u64 = 24 * 1024 * 1024;
const SHARED_BATCH_READ_BUFFER_BYTES: usize = 256 * 1024;

struct SharedBatchCandidate {
    file_path: PathBuf,
    prepared: plan::PreparedUploadPlan,
    file_type: FileType,
}

#[derive(Clone)]
struct SharedBatchEntry {
    file_path: PathBuf,
    prepared: plan::PreparedUploadPlan,
    record: persistence::UploadRecordContext,
}

#[derive(Clone, Copy, Debug)]
struct SharedBatchSizing {
    bytes: u64,
    limit_bytes: u64,
}

async fn resolve_upload_plan(
    ctx: &UploadContext,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> UploadResult<UploadPlan> {
    if let Some(plan) = upload_plan {
        return Ok(plan);
    }
    if let Some(pid) = profile_id {
        let profile = ctx.upload_profile_repo.get_profile_by_id(pid).await
            .map_err(|e| UploadError::db("Failed to load upload profile.", e))?;
        return Ok(profile.map(|p| p.plan).unwrap_or_else(omega_drive_gateway::upload::upload_plan::balanced_upload_plan));
    }

    let profiles = ctx.upload_profile_repo.get_upload_profiles().await
        .map_err(|e| UploadError::db("Failed to load upload profiles.", e))?;
    Ok(profiles
        .into_iter()
        .next()
        .map(|p| p.plan)
        .unwrap_or_else(omega_drive_gateway::upload::upload_plan::balanced_upload_plan))
}

fn has_heavy_derivatives(plan: &UploadPlan) -> bool {
    plan.derivatives
        .web_preview
        .as_ref()
        .map(|cfg| cfg.enabled)
        .unwrap_or(false)
        || plan
            .derivatives
            .zip_package
            .as_ref()
            .map(|cfg| cfg.enabled)
            .unwrap_or(false)
}

fn supports_shared_discord_batch(plan: &UploadPlan, prepared: &plan::PreparedUploadPlan) -> bool {
    !has_heavy_derivatives(plan)
        && prepared.total_parts == 1
        && prepared.total_bytes <= SHARED_BATCH_INLINE_ENTRY_BYTES_CAP
        && prepared.providers.len() == 1
        && prepared.providers[0] == ProviderType::Discord
}

fn build_shared_batch_target_name(_file_type: FileType) -> String {
    format!("batch-{}.bin", uuid::Uuid::new_v4().simple(),)
}

fn effective_shared_batch_limit(limit_bytes: u64, entry_bytes: u64) -> u64 {
    limit_bytes.max(entry_bytes).max(1)
}

fn shared_batch_sizing(entry: &SharedBatchEntry) -> SharedBatchSizing {
    let configured_limit = entry
        .prepared
        .provider_settings
        .get("discord")
        .map(|settings| settings.chunk_size)
        .unwrap_or(entry.prepared.base_chunk_size);

    SharedBatchSizing {
        bytes: entry.prepared.total_bytes,
        limit_bytes: effective_shared_batch_limit(
            configured_limit.min(SHARED_BATCH_INLINE_AGGREGATE_BYTES_CAP),
            entry.prepared.total_bytes,
        ),
    }
}

async fn read_shared_batch_file_payload(
    integrity: &dyn IntegrityService,
    path: &PathBuf,
    expected_bytes: u64,
) -> UploadResult<(Vec<u8>, String)> {
    let file = File::open(path)
        .await
        .map_err(|err| UploadError::io("Failed to open small file for batch upload", err))?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::with_capacity(expected_bytes.min(usize::MAX as u64) as usize);
    let mut hasher = integrity.create_hasher();
    let mut buffer = vec![0u8; SHARED_BATCH_READ_BUFFER_BYTES];

    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|err| UploadError::io("Failed to read small file for batch upload", err))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        bytes.extend_from_slice(&buffer[..read]);
    }

    if bytes.len() as u64 != expected_bytes {
        return Err(UploadError::conflict(format!(
            "Shared batch source changed while reading: expected {} bytes, got {}",
            expected_bytes,
            bytes.len()
        )));
    }

    Ok((bytes, hasher.finalize_hex()))
}

async fn cleanup_failed_shared_batch_upload(
    ctx: &UploadContext,
    shared_entries: &[SharedBatchEntry],
    upload_target: &RemoteUploadTarget,
) {
    let cleanup_target = match upload_target {
        RemoteUploadTarget::DiscordThread { thread_id, .. } => {
            omega_drive_gateway::provider::provider_types::RemoteObjectRef::DiscordThread {
                thread_id: *thread_id,
            }
        }
    };
    let file_ids: Vec<i64> = shared_entries
        .iter()
        .map(|entry| entry.record.file_sqlite_id)
        .collect();

    if let Err(err) =
        persistence::cleanup_failed_shared_batch_group(ctx, &file_ids, &cleanup_target).await
    {
        error!("Failed to clean up shared batch artifacts: {}", err);
    }
}

fn partition_shared_batch_sizes(
    entries: &[SharedBatchSizing],
    max_count: usize,
) -> Vec<Range<usize>> {
    if entries.is_empty() {
        return Vec::new();
    }

    let max_count = max_count.max(1);
    let mut ranges = Vec::new();
    let mut start = 0usize;

    while start < entries.len() {
        let mut end = start;
        let mut batch_bytes = 0u64;
        let mut batch_limit = u64::MAX;

        while end < entries.len() && end - start < max_count {
            let entry = entries[end];
            let entry_limit = effective_shared_batch_limit(entry.limit_bytes, entry.bytes);
            let next_limit = batch_limit.min(entry_limit);
            let next_bytes = batch_bytes.saturating_add(entry.bytes);

            if end > start && next_bytes > next_limit {
                break;
            }

            batch_limit = next_limit;
            batch_bytes = next_bytes;
            end += 1;
        }

        ranges.push(start..end);
        start = end;
    }

    ranges
}

async fn finalize_shared_batch_entry(
    state: &UploadContext,
    entry: &SharedBatchEntry,
) -> UploadResult<()> {
    if entry.prepared.is_video || entry.prepared.is_audio || entry.prepared.is_image {
        let meta_res = tokio::time::timeout(
            Duration::from_secs(90),
            extract_full_metadata(state, &entry.file_path),
        )
        .await;

        let (full_meta, mut duration) = match meta_res {
            Ok(Ok(metadata)) => {
                let duration = metadata
                    .format
                    .duration
                    .as_ref()
                    .and_then(|value| value.parse::<f64>().ok())
                    .unwrap_or(0.0);
                (Some(metadata), duration)
            }
            _ => (None, 0.0),
        };

        if duration < 1.0 {
            if let Ok(metadata) = tokio::fs::metadata(&entry.file_path).await {
                if metadata.len() > 500 * 1024 * 1024 {
                    duration = 7200.0;
                }
            }
            if duration < 1.0 {
                duration = 30.0;
            }
        }

        let _ = persistence::persist_metadata_and_duration(
            state,
            entry.record.file_sqlite_id,
            full_meta.as_ref(),
            (duration > 0.0).then_some(duration),
        )
        .await;
    }

    persistence::mark_status(state, entry.record.file_sqlite_id, "ready").await
}

async fn run_upload_batch(
    ctx: UploadContext,
    file_paths: Vec<String>,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    session_id: String,
    upload_plan: UploadPlan,
    cancel_token: CancellationToken,
) -> UploadResult<()> {
    let mut candidates = Vec::with_capacity(file_paths.len());
    let mut fallback = Vec::new();

    for file_path in file_paths {
        let path = PathBuf::from(&file_path);
        let source = plan::read_source_info(&path).await?;
        let prepared = plan::build_execution_plan(&ctx, &source, &upload_plan).await?;

        if supports_shared_discord_batch(&upload_plan, &prepared) {
            candidates.push(SharedBatchCandidate {
                file_path: path,
                prepared,
                file_type: ctx.file_classifier.file_type_from_filename(&source.filename),
            });
        } else {
            fallback.push(path);
        }
    }

    if candidates.len() >= 2 {
        let remote_folder: Option<RemoteFolderRef> = None;
        let mut groups = BTreeMap::<String, Vec<SharedBatchCandidate>>::new();
        for candidate in candidates {
            groups
                .entry(candidate.file_type.shared_drive_channel().to_string())
                .or_default()
                .push(candidate);
        }

        let gateway = ctx
            .provider_runtime
            .remote_folder_registry
            .get(drive_scope.remote_folder_provider_id())
            .ok_or_else(|| {
                UploadError::provider_message("Discord upload target gateway not available")
            })?;

        for candidates in groups.into_values() {
            if candidates.len() < 2 {
                fallback.extend(candidates.into_iter().map(|candidate| candidate.file_path));
                continue;
            }

            let batch_name = build_shared_batch_target_name(candidates[0].file_type);
            let upload_target = gateway
                .ensure_upload_target(&batch_name, remote_folder.as_ref())
                .await
                .map_err(|err| {
                    UploadError::provider("Failed to create shared batch target", err)
                })?;
            let (thread_id_str, thread_to_archive) = match &upload_target {
                RemoteUploadTarget::DiscordThread {
                    thread_id,
                    archive_on_finalize,
                } => (
                    (*thread_id).to_string(),
                    archive_on_finalize.then_some(*thread_id),
                ),
            };

            let mut shared_entries = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                let record = persistence::ensure_upload_target_with_existing_remote(
                    &ctx,
                    &candidate.prepared.file_path_str,
                    &candidate.prepared.filename,
                    candidate.prepared.total_bytes,
                    folder_id,
                    drive_scope,
                    upload_target.clone(),
                    thread_id_str.clone(),
                    None,
                )
                .await?;
                shared_entries.push(SharedBatchEntry {
                    file_path: candidate.file_path,
                    prepared: candidate.prepared,
                    record,
                });
            }

            let batch_size = upload_plan
                .advanced
                .as_ref()
                .and_then(|cfg| cfg.discord_batch_size)
                .unwrap_or(DEFAULT_DISCORD_PARTS_PER_MESSAGE as u32)
                .clamp(1, 10) as usize;
            let discord_gateway = ctx
                .provider_runtime
                .part_store_registry
                .get("discord")
                .ok_or_else(|| {
                    UploadError::provider_message("discord part store gateway not available")
                })?;

            let upload_shared_result: UploadResult<()> = async {
                let batch_ranges = partition_shared_batch_sizes(
                    &shared_entries
                        .iter()
                        .map(shared_batch_sizing)
                        .collect::<Vec<_>>(),
                    batch_size,
                );

                for range in batch_ranges {
                    let batch = &shared_entries[range];
                    let caption = String::new();
                    let mut requests = Vec::with_capacity(batch.len());
                    let mut fallback_attachment_names = Vec::with_capacity(batch.len());
                    let mut file_checksums = Vec::with_capacity(batch.len());

                    for entry in batch {
                        let (bytes, checksum) = read_shared_batch_file_payload(
                            ctx.engine.integrity.as_ref(),
                            &entry.file_path,
                            entry.prepared.total_bytes,
                        )
                        .await?;
                        file_checksums.push(checksum);

                        let attachment_name = provider_dispatch::build_discord_attachment_name(
                            &entry.prepared.filename,
                            1,
                        );
                        fallback_attachment_names.push(attachment_name.clone());
                        requests.push(UploadPartRequest {
                            target: upload_target.clone(),
                            data: bytes,
                            file_name: attachment_name,
                            caption: caption.clone(),
                            part_num: 1,
                            telegram_progress_tx: None,
                        });
                    }

                    let receipts =
                        discord_gateway
                            .upload_parts_batch(requests)
                            .await
                            .map_err(|err| {
                                UploadError::provider("Failed to upload shared Discord batch", err)
                            })?;

                    for (((entry, fallback_attachment_name), receipt), checksum) in batch
                        .iter()
                        .zip(fallback_attachment_names.into_iter())
                        .zip(receipts.into_iter())
                        .zip(file_checksums.into_iter())
                    {
                        let result = UploadedPart {
                            message_id: receipt.message_id,
                            platform: receipt.platform,
                            attachment_name: receipt
                                .attachment_name
                                .or(Some(fallback_attachment_name)),
                            part_index: 1,
                            size: receipt.size,
                            logical_size: Some(entry.prepared.total_bytes),
                            checksum: Some(checksum.clone()),
                        };

                        persistence::update_file_checksum(
                            &ctx,
                            entry.record.file_sqlite_id,
                            &checksum,
                        )
                        .await?;

                        persistence::persist_part_results(
                            &ctx,
                            entry.record.file_sqlite_id,
                            &[result],
                            PART_TYPE_ORIGINAL_CHUNK,
                        )
                        .await?;
                    }
                }
                Ok(())
            }
            .await;

            if let Err(err) = upload_shared_result {
                cleanup_failed_shared_batch_upload(&ctx, &shared_entries, &upload_target).await;
                for entry in &shared_entries {
                    let _ =
                        persistence::mark_failure(&ctx, entry.record.file_sqlite_id, &err).await;
                }
                return Err(err);
            }

            for entry in &shared_entries {
                finalize_shared_batch_entry(&ctx, entry).await?;
            }

            if let Some(thread_id) = thread_to_archive {
                if let Some(gateway) = ctx.provider_runtime.remote_object_registry.get("discord") {
                    let _ = gateway
                        .archive_object(
                            &omega_drive_gateway::provider::provider_types::RemoteObjectRef::DiscordThread {
                                thread_id,
                            },
                        )
                        .await;
                }
            }
        }
    } else {
        fallback.extend(candidates.into_iter().map(|candidate| candidate.file_path));
    }

    for (index, path) in fallback.into_iter().enumerate() {
        if cancel_token.is_cancelled() {
            break;
        }
        let child_session_id = format!("{session_id}:{}", index + 1);
        if let Err(err) = run_upload(
            ctx.clone(),
            UploadDataSource::File(path),
            folder_id,
            drive_scope,
            child_session_id,
            upload_plan.clone(),
            None,
            cancel_token.clone(),
            None,
            None,
        )
        .await
        {
            error!("Batch fallback upload error: {}", err);
        }
    }

    Ok(())
}

pub async fn upload_file_native(
    ctx: &UploadContext,
    file_path: String,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> UploadResult<Value> {
    let path = PathBuf::from(file_path);
    let fid = folder_id;

    let ctx_clone = ctx.clone();
    let sid_clone = session_id.clone();
    let senders_ref = std::sync::Arc::clone(&ctx.senders);
    let sid_for_cleanup = session_id.clone();

    let resolved_plan = resolve_upload_plan(ctx, profile_id, upload_plan).await?;
    let cancel_token = CancellationToken::new();
    let token_for_task = cancel_token.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = run_upload(
            ctx_clone,
            UploadDataSource::File(path),
            fid,
            drive_scope,
            sid_clone,
            resolved_plan,
            None,
            token_for_task,
            None,
            None,
        )
        .await
        {
            error!("Upload error: {}", e);
        }
        let mut map = senders_ref.lock().await;
        map.remove(&sid_for_cleanup);
    });

    let mut map = ctx.senders.lock().await;
    map.insert(
        session_id,
        SenderEntry {
            handle,
            transfer_type: TransferType::Upload { file_id: None },
            cancel_token,
        },
    );

    Ok(json!({ "status": "started" }))
}

pub async fn upload_files_from_paths(
    ctx: &UploadContext,
    file_paths: Vec<String>,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> UploadResult<Value> {
    let ctx_clone = ctx.clone();
    let sid_clone = session_id.clone();
    let senders_ref = std::sync::Arc::clone(&ctx.senders);
    let sid_for_cleanup = session_id.clone();
    let count = file_paths.len();
    let resolved_plan = resolve_upload_plan(ctx, profile_id, upload_plan).await?;
    let cancel_token = CancellationToken::new();
    let token_for_task = cancel_token.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = run_upload_batch(
            ctx_clone,
            file_paths,
            folder_id,
            drive_scope,
            sid_clone,
            resolved_plan,
            token_for_task,
        )
        .await
        {
            error!("Batch upload error: {}", e);
        }
        let mut map = senders_ref.lock().await;
        map.remove(&sid_for_cleanup);
    });

    let mut map = ctx.senders.lock().await;
    map.insert(
        session_id,
        SenderEntry {
            handle,
            transfer_type: TransferType::Upload { file_id: None },
            cancel_token,
        },
    );

    Ok(json!({ "status": "started", "count": count }))
}

pub async fn upload_file_from_path(
    ctx: &UploadContext,
    file_path: String,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    session_id: String,
    profile_id: Option<i64>,
    upload_plan: Option<UploadPlan>,
) -> UploadResult<Value> {
    upload_file_native(
        ctx,
        file_path,
        folder_id,
        drive_scope,
        session_id,
        profile_id,
        upload_plan,
    )
    .await
}

pub async fn pause_upload(ctx: &UploadContext, session_id: String) -> UploadResult<Value> {
    let map = ctx.senders.lock().await;
    if let Some(entry) = map.get(&session_id) {
        entry.handle.abort();
        info!("Paused upload: {session_id}");
    }
    Ok(json!({ "success": true, "status": "paused" }))
}

pub async fn resume_upload(
    ctx: &UploadContext,
    session_id: String,
    file_id: i64,
    file_path: String,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
) -> UploadResult<Value> {
    info!("Resume upload (ID: {}): {}", file_id, file_path);

    ctx.file_repo.update_file_local_path(file_id, Some(file_path.as_str())).await
        .map_err(|e| UploadError::db("DB error: failed to update local path.", e))?;

    upload_file_native(
        ctx,
        file_path,
        folder_id,
        drive_scope,
        session_id,
        None,
        None,
    )
    .await
}

pub async fn cancel_transfer(ctx: &UploadContext, session_id: String) -> UploadResult<Value> {
    let mut map = ctx.senders.lock().await;
    if let Some(entry) = map.remove(&session_id) {
        entry.cancel_token.cancel();
        info!("Cancellation token triggered for session: {session_id}");

        entry.handle.abort();

        if let TransferType::Download { path } = entry.transfer_type {
            if path.exists() {
                let _ = std::fs::remove_file(&path);
                info!("Deleted partial download file: {}", path.display());
            }
        }

        info!("Cancelled transfer: {session_id}");
    }
    Ok(json!({ "success": true }))
}

pub async fn cancel_transfer_by_file_id(
    ctx: &UploadContext,
    target_file_id: i64,
) -> UploadResult<Value> {
    let session_id_to_cancel = {
        let map = ctx.senders.lock().await;
        map.iter()
            .find_map(|(sid, entry)| match entry.transfer_type {
                TransferType::Upload { file_id: Some(id) } if id == target_file_id => {
                    Some(sid.clone())
                }
                _ => None,
            })
    };

    if let Some(sid) = session_id_to_cancel {
        info!(
            "Cancelling transfer for file_id {} (session_id: {})",
            target_file_id, sid
        );
        cancel_transfer(ctx, sid).await
    } else {
        Ok(json!({ "success": false, "reason": "session_not_found" }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partition_shared_batch_sizes_respects_max_count() {
        let entries = [
            SharedBatchSizing {
                bytes: 1,
                limit_bytes: 10,
            },
            SharedBatchSizing {
                bytes: 1,
                limit_bytes: 10,
            },
            SharedBatchSizing {
                bytes: 1,
                limit_bytes: 10,
            },
        ];

        let ranges = partition_shared_batch_sizes(&entries, 2);

        assert_eq!(ranges, vec![0..2, 2..3]);
    }

    #[test]
    fn partition_shared_batch_sizes_respects_aggregate_byte_limit() {
        let entries = [
            SharedBatchSizing {
                bytes: 4,
                limit_bytes: 8,
            },
            SharedBatchSizing {
                bytes: 4,
                limit_bytes: 8,
            },
            SharedBatchSizing {
                bytes: 4,
                limit_bytes: 8,
            },
        ];

        let ranges = partition_shared_batch_sizes(&entries, 10);

        assert_eq!(ranges, vec![0..2, 2..3]);
    }

    #[test]
    fn partition_shared_batch_sizes_uses_smallest_limit_in_each_batch() {
        let entries = [
            SharedBatchSizing {
                bytes: 3,
                limit_bytes: 10,
            },
            SharedBatchSizing {
                bytes: 3,
                limit_bytes: 5,
            },
            SharedBatchSizing {
                bytes: 3,
                limit_bytes: 10,
            },
        ];

        let ranges = partition_shared_batch_sizes(&entries, 10);

        assert_eq!(ranges, vec![0..1, 1..2, 2..3]);
    }
}
