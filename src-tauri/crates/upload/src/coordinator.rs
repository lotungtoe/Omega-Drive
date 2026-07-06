use std::{
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use tokio::{
    fs,
    io::{AsyncReadExt, BufReader},
    sync::{mpsc::UnboundedSender, Semaphore},
};
use tokio_util::sync::CancellationToken;



use tracing::{info, warn};

use omega_drive_gateway::{
    core::scope::DriveScope,
    provider::provider_types::{RemoteObjectRef, RemoteUploadTarget},
    upload::upload_plan::{AdvancedLimits, DerivativesPlan, ProviderType, UploadPlan, UploadStrategy},
};


use crate::{
    context::UploadContext,
    derivative_upload::upload_derivative_file,
    error::{UploadError, UploadResult},
    metadata::extract_full_metadata,
    persistence::{self, UploadRecordContext},
    plan::{self, UploadSourceInfo},
    progress::app_event_emitter,
    provider_dispatch,
    session::UploadSessionTracker,
};

const PART_TYPE_ORIGINAL_CHUNK: &str = "chunk";
const PART_TYPE_ZIP_CHUNK: &str = "zip_chunk";

pub enum UploadDataSource {
    File(PathBuf),
    Stream {
        info: UploadSourceInfo,
        reader: Box<dyn tokio::io::AsyncRead + Unpin + Send + 'static>,
    },
}

pub async fn run_upload(
    state: UploadContext,
    source: UploadDataSource,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    session_id: String,
    upload_plan: UploadPlan,
    attachment_parent: Option<(i64, String)>,
    cancel_token: CancellationToken,
    video_duration_sec: Option<f64>,
    video_container: Option<String>,
) -> UploadResult<i64> {
    let (source_info, source_file_path, source) = match source {
        UploadDataSource::File(path) => {
            let info = plan::read_source_info(&path).await?;
            let fp = path.clone();
            (info, Some(fp), UploadDataSource::File(path))
        }
        UploadDataSource::Stream { info, reader } => {
            let cloned = info.clone();
            (cloned, None, UploadDataSource::Stream { info, reader })
        }
    };
    let ui_emitter = app_event_emitter(&state);
    let session =
        UploadSessionTracker::new(ui_emitter, session_id.clone(), source_info.filename.clone());

    session.emit_preparing();

    let prepared = plan::build_execution_plan(&state, &source_info, &upload_plan).await?;
    session.configure(
        prepared.total_parts,
        prepared.total_bytes,
        prepared.per_provider_bytes.clone(),
    );

    let record = if let Some((parent_id, attachment_type)) = &attachment_parent {
        persistence::reserve_attachment_upload_target(
            &state,
            &prepared.file_path_str,
            &prepared.filename,
            prepared.total_bytes,
            *parent_id,
            attachment_type,
        )
        .await?
    } else {
        persistence::ensure_upload_target(
            &state,
            &prepared.file_path_str,
            &prepared.filename,
            prepared.total_bytes,
            folder_id,
            drive_scope,
            video_duration_sec,
            video_container,
        )
        .await?
    };

    {
        let mut map = state.senders.lock().await;
        if let Some(entry) = map.get_mut(&session_id) {
            if let crate::types::TransferType::Upload { ref mut file_id } =
                entry.transfer_type
            {
                *file_id = Some(record.file_sqlite_id);
            }
        }
    }
    session.set_file_id(record.file_sqlite_id);

    for part in &record.existing_parts {
        session.record_existing_part(&part.platform, part.size.max(0) as u64);
    }

    let tg_authorized = match state
        .provider_runtime
        .provider_admin_registry
        .get("telegram")
    {
        Some(gateway) => gateway
            .connection_status()
            .await
            .map(|status| status.authorized)
            .unwrap_or(false),
        None => false,
    };

    let sem = Arc::new(Semaphore::new(prepared.parallel_sends));
    let telegram_progress_tx = session.spawn_telegram_progress_listener();

    let final_checksum = match run_original_upload(
        &state,
        &prepared,
        &record,
        &session,
        source,
        tg_authorized,
        telegram_progress_tx,
        sem,
        upload_plan.advanced.clone(),
        cancel_token.clone(),
        video_duration_sec,
    )
    .await
    {
        Ok(final_checksum) => final_checksum,
        Err(err) => {
            session.emit_failed(&format_upload_error_detail(&err));
            if let Err(cleanup_err) = persistence::cleanup_failed_upload_artifacts(
                &state,
                record.file_sqlite_id,
                persistence::cleanup_target_for_record(&record),
            )
            .await
            {
                warn!(
                    "Failed to clean up partial upload artifacts for file {}: {}",
                    record.file_sqlite_id, cleanup_err
                );
            }
            let _ = persistence::mark_failure(&state, record.file_sqlite_id, &err).await;
            return Err(err);
        }
    };

    session.emit_finalizing_integrity();
    if let Err(err) =
        persistence::update_file_checksum(&state, record.file_sqlite_id, &final_checksum).await
    {
        session.emit_failed(&format_upload_error_detail(&err));
        let _ = persistence::mark_failure(&state, record.file_sqlite_id, &err).await;
        return Err(err);
    }

    emit_telegram_backed_manifest_note(&state, &prepared, &record).await;

    if !upload_plan.audio_attachments.is_empty() {
        let thread_id = record.thread_id_str.parse::<u64>().unwrap_or_default();
        let mut audio_file_ids = Vec::new();
        for audio_path in &upload_plan.audio_attachments {
            let audio_file_id = upload_hidden_audio_file(
                &state,
                audio_path,
                &record.thread_id_str,
                thread_id,
                drive_scope,
                folder_id,
                tg_authorized,
                prepared.strategy,
                &prepared.providers,
            )
            .await?;
            audio_file_ids.push(audio_file_id);
        }
        crate::audio_attach::attach_audio_files(
            &state,
            record.file_sqlite_id,
            audio_file_ids,
            None,
        )
        .await?;
    }

    let derivatives = upload_plan.derivatives.clone();
    if has_any_derivative(&derivatives) {
        if let Some(fp) = source_file_path {
            persistence::mark_status(&state, record.file_sqlite_id, "processing").await?;
            session.emit_processing();
            spawn_derivative_processing(
                state.clone(),
                session.clone(),
                prepared.clone(),
                fp,
                record.file_sqlite_id,
                record.thread_id_str.parse::<u64>().unwrap_or_default(),
                session_id,
                derivatives,
            );
        } else {
            persistence::mark_status(&state, record.file_sqlite_id, "ready").await?;
            session.emit_done();
        }
    } else {
        persistence::mark_status(&state, record.file_sqlite_id, "ready").await?;
        session.emit_done();
    }

    info!(
        "Upload completed: '{}' (ID: {})",
        prepared.filename, record.file_sqlite_id
    );

    if let Some(thread_id) = record.thread_to_archive {
        if let Some(gateway) = state.provider_runtime.remote_object_registry.get("discord") {
            let _ = gateway
                .archive_object(&RemoteObjectRef::DiscordThread { thread_id })
                .await;
        }
    }

    if let Some(webhook_url) = upload_plan
        .advanced
        .as_ref()
        .and_then(|a| a.webhook_url.as_ref())
    {
        if !webhook_url.is_empty() {
            let content = format!(
                "&#x1f7e2; **Upload Completed**: `{}`\nID: `{}`",
                prepared.filename, record.file_sqlite_id
            );
            let _ = send_webhook_notification(webhook_url, &content).await;
        }
    }

    Ok(record.file_sqlite_id)
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
async fn run_original_upload(
    state: &UploadContext,
    prepared: &plan::PreparedUploadPlan,
    record: &UploadRecordContext,
    session: &UploadSessionTracker,
    source: UploadDataSource,
    tg_authorized: bool,
    telegram_progress_tx: UnboundedSender<usize>,
    sem: Arc<Semaphore>,
    advanced: Option<AdvancedLimits>,
    cancel_token: CancellationToken,
    _video_duration_sec: Option<f64>,
) -> UploadResult<String> {
    let has_discord = prepared
        .providers
        .iter()
        .any(|p| matches!(p, ProviderType::Discord));
    let has_telegram = prepared
        .providers
        .iter()
        .any(|p| matches!(p, ProviderType::Telegram));

    let source_path_for_cache = match &source {
        UploadDataSource::File(path) => Some(path.clone()),
        UploadDataSource::Stream { .. } => None,
    };

    let is_stream = matches!(&source, UploadDataSource::Stream { .. });

    let mut file: Box<dyn tokio::io::AsyncRead + Unpin + Send> = match source {
        UploadDataSource::File(file_path) => {
            Box::new(BufReader::new(fs::File::open(&file_path).await.map_err(|err| {
                UploadError::io(format!("Unable to open file: {}", file_path.display()), err)
            })?))
        }
        UploadDataSource::Stream { reader, .. } => reader,
    };

    let (discord_tx, discord_rx) = if has_discord {
        let parallel = prepared
            .provider_settings
            .get("discord")
            .map(|s| s.parallel_sends)
            .unwrap_or(1);
        let (tx, rx) = tokio::sync::mpsc::channel(parallel * 4);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let (telegram_tx, telegram_rx) = if has_telegram {
        let parallel = prepared
            .provider_settings
            .get("telegram")
            .map(|s| s.parallel_sends)
            .unwrap_or(1);
        let (tx, rx) = tokio::sync::mpsc::channel(parallel * 4);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let discord_worker = if let Some(rx) = discord_rx {
        let state = state.clone();
        let record = record.clone();
        let prepared = prepared.clone();
        let session = session.clone();
        let advanced = advanced.clone();
        let sem = Arc::clone(&sem);
        let token = cancel_token.clone();

        Some(tokio::spawn(async move {
            run_discord_worker(state, rx, record, prepared, session, advanced, sem, token).await
        }))
    } else {
        None
    };

    let telegram_worker = if let Some(rx) = telegram_rx {
        let state = state.clone();
        let record = record.clone();
        let prepared = prepared.clone();
        let session = session.clone();
        let advanced = advanced.clone();
        let sem = Arc::clone(&sem);
        let tg_progress_tx = telegram_progress_tx.clone();
        let token = cancel_token.clone();

        Some(tokio::spawn(async move {
            run_telegram_worker(
                state,
                rx,
                record,
                prepared,
                session,
                advanced,
                sem,
                tg_authorized,
                tg_progress_tx,
                token,
            )
            .await
        }))
    } else {
        None
    };

    let mut bytes_left = prepared.total_bytes;
    let mut file_hasher = blake3::Hasher::new();
    let mut dispatch_error = None;

    for idx in 0..prepared.total_base_parts {
        if cancel_token.is_cancelled() {
            dispatch_error = Some(UploadError::internal(
                "Upload cancelled by user",
                "cancellation token signalled",
            ));
            break;
        }

        let part_num = (idx + 1) as u32;
        let current_chunk_size = if bytes_left >= prepared.base_chunk_size {
            prepared.base_chunk_size
        } else {
            bytes_left
        };

        let mut buffer = vec![0u8; current_chunk_size as usize];
        if is_stream {
            // Pipe mode: read up to chunk_size bytes, handle partial last chunk
            let mut read = 0u64;
            while read < current_chunk_size {
                let buf_slice = &mut buffer[read as usize..current_chunk_size as usize];
                match file.read(buf_slice).await {
                    Ok(0) => break, // EOF
                    Ok(n) => read += n as u64,
                    Err(err) => {
                        dispatch_error = Some(UploadError::io("Failed to read from upload pipe", err));
                        break;
                    }
                }
            }
            buffer.truncate(read as usize);
            if buffer.is_empty() && idx > 0 {
                break; // clean EOF after at least one chunk
            }
            bytes_left = bytes_left.saturating_sub(buffer.len() as u64);
        } else {
            if let Err(err) = file.read_exact(&mut buffer).await {
                dispatch_error = Some(UploadError::io("Failed to read upload chunk", err));
                break;
            }
            bytes_left -= current_chunk_size;
        }

        file_hasher.update(&buffer);

        let checksum = state.engine.integrity.calculate_bytes_blake3(&buffer);
        let logical_size = buffer.len() as u64;
        let chunk = Arc::new(UploadChunk {
            index: part_num,
            data: buffer,
            logical_size,
            part_type: PART_TYPE_ORIGINAL_CHUNK,
            checksum: Some(checksum),
        });

        if should_send_to_provider(
            prepared.strategy,
            &prepared.providers,
            ProviderType::Discord,
            part_num,
        ) {
            if let Some(tx) = &discord_tx {
                if tx.send(chunk.clone()).await.is_err() {
                    dispatch_error = Some(UploadError::internal(
                        "Discord upload worker channel closed unexpectedly",
                        "channel send failed",
                    ));
                    break;
                }
            }
        }
        if should_send_to_provider(
            prepared.strategy,
            &prepared.providers,
            ProviderType::Telegram,
            part_num,
        ) {
            if let Some(tx) = &telegram_tx {
                if tx.send(chunk).await.is_err() {
                    dispatch_error = Some(UploadError::internal(
                        "Telegram upload worker channel closed unexpectedly",
                        "channel send failed",
                    ));
                    break;
                }
            }
        }
    }
    // Drain remaining data from pipe if estimate was too low
    if is_stream && dispatch_error.is_none() {
        let mut part_num = prepared.total_base_parts as u32 + 1;
        loop {
            let mut buffer = vec![0u8; prepared.base_chunk_size as usize];
            let mut read = 0u64;
            while read < prepared.base_chunk_size {
                let buf_slice = &mut buffer[read as usize..];
                match file.read(buf_slice).await {
                    Ok(0) => break,
                    Ok(n) => read += n as u64,
                    Err(err) => {
                        dispatch_error = Some(UploadError::io("Failed to read from upload pipe", err));
                        break;
                    }
                }
            }
            if read == 0 { break; }
            buffer.truncate(read as usize);
            file_hasher.update(&buffer);

            let checksum = state.engine.integrity.calculate_bytes_blake3(&buffer);
            let chunk = Arc::new(UploadChunk {
                index: part_num,
                data: buffer,
                logical_size: read,
                part_type: PART_TYPE_ORIGINAL_CHUNK,
                checksum: Some(checksum),
            });
            if should_send_to_provider(prepared.strategy, &prepared.providers, ProviderType::Discord, part_num) {
                if let Some(tx) = &discord_tx {
                    if tx.send(chunk.clone()).await.is_err() { break; }
                }
            }
            if should_send_to_provider(prepared.strategy, &prepared.providers, ProviderType::Telegram, part_num) {
                if let Some(tx) = &telegram_tx {
                    if tx.send(chunk).await.is_err() { break; }
                }
            }
            part_num += 1;
        }
    }
    drop(discord_tx);
    drop(telegram_tx);

    let mut errors = Vec::new();
    if let Some(err) = dispatch_error {
        errors.push(err);
    }
    if let Some(handle) = discord_worker {
        match handle.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(UploadError::internal("Discord worker panicked", e)),
        }
    }
    if let Some(handle) = telegram_worker {
        match handle.await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => errors.push(e),
            Err(e) => errors.push(UploadError::internal("Telegram worker panicked", e)),
        }
    }

    // Non-blocking idx extraction — skip for remuxed files or streams (no original layout)
    if !is_stream {
        if let Some(fp) = source_path_for_cache {
            let idx_cache_dir = state.base_dir.join("idx_cache");
            let fid = record.file_sqlite_id;
            let fsz = prepared.total_bytes;
            tokio::spawn(async move {
                let _ = crate::index_extractor::extract_and_save_idx(
                    &fp, fsz, fid, &idx_cache_dir,
                ).await
                .map_err(|e| tracing::warn!("idx_cache: extract failed for file {}: {}", fid, e));
            });
        }
    }

    if !errors.is_empty() {
        let err = errors.remove(0);
        tracing::error!("Upload failed for '{}'.", prepared.filename);
        return Err(err);
    }

    Ok(file_hasher.finalize().to_hex().to_string())
}

async fn emit_telegram_backed_manifest_note(
    state: &UploadContext,
    prepared: &plan::PreparedUploadPlan,
    record: &UploadRecordContext,
) {
    let Some(thread_id) = record
        .thread_to_archive
        .or_else(|| record.thread_id_str.parse::<u64>().ok())
    else {
        return;
    };

    let parts = match state.file_repo.get_original_parts_for_file(record.file_sqlite_id).await {
        Ok(parts) => parts,
        Err(err) => {
            warn!(
                "Failed to load parts for Telegram manifest note (file_id={}): {}",
                record.file_sqlite_id, err
            );
            return;
        }
    };

    let has_discord_parts = parts.iter().any(|part| part.platform == "discord");
    let mut telegram_message_ids = parts
        .iter()
        .filter(|part| part.platform == "telegram")
        .map(|part| part.message_id.clone())
        .collect::<Vec<_>>();
    telegram_message_ids.sort();
    telegram_message_ids.dedup();

    if has_discord_parts || telegram_message_ids.is_empty() {
        return;
    }

    let preview_ids = telegram_message_ids
        .iter()
        .take(6)
        .cloned()
        .collect::<Vec<_>>();
    let remaining = telegram_message_ids.len().saturating_sub(preview_ids.len());
    let suffix = if remaining > 0 {
        format!(" (+{} more)", remaining)
    } else {
        String::new()
    };

    let note = format!(
        "SHARED DRIVE MANIFEST\nfile: `{}`\nkind: `{}`\nstorage_backend: `telegram`\ntelegram_parts: `{}`\ntelegram_messages: `{}`{}",
        prepared.filename,
        state.file_classifier.storage_kind_from_filename(&prepared.filename),
        parts.iter().filter(|part| part.platform == "telegram").count(),
        preview_ids.join(", "),
        suffix,
    );

    let Some(gateway) = state.provider_runtime.remote_object_registry.get("discord") else {
        return;
    };

    if let Err(err) = gateway
        .post_note(&RemoteObjectRef::DiscordThread { thread_id }, &note)
        .await
    {
        warn!(
            "Failed to post Telegram-backed manifest note to Discord thread {}: {}",
            thread_id, err
        );
    }
}

struct UploadChunk {
    index: u32,
    data: Vec<u8>,
    logical_size: u64,
    part_type: &'static str,
    checksum: Option<String>,
}

fn should_send_to_provider(
    strategy: UploadStrategy,
    providers: &[ProviderType],
    target: ProviderType,
    part_num: u32,
) -> bool {
    match strategy {
        UploadStrategy::Fast => {
            providers
                .get((part_num.saturating_sub(1) as usize) % providers.len().max(1))
                .copied()
                == Some(target)
        }
        UploadStrategy::Safe | UploadStrategy::None => providers.contains(&target),
    }
}

async fn run_discord_worker(
    state: UploadContext,
    mut rx: tokio::sync::mpsc::Receiver<Arc<UploadChunk>>,
    record: UploadRecordContext,
    prepared: plan::PreparedUploadPlan,
    session: UploadSessionTracker,
    advanced: Option<AdvancedLimits>,
    sem: Arc<Semaphore>,
    cancel_token: CancellationToken,
) -> UploadResult<()> {
    let settings = prepared
        .provider_settings
        .get("discord")
        .ok_or_else(|| UploadError::internal("Missing discord provider settings", ""))?;

    let batch_multiplier = settings.batch_multiplier;
    let mut current_batch = Vec::new();
    let mut handles = Vec::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                warn!("Discord worker: cancellation received, stopping chunk reception.");
                break;
            }
            chunk = rx.recv() => {
                match chunk {
                    None => break,
                    Some(chunk) => {
                        let is_done = record
                            .existing_parts
                            .iter()
                            .any(|p| p.part_index == chunk.index && p.platform == "discord");
                        if is_done {
                            continue;
                        }

                        current_batch.push(chunk);
                        if current_batch.len() >= batch_multiplier {
                            let batch = std::mem::take(&mut current_batch);
                            handles.push(spawn_discord_batch_task(
                                &state, &record, &prepared, &session, &advanced, &sem, batch,
                                cancel_token.clone(),
                            ));
                        }
                    }
                }
            }
        }
    }

    if cancel_token.is_cancelled() {
        for h in handles {
            h.abort();
        }
        return Ok(());
    }

    if !current_batch.is_empty() {
        handles.push(spawn_discord_batch_task(
            &state,
            &record,
            &prepared,
            &session,
            &advanced,
            &sem,
            current_batch,
            cancel_token.clone(),
        ));
    }

    for h in handles {
        h.await
            .map_err(|e| UploadError::internal("Discord task join failure", e))??;
    }
    Ok(())
}

