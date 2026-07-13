use anyhow::{Context, Result};
use futures_util::{stream::FuturesUnordered, StreamExt};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::fs;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;

use omega_drive_gateway::core::engine_context::{IntegrityService, ZipService};
use omega_drive_gateway::core::error::AppError;
use omega_drive_gateway::provider::provider_types::ByteRange;
use omega_drive_gateway::provider::storage::PartMetadata;
use omega_drive_gateway::core::data::DownloadJob;

use crate::context::DownloadContext;
use crate::throttle::DownloadThrottle;

const MIN_SOFT_BPS: f64 = 1_048_576.0;

#[derive(Debug)]
pub struct DownloadCompletion {
    pub file_id: i64,
    pub filename: String,
    pub target_path: String,
}

#[derive(Debug)]
pub enum DownloadJobError {
    Cancelled,
    DiskFull,
    Other(anyhow::Error),
}

impl From<anyhow::Error> for DownloadJobError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err)
    }
}

impl From<AppError> for DownloadJobError {
    fn from(err: AppError) -> Self {
        Self::Other(err.into())
    }
}

impl From<std::num::ParseIntError> for DownloadJobError {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::Other(err.into())
    }
}

#[derive(Debug, Clone)]
struct PartDownloadPlan {
    sequence: usize,
    completed_parts: usize,
    part: PartMetadata,
    part_start: u64,
    write_offset: u64,
    range: Option<ByteRange>,
    full_size: u64,
    verify_from_disk: bool,
}

#[derive(Debug)]
struct DownloadedPart {
    plan: PartDownloadPlan,
    data: Vec<u8>,
    elapsed: Duration,
}

fn logical_part_size(part: &PartMetadata) -> u64 {
    part.size.max(0) as u64
}

fn decode_original_download_bytes(zip: &dyn ZipService, raw_bytes: Vec<u8>) -> Result<Vec<u8>> {
    zip.unzip_or_raw(raw_bytes).map_err(|e| anyhow::anyhow!("{}", e))
}

pub fn build_temp_path(save_path: &Path) -> PathBuf {
    let ext = save_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let part_ext = if ext.is_empty() {
        "part".to_string()
    } else {
        format!("{ext}.part")
    };
    save_path.with_extension(part_ext)
}

