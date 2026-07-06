use std::path::Path;
use tokio::sync::mpsc::UnboundedSender;

use omega_drive_gateway::provider::provider_types::{RemoteUploadTarget, UploadPartRequest};
use omega_drive_gateway::upload::upload_plan::{ProviderType, UploadStrategy};
pub use omega_drive_gateway::upload::upload_types::UploadedPart;

use crate::context::UploadContext;
use crate::error::{UploadError, UploadResult};

pub(crate) fn build_discord_attachment_name(file_name: &str, part_num: u32) -> String {
    let base_name = Path::new(file_name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("file.bin");
    format!("{base_name}.part{part_num}")
}

pub async fn dispatch_original_part(
    state: &UploadContext,
    upload_target: &RemoteUploadTarget,
    tg_authorized: bool,
    strategy: UploadStrategy,
    providers: &[ProviderType],
    _file_id: i64,
    data: Vec<u8>,
    file_name: &str,
    part_num: u32,
    _total_parts: usize,
    chunk_checksum: Option<String>,
    telegram_progress_tx: Option<UnboundedSender<usize>>,
) -> UploadResult<Vec<UploadedPart>> {
    let caption = String::new();

    if providers.is_empty() {
        return Err(UploadError::provider_message(
            "No providers selected for upload",
        ));
    }

    let targets = match strategy {
        UploadStrategy::Safe | UploadStrategy::None => providers.to_vec(),
        UploadStrategy::Fast => {
            let idx = (part_num as usize - 1) % providers.len();
            vec![providers[idx]]
        }
    };

    let mut results = Vec::with_capacity(targets.len());

    for target_provider in targets {
        let platform_id = format!("{:?}", target_provider).to_lowercase();

        if platform_id == "telegram" && !tg_authorized {
            return Err(UploadError::provider_message("Telegram is not authorized"));
        }

        let gateway = state
            .provider_runtime
            .part_store_registry
            .get(&platform_id)
            .ok_or_else(|| {
                UploadError::provider_message(format!(
                    "{} part store gateway not available",
                    platform_id
                ))
            })?;

        let upload_filename = if platform_id == "discord" {
            build_discord_attachment_name(file_name, part_num)
        } else {
            file_name.to_string()
        };

        let receipt = gateway
            .upload_part(UploadPartRequest {
                target: upload_target.clone(),
                data: data.clone(),
                file_name: upload_filename,
                caption: caption.clone(),
                part_num,
                telegram_progress_tx: if platform_id == "telegram" {
                    telegram_progress_tx.clone()
                } else {
                    None
                },
            })
            .await
            .map_err(|err| {
                UploadError::provider(format!("Failed to upload part to {}", platform_id), err)
            })?;

        results.push(UploadedPart {
            message_id: receipt.message_id,
            platform: receipt.platform,
            attachment_name: receipt.attachment_name,
            part_index: part_num,
            size: receipt.size,
            logical_size: None,
            checksum: chunk_checksum.clone(),
        });
    }

    Ok(results)
}

pub(crate) async fn dispatch_discord_batch(
    state: &UploadContext,
    upload_target: &RemoteUploadTarget,
    parts: Vec<(Vec<u8>, u32, Option<String>)>,
    _file_id: i64,
    file_name: &str,
    _total_parts: usize,
) -> UploadResult<Vec<UploadedPart>> {
    if parts.is_empty() {
        return Ok(vec![]);
    }

    let gateway = state
        .provider_runtime
        .part_store_registry
        .get("discord")
        .ok_or_else(|| UploadError::provider_message("discord part store gateway not available"))?;

    let caption = String::new();

    let requests: Vec<UploadPartRequest> = parts
        .iter()
        .map(|(data, part_num, _)| {
            let upload_filename = build_discord_attachment_name(file_name, *part_num);
            UploadPartRequest {
                target: upload_target.clone(),
                data: data.clone(),
                file_name: upload_filename,
                caption: caption.clone(),
                part_num: *part_num,
                telegram_progress_tx: None,
            }
        })
        .collect();

    let receipts = gateway
        .upload_parts_batch(requests)
        .await
        .map_err(|err| UploadError::provider("Failed to batch upload parts to discord", err))?;

    let results = parts
        .iter()
        .zip(receipts.iter())
        .map(|((_, part_num, checksum), receipt)| UploadedPart {
            message_id: receipt.message_id,
            platform: receipt.platform.clone(),
            attachment_name: receipt.attachment_name.clone(),
            part_index: *part_num,
            size: receipt.size,
            logical_size: None,
            checksum: checksum.clone(),
        })
        .collect();

    Ok(results)
}