fn spawn_discord_batch_task(
    state: &UploadContext,
    record: &UploadRecordContext,
    prepared: &plan::PreparedUploadPlan,
    session: &UploadSessionTracker,
    advanced: &Option<AdvancedLimits>,
    sem: &Arc<Semaphore>,
    batch: Vec<Arc<UploadChunk>>,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<UploadResult<()>> {
    let state = state.clone();
    let record = record.clone();
    let prepared = prepared.clone();
    let session = session.clone();
    let advanced = advanced.clone();
    let sem = Arc::clone(sem);

    tokio::spawn(async move {
        let permit = sem
            .acquire_owned()
            .await
            .map_err(|err| UploadError::internal("Semaphore acquire failed", err))?;

        if cancel_token.is_cancelled() {
            drop(permit);
            return Ok(());
        }

        if let Some(limit_kbps) = advanced.as_ref().and_then(|a| a.bandwidth_limit_kbps) {
            if limit_kbps > 0 {
                let bytes_in_batch: f64 = batch.iter().map(|c| c.data.len() as f64).sum();
                tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => {
                        drop(permit);
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_secs_f64(
                        bytes_in_batch / (limit_kbps as f64 * 1024.0),
                    )) => {}
                }
            }
        }

        let mut dispatch_data = Vec::new();
        for chunk in &batch {
            dispatch_data.push((chunk.data.clone(), chunk.index, chunk.checksum.clone()));
        }

        let dispatch_fut = provider_dispatch::dispatch_discord_batch(
            &state,
            &record.upload_target,
            dispatch_data,
            record.file_sqlite_id,
            &prepared.filename,
            prepared.total_parts,
        );

        let mut results = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                drop(permit);
                return Ok(());
            }
            res = dispatch_fut => res?,
        };

        for (result, chunk) in results.iter_mut().zip(batch.iter()) {
            result.logical_size = Some(chunk.logical_size);
        }

        if cancel_token.is_cancelled() {
            drop(permit);
            return Ok(());
        }

        if let Err(err) = persistence::persist_part_results(
            &state,
            record.file_sqlite_id,
            &results,
            batch
                .first()
                .map(|chunk| chunk.part_type)
                .unwrap_or(PART_TYPE_ORIGINAL_CHUNK),
        )
        .await
        {
            if let Err(cleanup_err) = persistence::cleanup_uploaded_parts_without_db(
                &state,
                record.file_sqlite_id,
                &results,
            )
            .await
            {
                warn!(
                    "Inline cleanup after Discord part persistence failure for file {} also failed: {}",
                    record.file_sqlite_id, cleanup_err
                );
            }
            return Err(err);
        }

        for res in results {
            session.complete_part(&res.platform, res.logical_size.unwrap_or(res.size));
        }

        drop(permit);
        Ok(())
    })
}