pub async fn run_download_job(
    state: DownloadContext,
    job: DownloadJob,
    cancel: CancellationToken,
) -> Result<DownloadCompletion, DownloadJobError> {
    let (file_info, parts) = {
        let f = state
            .file_repo
            .get_file_by_id(job.file_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("File not found in DB"))?;
        let p = state.file_repo.get_original_parts_for_file(job.file_id).await?;
        (f, p)
    };

    let mut unique_parts_map = BTreeMap::new();
    for p in parts {
        use std::collections::btree_map::Entry;
        match unique_parts_map.entry(p.part_index) {
            Entry::Vacant(e) => {
                e.insert(p);
            }
            Entry::Occupied(mut e) => {
                if p.platform == "telegram" && e.get().platform == "discord" {
                    e.insert(p);
                }
            }
        }
    }
    let unique_parts: Vec<_> = unique_parts_map.into_values().collect();
    let total_parts_unique = unique_parts.len();
    for p in &unique_parts {
        tracing::debug!("[dl] unique part: idx={} platform={} msg={} size={}", p.part_index, p.platform, p.message_id, p.size);
    }
    let total_bytes_original: u64 = unique_parts.iter().map(|p| p.size.max(0) as u64).sum();
    let session_id = format!("dl-{}", job.file_id);

    let save_path = PathBuf::from(&job.target_path);
    let temp_path = build_temp_path(&save_path);

    if cancel.is_cancelled() {
        return Err(DownloadJobError::Cancelled);
    }

    check_disk_space(&save_path, total_bytes_original)?;

    let mut temp_file = if temp_path.exists() {
        tokio::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&temp_path)
            .await
            .with_context(|| format!("Cannot open temp file: {}", temp_path.display()))?
    } else {
        fs::File::create(&temp_path)
            .await
            .with_context(|| format!("Cannot create temp file: {}", temp_path.display()))?
    };

    let actual_len = temp_file.metadata().await.map(|m| m.len()).unwrap_or(0);
    let initial_len = actual_len.min(total_bytes_original);
    if actual_len > total_bytes_original {
        temp_file.set_len(total_bytes_original).await.ok();
    }

    let (resume_bytes, completed_parts, plans) = build_download_plans(&unique_parts, initial_len);
    if resume_bytes != initial_len {
        temp_file.set_len(resume_bytes).await.ok();
    }
    let mut bytes_downloaded = resume_bytes;
    temp_file
        .seek(std::io::SeekFrom::Start(bytes_downloaded))
        .await
        .ok();

    {
        let _ = state
            .download_job_repo
            .update_progress(job.id, completed_parts as i64)
            .await;
    }
    crate::progress::emit_progress(
        state.app_ctx.clone(),
        "download-progress",
        &session_id,
        &file_info.filename,
        "downloading",
        completed_parts,
        total_parts_unique,
        "Starting download...",
        bytes_downloaded,
        total_bytes_original,
        0,
        0,
        None,
    );

    let mut throttle = DownloadThrottle::new(0.0);
    let parallel_limit = compute_download_parallelism(&state, &unique_parts);
    let mut in_flight = FuturesUnordered::new();
    let mut pending = BTreeMap::new();
    let mut next_schedule = 0usize;
    let mut next_write = 0usize;

    while next_write < plans.len() {
        while next_schedule < plans.len() && in_flight.len() < parallel_limit {
            if cancel.is_cancelled() {
                return Err(DownloadJobError::Cancelled);
            }

            let plan = plans[next_schedule].clone();
            let part_idx = plan.completed_parts.saturating_sub(1);
            if state.cfg.read().expect("cfg RwLock").disk_check_interval_parts > 0
                && part_idx
                    % state
                        .cfg
                        .read()
                        .expect("cfg RwLock")
                        .disk_check_interval_parts
                        as usize
                    == 0
            {
                check_disk_space(
                    &save_path,
                    total_bytes_original.saturating_sub(bytes_downloaded),
                )?;
            }

            let expected_bytes = plan
                .range
                .as_ref()
                .map(|range| range.len)
                .unwrap_or(plan.full_size) as usize;
            let rate_bps = compute_effective_rate(&state, &throttle);
            throttle.set_rate(rate_bps);
            throttle.throttle(expected_bytes).await;

            let state_cloned = state.clone();
            let cancel_cloned = cancel.clone();
            in_flight.push(async move {
                let started = Instant::now();
                let data = download_part_with_retry(
                    &state_cloned,
                    &plan.part,
                    plan.range.clone(),
                    &cancel_cloned,
                )
                .await?;
                Ok::<DownloadedPart, DownloadJobError>(DownloadedPart {
                    plan,
                    data,
                    elapsed: started.elapsed(),
                })
            });
            next_schedule += 1;

            let part_delay_ms = state.cfg.read().expect("cfg RwLock").part_delay_ms;
            if part_delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(part_delay_ms)).await;
            }
        }

        let finished = tokio::select! {
            _ = cancel.cancelled() => return Err(DownloadJobError::Cancelled),
            maybe_result = in_flight.next() => maybe_result,
        }
        .ok_or_else(|| {
            DownloadJobError::Other(anyhow::anyhow!("Download queue stopped unexpectedly"))
        })??;

        throttle.observe(finished.data.len(), finished.elapsed);
        pending.insert(finished.plan.sequence, finished);

        while let Some(downloaded) = pending.remove(&next_write) {
            if downloaded.plan.write_offset != bytes_downloaded {
                temp_file
                    .seek(std::io::SeekFrom::Start(downloaded.plan.write_offset))
                    .await
                    .ok();
            }

            if let Err(e) = temp_file.write_all(&downloaded.data).await {
                if is_disk_full_error(&e) {
                    return Err(DownloadJobError::DiskFull);
                }
                return Err(DownloadJobError::Other(e.into()));
            }

            verify_downloaded_part(state.engine.integrity.as_ref(), &temp_path, &downloaded.plan, &downloaded.data)?;
            bytes_downloaded = downloaded
                .plan
                .part_start
                .saturating_add(downloaded.plan.full_size);

            {
                let _ = state
                    .download_job_repo
                    .update_progress(job.id, downloaded.plan.completed_parts as i64)
                    .await;
            }
            crate::progress::emit_progress(
                state.app_ctx.clone(),
                "download-progress",
                &session_id,
                &file_info.filename,
                "downloading",
                downloaded.plan.completed_parts,
                total_parts_unique,
                &format!(
                    "Downloading part {}/{}...",
                    downloaded.plan.completed_parts, total_parts_unique
                ),
                bytes_downloaded,
                total_bytes_original,
                0,
                0,
                None,
            );

            next_write += 1;
        }
    }

    if let Err(e) = temp_file.flush().await {
        if is_disk_full_error(&e) {
            return Err(DownloadJobError::DiskFull);
        }
        return Err(DownloadJobError::Other(e.into()));
    }
    drop(temp_file);

    check_magic_bytes(&temp_path)?;

    if let Some(expected_hash) = file_info.checksum {
        if !state.engine.integrity.verify_file_integrity(&temp_path, &expected_hash).await.map_err(|e| anyhow::anyhow!("{}", e))? {
            let _ = fs::remove_file(&temp_path).await;
            return Err(DownloadJobError::Other(anyhow::anyhow!(
                "Download data integrity check failed"
            )));
        }
    }

    fs::rename(&temp_path, &save_path).await.with_context(|| {
        format!(
            "Cannot rename temp file {} -> {}",
            temp_path.display(),
            save_path.display()
        )
    })?;

    crate::progress::emit_progress(
        state.app_ctx.clone(),
        "download-progress",
        &session_id,
        &file_info.filename,
        "done",
        total_parts_unique,
        total_parts_unique,
        "Download complete.",
        total_bytes_original,
        total_bytes_original,
        0,
        0,
        None,
    );

    Ok(DownloadCompletion {
        file_id: job.file_id,
        filename: file_info.filename,
        target_path: save_path.to_string_lossy().to_string(),
    })
}

