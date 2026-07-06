use std::path::Path;
use std::collections::HashMap;

use omega_drive_gateway::core::config::DEFAULT_DISCORD_PARTS_PER_MESSAGE;
pub use omega_drive_gateway::upload::upload_plan::{PreparedUploadPlan, ProviderExecutionSettings, UploadSourceInfo};
use omega_drive_gateway::upload::upload_plan::{PriorityMode, ProviderType, UploadPlan, UploadStrategy};
use tokio::fs;
use tracing::warn;

use crate::context::UploadContext;
use crate::error::{UploadError, UploadResult};

pub(crate) async fn read_source_info(path: &Path) -> UploadResult<UploadSourceInfo> {
    let metadata = fs::metadata(path)
        .await
        .map_err(|e| UploadError::io("Failed to read file metadata", e))?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    Ok(UploadSourceInfo { total_bytes: metadata.len(), filename, file_path_str: path.to_string_lossy().to_string() })
}

pub async fn build_execution_plan(
    state: &UploadContext,
    source: &UploadSourceInfo,
    upload_plan: &UploadPlan,
) -> UploadResult<PreparedUploadPlan> {
    state.ext_normalizer.validate_upload_plan(upload_plan).map_err(UploadError::create_validation_error)?;

    let is_video = state.file_classifier.is_video_file(&source.filename);
    let is_audio = state.file_classifier.is_audio_file(&source.filename);
    let is_image = state.file_classifier.is_image_file(&source.filename);
    let strategy = upload_plan.original_upload.strategy;
    let providers = upload_plan.original_upload.providers.clone();
    let priority_mode = upload_plan.original_upload.priority_mode;

    let mut provider_configs: HashMap<String, (usize, u64, Option<usize>)> = HashMap::new();
    let mut min_chunk_bytes;

    {
        let cfg = state.cfg.read().expect("cfg RwLock");
        min_chunk_bytes = cfg.general.chunk_bytes;
        for provider in &providers {
            let provider_id = format!("{:?}", provider).to_lowercase();
            let mut parallel = cfg.providers.get(&provider_id).map(|p| p.transfer.parallel_sends).unwrap_or(cfg.general.parallel_sends);
            if matches!(priority_mode, PriorityMode::Background) { parallel = 1; }
            let chunk_mb = cfg.providers.get(&provider_id).and_then(|p| p.transfer.chunk_mb).unwrap_or(0);
            let chunk_bytes = chunk_mb * 1024 * 1024;
            let batch_size = cfg.providers.get(&provider_id)
                .and_then(|p| p.transfer.batch_size);
            if chunk_bytes > 0 && chunk_bytes < min_chunk_bytes {
                min_chunk_bytes = chunk_bytes;
            }
            provider_configs.insert(provider_id, (parallel, chunk_bytes, batch_size));
        }
    }

    let discord_safe_limit = state.cfg.read().expect("cfg RwLock")
        .providers.get("discord")
        .map(|p| p.limits.hard_limit_bytes)
        .unwrap_or(0) as u64;
    for (provider_id, (_parallel, chunk_bytes, batch_size)) in &mut provider_configs {
        if provider_id == "discord" {
            let discord_limit = load_discord_max_bytes(state).await;
            let effective_limit = if discord_safe_limit > 0 { discord_limit.min(discord_safe_limit) } else { discord_limit };
            let limit_mb = effective_limit / 1024 / 1024;
            let chunk_mb = *chunk_bytes / 1024 / 1024;
            if chunk_mb > limit_mb && limit_mb > 0 {
                *chunk_bytes = limit_mb * 1024 * 1024;
                if *chunk_bytes < min_chunk_bytes { min_chunk_bytes = *chunk_bytes; }
            }
            *batch_size = upload_plan
                .advanced
                .as_ref()
                .and_then(|a| a.discord_batch_size)
                .filter(|&v| v > 0)
                .map(|v| v as usize)
                .or(*batch_size);
        }
    }

    if let Some(advanced) = &upload_plan.advanced {
        if let Some(chunk_mb) = advanced.chunk_size_mb {
            if chunk_mb > 0 {
                let chunk_bytes = chunk_mb as u64 * 1024 * 1024;
                if chunk_bytes < min_chunk_bytes { min_chunk_bytes = chunk_bytes; }
            }
        }
    }

    min_chunk_bytes = min_chunk_bytes.max(1024 * 1024);

    let needs_remux = false;

    let total_base_parts = div_ceil_u64(source.total_bytes, min_chunk_bytes) as usize;
    let provider_part_counts = build_provider_part_counts(total_base_parts, strategy, &providers);
    let per_provider_bytes = build_provider_byte_totals(source.total_bytes, min_chunk_bytes, strategy, &providers);

    let mut provider_settings = HashMap::new();
    for (provider_id, (parallel, _requested_bytes, configured_batch_size)) in provider_configs {
        let batch_multiplier = if provider_id == "discord" {
            configured_batch_size.unwrap_or(DEFAULT_DISCORD_PARTS_PER_MESSAGE).max(1)
        } else {
            1
        };
        let actual_provider_chunk = if provider_id == "discord" {
            min_chunk_bytes * batch_multiplier as u64
        } else {
            min_chunk_bytes
        };
        let total_parts = provider_part_counts.get(&provider_id).copied().unwrap_or(0);
        provider_settings.insert(provider_id, ProviderExecutionSettings {
            parallel_sends: parallel,
            chunk_size: actual_provider_chunk,
            batch_multiplier,
            total_parts,
        });
    }
    let parallel_sends = provider_settings
        .values()
        .map(|s| s.parallel_sends)
        .max()
        .unwrap_or(state.cfg.read().expect("cfg RwLock").general.parallel_sends.max(1));

    Ok(PreparedUploadPlan {
        total_bytes: source.total_bytes,
        filename: source.filename.clone(),
        file_path_str: source.file_path_str.clone(),
        is_video, is_audio, is_image,
        needs_remux,
        base_chunk_size: min_chunk_bytes,
        total_base_parts,
        total_parts: total_base_parts,
        parallel_sends,
        per_provider_bytes,
        strategy, providers,
        provider_settings,
    })
}