async fn run_telegram_worker(
    state: UploadContext,
    mut rx: tokio::sync::mpsc::Receiver<Arc<UploadChunk>>,
    record: UploadRecordContext,
    prepared: plan::PreparedUploadPlan,
    session: UploadSessionTracker,
    advanced: Option<AdvancedLimits>,
    sem: Arc<Semaphore>,
    tg_authorized: bool,
    tg_progress_tx: UnboundedSender<usize>,
    cancel_token: CancellationToken,
) -> UploadResult<()> {
    if !tg_authorized {
        return Ok(());
    }
    let mut handles = Vec::new();

    loop {
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                warn!("Telegram worker: cancellation received, stopping chunk reception.");
                break;
            }
            chunk = rx.recv() => {
                match chunk {
                    None => break,
                    Some(chunk) => {
                        let is_done = record
                            .existing_parts
                            .iter()
                            .any(|p| p.part_index == chunk.index && p.platform == "telegram");
                        if is_done {
                            continue;
                        }

                        handles.push(spawn_telegram_chunk_task(
                            &state,
                            &record,
                            &prepared,
                            &session,
                            &advanced,
                            &sem,
                            chunk,
                            tg_progress_tx.clone(),
                            cancel_token.clone(),
                        ));
                    }
                }
            }
        }
    }

    if cancel_token.is_cancelled() {
        for h in handles {
            h.abort();
        }
        return Ok(());
    }

    for h in handles {
        h.await
            .map_err(|e| UploadError::internal("Telegram task join failure", e))??;
    }

    Ok(())
}