fn build_download_plans(
    parts: &[PartMetadata],
    bytes_done: u64,
) -> (u64, usize, Vec<PartDownloadPlan>) {
    let effective_bytes_done = bytes_done;
    let mut part_start = 0u64;
    let mut completed_parts = 0usize;
    let mut plans = Vec::new();

    for (idx, part) in parts.iter().enumerate() {
        let full_size = logical_part_size(part);
        let next_part_start = part_start.saturating_add(full_size);
        let downloaded = effective_bytes_done
            .saturating_sub(part_start)
            .min(full_size);

        if downloaded == full_size {
            completed_parts = idx + 1;
        } else {
            let supports_partial_resume = downloaded > 0;
            plans.push(PartDownloadPlan {
                sequence: plans.len(),
                completed_parts: idx + 1,
                part: part.clone(),
                part_start,
                write_offset: if supports_partial_resume {
                    part_start.saturating_add(downloaded)
                } else {
                    part_start
                },
                range: supports_partial_resume.then_some(ByteRange {
                    start: downloaded,
                    len: full_size.saturating_sub(downloaded),
                }),
                full_size,
                verify_from_disk: supports_partial_resume,
            });
        }

        part_start = next_part_start;
    }

    (effective_bytes_done.min(part_start), completed_parts, plans)
}

fn compute_download_parallelism(state: &DownloadContext, parts: &[PartMetadata]) -> usize {
    let cfg = state.cfg.read().expect("cfg RwLock");
    let mut parallel = cfg.general.parallel_sends.max(1);
    for part in parts {
        if let Some(provider) = cfg.providers.get(&part.platform) {
            parallel = parallel.max(provider.transfer.parallel_sends.max(1));
        }
    }
    parallel.clamp(1, 8)
}