fn div_ceil_u64(value: u64, divisor: u64) -> u64 {
    if divisor == 0 { return 0; }
    value.div_ceil(divisor)
}

fn provider_key(provider: ProviderType) -> String {
    format!("{:?}", provider).to_lowercase()
}

fn build_provider_part_counts(total_base_parts: usize, strategy: UploadStrategy, providers: &[ProviderType]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    if total_base_parts == 0 || providers.is_empty() { return counts; }
    match strategy {
        UploadStrategy::Safe | UploadStrategy::None => {
            for provider in providers { counts.insert(provider_key(*provider), total_base_parts); }
        }
        UploadStrategy::Fast => {
            for idx in 0..total_base_parts {
                let provider = providers[idx % providers.len()];
                *counts.entry(provider_key(provider)).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn build_provider_byte_totals(total_bytes: u64, base_chunk_size: u64, strategy: UploadStrategy, providers: &[ProviderType]) -> HashMap<String, u64> {
    let mut totals = HashMap::new();
    if total_bytes == 0 || base_chunk_size == 0 || providers.is_empty() { return totals; }
    let total_base_parts = div_ceil_u64(total_bytes, base_chunk_size) as usize;
    let mut bytes_left = total_bytes;
    for idx in 0..total_base_parts {
        let chunk_bytes = bytes_left.min(base_chunk_size);
        bytes_left = bytes_left.saturating_sub(chunk_bytes);
        match strategy {
            UploadStrategy::Safe | UploadStrategy::None => {
                for provider in providers { *totals.entry(provider_key(*provider)).or_insert(0) += chunk_bytes; }
            }
            UploadStrategy::Fast => {
                let provider = providers[idx % providers.len()];
                *totals.entry(provider_key(provider)).or_insert(0) += chunk_bytes;
            }
        }
    }
    totals
}

async fn load_discord_max_bytes(state: &UploadContext) -> u64 {
    match state.provider_runtime.provider_admin_registry.get("discord") {
        Some(gateway) => match gateway.fetch_upload_limits().await {
            Ok(constraints) => constraints.max_part_bytes.unwrap_or(25 * 1024 * 1024),
            Err(err) => { warn!("Unable to read Discord upload constraints, using 25MB: {}", err); 25 * 1024 * 1024 }
        },
        None => 25 * 1024 * 1024,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_strategy_duplicates_part_counts_for_each_provider() {
        let providers = vec![ProviderType::Discord, ProviderType::Telegram];
        let counts = build_provider_part_counts(3, UploadStrategy::Safe, &providers);
        assert_eq!(counts.get("discord"), Some(&3));
        assert_eq!(counts.get("telegram"), Some(&3));
    }

    #[test]
    fn fast_strategy_stripes_part_counts_across_providers() {
        let providers = vec![ProviderType::Discord, ProviderType::Telegram];
        let counts = build_provider_part_counts(5, UploadStrategy::Fast, &providers);
        assert_eq!(counts.get("discord"), Some(&3));
        assert_eq!(counts.get("telegram"), Some(&2));
    }

    #[test]
    fn fast_strategy_distributes_provider_bytes_by_base_chunk() {
        let providers = vec![ProviderType::Discord, ProviderType::Telegram];
        let totals = build_provider_byte_totals(10, 4, UploadStrategy::Fast, &providers);
        assert_eq!(totals.get("discord"), Some(&6));
        assert_eq!(totals.get("telegram"), Some(&4));
    }

    #[test]
    fn safe_strategy_duplicates_provider_bytes_for_each_provider() {
        let providers = vec![ProviderType::Discord, ProviderType::Telegram];
        let totals = build_provider_byte_totals(10, 4, UploadStrategy::Safe, &providers);
        assert_eq!(totals.get("discord"), Some(&10));
        assert_eq!(totals.get("telegram"), Some(&10));
    }
}