fn spawn_telegram_chunk_task(
    state: &UploadContext,
    record: &UploadRecordContext,
    prepared: &plan::PreparedUploadPlan,
    session: &UploadSessionTracker,
    advanced: &Option<AdvancedLimits>,
    sem: &Arc<Semaphore>,
    chunk: Arc<UploadChunk>,
    tg_progress_tx: UnboundedSender<usize>,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<UploadResult<()>> {
    let state = state.clone();
    let record = record.clone();
    let prepared = prepared.clone();
    let session = session.clone();
    let advanced = advanced.clone();
    let sem = Arc::clone(sem);

    tokio::spawn(async move {
        let permit = sem
            .acquire_owned()
            .await
            .map_err(|err| UploadError::internal("Semaphore acquire failed", err))?;

        if cancel_token.is_cancelled() {
            drop(permit);
            return Ok(());
        }

        if let Some(limit_kbps) = advanced.as_ref().and_then(|a| a.bandwidth_limit_kbps) {
            if limit_kbps > 0 {
                let chunk_bytes = chunk.data.len() as f64;
                tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => {
                        drop(permit);
                        return Ok(());
                    }
                    _ = tokio::time::sleep(Duration::from_secs_f64(
                        chunk_bytes / (limit_kbps as f64 * 1024.0),
                    )) => {}
                }
            }
        }

        let dispatch_fut = provider_dispatch::dispatch_original_part(
            &state,
            &record.upload_target,
            true,
            UploadStrategy::None,
            &[ProviderType::Telegram],
            record.file_sqlite_id,
            chunk.data.clone(),
            &prepared.filename,
            chunk.index,
            prepared.total_parts,
            chunk.checksum.clone(),
            Some(tg_progress_tx),
        );

        let mut results = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => {
                drop(permit);
                return Ok(());
            }
            res = dispatch_fut => res?,
        };

        for result in &mut results {
            result.logical_size = Some(chunk.logical_size);
        }

        if cancel_token.is_cancelled() {
            drop(permit);
            return Ok(());
        }

        if let Err(err) = persistence::persist_part_results(
            &state,
            record.file_sqlite_id,
            &results,
            chunk.part_type,
        )
        .await
        {
            if let Err(cleanup_err) = persistence::cleanup_uploaded_parts_without_db(
                &state,
                record.file_sqlite_id,
                &results,
            )
            .await
            {
                warn!(
                    "Inline cleanup after Telegram part persistence failure for file {} also failed: {}",
                    record.file_sqlite_id, cleanup_err
                );
            }
            return Err(err);
        }
        for _res in results {
            session.complete_part_without_bytes();
        }

        drop(permit);
        Ok(())
    })
}