fn verify_part_checksum_from_disk(
    integrity: &dyn IntegrityService,
    path: &Path,
    part_start: u64,
    part_len: u64,
    expected_hash: &str,
) -> Result<(), DownloadJobError> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Cannot read temp file: {}", path.display()))
        .map_err(DownloadJobError::from)?;
    file.seek(SeekFrom::Start(part_start))
        .with_context(|| format!("Cannot seek to chunk offset {}", part_start))
        .map_err(DownloadJobError::from)?;

    let mut remaining = part_len;
    let mut hasher = integrity.create_hasher();
    let mut buffer = [0u8; 64 * 1024];

    while remaining > 0 {
        let to_read = buffer.len().min(remaining as usize);
        file.read_exact(&mut buffer[..to_read])
            .context("Cannot read chunk data for checksum verification")
            .map_err(DownloadJobError::from)?;
        hasher.update(&buffer[..to_read]);
        remaining -= to_read as u64;
    }

    let actual_hash = hasher.finalize_hex();
    if actual_hash != expected_hash {
        return Err(DownloadJobError::Other(anyhow::anyhow!(
            "Chunk integrity failure! Expected {}, got {}",
            expected_hash,
            actual_hash
        )));
    }

    Ok(())
}

fn verify_downloaded_part(
    integrity: &dyn IntegrityService,
    temp_path: &Path,
    plan: &PartDownloadPlan,
    data: &[u8],
) -> Result<(), DownloadJobError> {
    let Some(expected_hash) = plan.part.checksum.as_ref() else {
        return Ok(());
    };

    if plan.verify_from_disk {
        return verify_part_checksum_from_disk(
            integrity,
            temp_path,
            plan.part_start,
            plan.full_size,
            expected_hash,
        );
    }

    let actual_hash = integrity.calculate_bytes_blake3(data);
    if &actual_hash != expected_hash {
        return Err(DownloadJobError::Other(anyhow::anyhow!(
            "Chunk {} integrity failure! Expected {}, got {}",
            plan.completed_parts.saturating_sub(1),
            expected_hash,
            actual_hash
        )));
    }

    Ok(())
}

