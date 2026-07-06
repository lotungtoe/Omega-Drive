use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, BufReader};

use omega_drive_gateway::provider::provider_types::RemoteUploadTarget;
use omega_drive_gateway::upload::upload_plan::{ProviderType, UploadStrategy};

use crate::context::UploadContext;
use crate::progress::{app_event_emitter, emit_progress_to};
use crate::provider_dispatch;

pub(crate) async fn upload_derivative_file(
    state: &UploadContext,
    file_sqlite_id: i64,
    file_path: &PathBuf,
    display_name: &str,
    session_id: &str,
    part_type: &str,
    strategy: UploadStrategy,
    providers: &[ProviderType],
) -> Result<()> {
    let ui_emitter = app_event_emitter(state);
    let metadata = fs::metadata(file_path)
        .await
        .with_context(|| format!("Unable to read derivative file: {}", file_path.display()))?;
    let total_bytes = metadata.len();

    let safe_limit = state.cfg.read().expect("cfg RwLock")
        .providers.get("discord")
        .map(|p| p.limits.hard_limit_bytes)
        .unwrap_or(0) as u64;
    let chunk_size = std::cmp::min(state.cfg.read().expect("cfg RwLock").general.chunk_bytes, safe_limit).max(1);
    let mut total_parts = (total_bytes as f64 / chunk_size as f64).ceil() as usize;
    if total_parts == 0 && total_bytes > 0 {
        total_parts = 1;
    }

    let file_meta = state
        .file_repo
        .get_file_by_id(file_sqlite_id)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
        .ok_or_else(|| anyhow::anyhow!("File not found for derivative upload"))?;
    let thread_id_str = file_meta.thread_id;

    let thread_id = thread_id_str
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("Invalid thread id"))?;

    let upload_target = RemoteUploadTarget::DiscordThread {
        thread_id,
        archive_on_finalize: true,
    };

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

    let mut file = BufReader::new(fs::File::open(file_path).await?);
    let mut bytes_left = total_bytes;

    for idx in 0..total_parts {
        let part_num = (idx + 1) as u32;
        let current_chunk_size = if bytes_left >= chunk_size {
            chunk_size
        } else {
            bytes_left
        };
        let mut buffer = vec![0u8; current_chunk_size as usize];
        file.read_exact(&mut buffer).await?;
        bytes_left -= current_chunk_size;

        let part_results = provider_dispatch::dispatch_original_part(
            state,
            &upload_target,
            tg_authorized,
            strategy,
            providers,
            file_sqlite_id,
            buffer,
            display_name,
            part_num,
            total_parts,
            None,
            None,
        )
        .await
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

        for result in &part_results {
            state
                .file_repo
                .insert_part(
                    file_sqlite_id,
                    &result.platform,
                    &result.message_id.to_string(),
                    result.attachment_name.as_deref(),
                    result.part_index,
                    result.size as i64,
                    part_type,
                    None,
                )
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        }

        emit_progress_to(
            ui_emitter.as_ref(),
            "upload-progress",
            session_id,
            display_name,
            "processing",
            idx + 1,
            total_parts,
            "Uploading derivative...",
            0,
            0,
            0,
            0,
            Some(file_sqlite_id),
        );
    }

    Ok(())
}