fn format_upload_error_detail(err: &UploadError) -> String {
    match err {
        UploadError::Validation { message }
        | UploadError::Conflict { message }
        | UploadError::Timeout { message } => format!("Upload failed: {message}"),
        UploadError::Db { message, source }
        | UploadError::Io { message, source }
        | UploadError::Provider { message, source }
        | UploadError::Internal { message, source } => {
            if let Some(source) = source {
                format!("Upload failed: {message} ({source})")
            } else {
                format!("Upload failed: {message}")
            }
        }
    }
}

fn spawn_derivative_processing(
    state: UploadContext,
    session: UploadSessionTracker,
    prepared: plan::PreparedUploadPlan,
    file_path: PathBuf,
    file_sqlite_id: i64,
    _thread_id: u64,
    session_id: String,
    derivatives: DerivativesPlan,
) {
    let meta_timeout = Duration::from_secs(90);
    let zip_timeout = Duration::from_secs(15 * 60);
    let derivative_upload_timeout = Duration::from_secs(30 * 60);

    tokio::spawn(async move {
        if prepared.is_video || prepared.is_audio || prepared.is_image {
            let meta_res =
                tokio::time::timeout(meta_timeout, extract_full_metadata(&state, &file_path)).await;
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
                if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                    if metadata.len() > 500 * 1024 * 1024 {
                        duration = 7200.0;
                    }
                }
                if duration < 1.0 {
                    duration = 30.0;
                }
            }

            let _ = persistence::persist_metadata_and_duration(
                &state,
                file_sqlite_id,
                full_meta.as_ref(),
                (duration > 0.0).then_some(duration),
            )
            .await;
        }

        if derivatives
            .zip_package
            .as_ref()
            .map(|p| p.enabled)
            .unwrap_or(false)
        {
            process_zip_derivative(
                &state,
                &prepared,
                &file_path,
                file_sqlite_id,
                &session_id,
                zip_timeout,
                derivative_upload_timeout,
            )
            .await;
        }

        let mut sanity_failed = false;
        for p_enum in &prepared.providers {
            let p_name = format!("{:?}", p_enum).to_lowercase();
            let expected_parts = prepared
                .provider_settings
                .get(&p_name)
                .map(|s| s.total_parts)
                .unwrap_or(0);

            let parts = state.file_repo.get_parts_for_file(file_sqlite_id).await.unwrap_or_default();
            let count = parts.iter().filter(|p| p.platform == p_name).count() as i64;

            if count < expected_parts as i64 {
                tracing::error!(
                    "Sanity check FAILED for {}: {}/{} parts found in DB.",
                    p_name,
                    count,
                    expected_parts
                );
                sanity_failed = true;
            }
        }

        if sanity_failed {
            let _ = persistence::mark_status(&state, file_sqlite_id, "error").await;
        } else {
            tracing::info!(
                "Sanity check PASSED for all providers. Marking file {} as ready.",
                file_sqlite_id
            );
            let _ = persistence::mark_status(&state, file_sqlite_id, "ready").await;
        }

        session.emit_done();
    });
}