async fn download_part_with_retry(
    state: &DownloadContext,
    part: &PartMetadata,
    range: Option<ByteRange>,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, DownloadJobError> {
    let retries = state.cfg.read().expect("cfg RwLock").download_retry;
    let base_delay_s = state.cfg.read().expect("cfg RwLock").download_retry_base_s;

    let mut last_err: Option<anyhow::Error> = None;

    for attempt in 0..=retries {
        if cancel.is_cancelled() {
            return Err(DownloadJobError::Cancelled);
        }
        if attempt > 0 {
            let delay = base_delay_s * (1u64 << attempt.min(5));
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }

        let gateway = state
            .provider_runtime
            .stream_registry
            .get(&part.platform)
            .ok_or_else(|| anyhow::anyhow!("Stream gateway '{}' chua san sang", part.platform))?;
        tracing::debug!("[dl] downloading part: idx={} platform={} msg={} size={} attempt={}/{}",
            part.part_index, part.platform, part.message_id, part.size, attempt, retries);
        let result = tokio::select! {
            _ = cancel.cancelled() => return Err(DownloadJobError::Cancelled),
            res = gateway.download_part_range(part, range.clone()) => res.map_err(DownloadJobError::from),
        };

        match result {
            Ok(bytes) => {
                if let Some(_range) = range.as_ref() {
                    return Ok(bytes);
                }
                match decode_original_download_bytes(state.engine.zip.as_ref(), bytes) {
                    Ok(data) => return Ok(data),
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
            Err(e) => match e {
                DownloadJobError::Other(err) => {
                    last_err = Some(err);
                }
                DownloadJobError::Cancelled => return Err(DownloadJobError::Cancelled),
                DownloadJobError::DiskFull => return Err(DownloadJobError::DiskFull),
            },
        }
    }

    Err(DownloadJobError::Other(
        last_err.unwrap_or_else(|| anyhow::anyhow!("Download part failed")),
    ))
}

fn is_disk_full_error(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::StorageFull {
        return true;
    }
    matches!(err.raw_os_error(), Some(28 | 112 | 39))
}

fn is_system_drive(path: &Path) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::path::Component;
        let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        if let Some(Component::Prefix(prefix)) = path.components().next() {
            let drive = prefix.as_os_str().to_string_lossy().to_string();
            return drive.eq_ignore_ascii_case(&system);
        }
        return false;
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.components()
            .next()
            .map(|c| matches!(c, std::path::Component::RootDir))
            .unwrap_or(false)
    }
}

fn check_disk_space(path: &Path, remaining: u64) -> Result<(), DownloadJobError> {
    let available =
        fs2::available_space(path).map_err(|e| DownloadJobError::Other(e.into()))?;
    let total = fs2::total_space(path).map_err(|e| DownloadJobError::Other(e.into()))?;
    let required = if is_system_drive(path) {
        let reserve = (total as f64 * 0.01) as u64;
        remaining.saturating_add(reserve)
    } else {
        remaining
    };
    if available < required {
        return Err(DownloadJobError::DiskFull);
    }
    Ok(())
}

// ponytail: player-aware throttle removed from download crate
fn is_soft_limit_active(_state: &DownloadContext) -> bool {
    false
}

fn compute_effective_rate(state: &DownloadContext, throttle: &DownloadThrottle) -> f64 {
    let (hard_bps, soft_limit_ratio) = {
        let cfg = state.cfg.read().expect("cfg RwLock");
        (
            cfg.bandwidth_limit_kbps as f64 * 1024.0,
            cfg.soft_limit_ratio,
        )
    };
    let soft_active = is_soft_limit_active(state);
    if hard_bps > 0.0 {
        if soft_active {
            hard_bps * soft_limit_ratio
        } else {
            hard_bps
        }
    } else if soft_active {
        let ema = throttle.ema_bps();
        let base = if ema > 0.0 {
            ema * soft_limit_ratio
        } else {
            MIN_SOFT_BPS
        };
        base.max(MIN_SOFT_BPS)
    } else {
        0.0
    }
}

fn check_magic_bytes(path: &Path) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext.is_empty() {
        return Ok(());
    }

    let mut file = std::fs::File::open(path)?;
    let mut header = [0u8; 12];
    let count = file.read(&mut header)?;
    if count < 4 {
        return Err(anyhow::anyhow!("File header too short"));
    }

    let ok = match ext.as_str() {
        "mp4" | "m4v" => count >= 8 && &header[4..8] == b"ftyp",
        "mkv" | "webm" => header.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        "zip" => header.starts_with(b"PK\x03\x04"),
        "png" => header.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "jpg" | "jpeg" => header.starts_with(&[0xFF, 0xD8, 0xFF]),
        "gif" => header.starts_with(b"GIF8"),
        "pdf" => header.starts_with(b"%PDF"),
        _ => true,
    };

    if ok {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Magic bytes mismatch"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_part(part_index: u32, size: i64) -> PartMetadata {
        PartMetadata {
            id: i64::from(part_index),
            file_id: 1,
            platform: "telegram".to_string(),
            message_id: format!("msg-{part_index}"),
            part_index,
            size,
            checksum: None,
        }
    }

    #[test]
    fn build_download_plans_keeps_partial_resume_inside_current_part() {
        let parts = vec![test_part(1, 100), test_part(2, 100), test_part(3, 80)];
        let (resume_bytes, completed_parts, plans) = build_download_plans(&parts, 150);

        assert_eq!(resume_bytes, 150);
        assert_eq!(completed_parts, 1);
        assert_eq!(plans.len(), 2);

        let first = &plans[0];
        assert_eq!(first.sequence, 0);
        assert_eq!(first.completed_parts, 2);
        assert_eq!(first.part_start, 100);
        assert_eq!(first.write_offset, 150);
        assert!(first.verify_from_disk);
        assert_eq!(
            first.range.as_ref().map(|range| (range.start, range.len)),
            Some((50, 50))
        );

        let second = &plans[1];
        assert_eq!(second.sequence, 1);
        assert_eq!(second.completed_parts, 3);
        assert_eq!(second.part_start, 200);
        assert_eq!(second.write_offset, 200);
        assert!(!second.verify_from_disk);
        assert!(second.range.is_none());
    }

    #[test]
    fn build_download_plans_clamps_resume_beyond_total_size() {
        let parts = vec![test_part(1, 64), test_part(2, 32)];
        let (resume_bytes, completed_parts, plans) = build_download_plans(&parts, 999);

        assert_eq!(resume_bytes, 96);
        assert_eq!(completed_parts, 2);
        assert!(plans.is_empty());
    }
}