async fn process_zip_derivative(
    state: &UploadContext,
    prepared: &plan::PreparedUploadPlan,
    file_path: &PathBuf,
    file_sqlite_id: i64,
    session_id: &str,
    zip_timeout: Duration,
    derivative_upload_timeout: Duration,
) {
    let temp_dir = std::env::temp_dir().join(format!("omega_zip_{}", uuid::Uuid::new_v4()));
    if fs::create_dir_all(&temp_dir).await.is_err() {
        return;
    }

    let zip_path = temp_dir.join(format!("{}.zip", prepared.filename));
    let entry_name = prepared.filename.clone();
    let zip_level = state.cfg.read().expect("cfg RwLock").general.zip_level;
    let src_path = file_path.clone();
    let zip_path_clone = zip_path.clone();
    let engine = state.engine.clone();

    let create_res = tokio::time::timeout(
        zip_timeout,
        tokio::task::spawn_blocking(move || {
            engine.zip.zip_file_to_path(&src_path, &zip_path_clone, &entry_name, zip_level, None)
        }),
    )
    .await;

    let mut zip_ready = false;
    match create_res {
        Ok(join_res) => match join_res {
            Ok(Ok(())) => zip_ready = true,
            Ok(Err(err)) => tracing::error!("Zip creation failed for {}: {}", file_sqlite_id, err),
            Err(err) => tracing::error!("Zip task failed for {}: {}", file_sqlite_id, err),
        },
        Err(_) => tracing::warn!("Zip creation timed out for {}", file_sqlite_id),
    }

    if zip_ready {
        let upload_res = tokio::time::timeout(
            derivative_upload_timeout,
            upload_derivative_file(
                state,
                file_sqlite_id,
                &zip_path,
                &format!("{}.zip", prepared.filename),
                session_id,
                PART_TYPE_ZIP_CHUNK,
                prepared.strategy,
                &prepared.providers,
            ),
        )
        .await;

        match upload_res {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => tracing::error!("Zip upload failed for {}: {}", file_sqlite_id, err),
            Err(_) => tracing::warn!("Zip upload timed out for {}", file_sqlite_id),
        }
    }

    let _ = fs::remove_file(&zip_path).await;
    let _ = fs::remove_dir_all(&temp_dir).await;
}

fn has_any_derivative(derivatives: &DerivativesPlan) -> bool {
    derivatives
        .web_preview
        .as_ref()
        .map(|plan| plan.enabled)
        .unwrap_or(false)
        || derivatives
            .zip_package
            .as_ref()
            .map(|plan| plan.enabled)
            .unwrap_or(false)
        || derivatives
            .hashes
            .as_ref()
            .map(|plan| plan.enabled)
            .unwrap_or(false)
}

async fn upload_hidden_audio_file(
    state: &UploadContext,
    audio_path: &str,
    thread_id_str: &str,
    thread_id: u64,
    drive_scope: DriveScope,
    folder_id: Option<i64>,
    tg_authorized: bool,
    strategy: UploadStrategy,
    providers: &[ProviderType],
) -> UploadResult<i64> {
    let path = std::path::PathBuf::from(audio_path);
    let metadata = tokio::fs::metadata(&path).await
        .map_err(|e| UploadError::io("Failed to read audio file metadata", e))?;
    let total_bytes = metadata.len();
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.bin")
        .to_string();

    let audio_file_id = state.file_repo.insert_file(
        &filename,
        total_bytes as i64,
        thread_id_str,
        folder_id,
        drive_scope.as_str(),
        None,
        None,
    ).await
        .map_err(|e| UploadError::db("Failed to insert audio file record", e))?;

    state.file_repo.toggle_hidden(audio_file_id, true).await
        .map_err(|e| UploadError::db("Failed to hide audio file", e))?;

    let upload_target = RemoteUploadTarget::DiscordThread {
        thread_id,
        archive_on_finalize: false,
    };

    let safe_limit = state.cfg.read().expect("cfg RwLock")
        .providers.get("discord")
        .map(|p| p.limits.hard_limit_bytes)
        .unwrap_or(0) as u64;
    let chunk_size = std::cmp::min(
        state.cfg.read().expect("cfg RwLock").general.chunk_bytes,
        safe_limit,
    ).max(1);
    let mut total_parts = (total_bytes as f64 / chunk_size as f64).ceil() as usize;
    if total_parts == 0 && total_bytes > 0 {
        total_parts = 1;
    }

    let mut file = tokio::io::BufReader::new(
        tokio::fs::File::open(&path).await
            .map_err(|e| UploadError::io("Failed to open audio file", e))?
    );
    let mut bytes_left = total_bytes;

    for idx in 0..total_parts {
        let part_num = (idx + 1) as u32;
        let current_chunk_size = if bytes_left >= chunk_size { chunk_size } else { bytes_left };
        let mut buffer = vec![0u8; current_chunk_size as usize];
        file.read_exact(&mut buffer).await
            .map_err(|e| UploadError::io("Failed to read audio chunk", e))?;
        bytes_left -= current_chunk_size;

        let part_results = provider_dispatch::dispatch_original_part(
            state,
            &upload_target,
            tg_authorized,
            strategy,
            providers,
            audio_file_id,
            buffer,
            &filename,
            part_num,
            total_parts,
            None,
            None,
        ).await?;

        for part in &part_results {
            state.file_repo.insert_part(
                audio_file_id,
                &part.platform,
                &part.message_id.to_string(),
                part.attachment_name.as_deref(),
                part.part_index,
                part.size as i64,
                "chunk",
                None,
            ).await
                .map_err(|e| UploadError::db("Failed to persist audio part", e))?;
        }
    }

    state.file_repo.update_file_status(audio_file_id, "ready").await
        .map_err(|e| UploadError::db("Failed to mark audio file ready", e))?;

    Ok(audio_file_id)
}

async fn send_webhook_notification(url: &str, content: &str) {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "content": content,
        "username": "Omega Drive",
        "avatar_url": "https://cdn.discordapp.com/embed/avatars/0.png"
    });

    match client.post(url).json(&payload).send().await {
        Ok(_) => tracing::info!("Webhook notification sent successfully"),
        Err(err) => tracing::error!("Failed to send webhook notification: {}", err),
    }
}

pub async fn run_streaming_upload(
    state: UploadContext,
    source_info: UploadSourceInfo,
    stream: Box<dyn tokio::io::AsyncRead + Unpin + Send + 'static>,
    yt_dlp_child: Option<tokio::process::Child>,
    folder_id: Option<i64>,
    drive_scope: DriveScope,
    session_id: String,
    upload_plan: UploadPlan,
    cancel_token: CancellationToken,
    video_duration_sec: Option<f64>,
    video_container: Option<String>,
) -> UploadResult<i64> {
    let data_source = UploadDataSource::Stream {
        info: source_info,
        reader: stream,
    };

    let result = run_upload(state, data_source, folder_id, drive_scope, session_id, upload_plan, None, cancel_token, video_duration_sec, video_container).await;

    if let Some(mut child) = yt_dlp_child {
        let wait_result = tokio::time::timeout(std::time::Duration::from_secs(30), child.wait()).await;
        match wait_result {
            Ok(Ok(status)) if status.success() => {}
            Ok(Ok(status)) => {
                return Err(UploadError::internal(
                    &format!("yt-dlp download failed with status: {}", status),
                    "yt_dlp_child_exit",
                ));
            }
            Ok(Err(e)) => {
                return Err(UploadError::internal(
                    &format!("Failed to wait for yt-dlp: {}", e),
                    "yt_dlp_child_wait",
                ));
            }
            Err(_) => {
                let _ = child.kill().await;
                return Err(UploadError::internal(
                    "yt-dlp did not exit within 30s after stdout closed",
                    "yt_dlp_child_timeout",
                ));
            }
        }
    }

    result
}
