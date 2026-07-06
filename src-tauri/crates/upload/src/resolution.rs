use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use omega_drive_gateway::core::file_types::FileType;
use omega_drive_gateway::core::services::FileTypeClassifier;
use omega_drive_gateway::upload::upload_profile_selection::UploadProfileCandidate;

use crate::context::UploadContext;
use crate::upload_profile::{prepare_upload_rules, select_upload_profile};
use crate::error::UploadError;
use crate::UploadResult;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadBatchRequestItem {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadBatchResolvedItem {
    pub path: String,
    pub file_type: FileType,
    pub extension: Option<String>,
    pub profile_id: Option<i64>,
    pub rule_id: Option<i64>,
}

async fn sniff_file_type(classifier: &dyn FileTypeClassifier, path: &Path) -> Option<FileType> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await.ok()?;
    let mut buf = vec![0u8; 512];
    let n = file.read(&mut buf).await.ok()?;
    buf.truncate(n);
    classifier.sniff_magic_bytes(&buf)
}

pub async fn resolve_upload_profile_for_batch(
    ctx: &UploadContext,
    items: Vec<UploadBatchRequestItem>,
) -> UploadResult<Vec<UploadBatchResolvedItem>> {
    let mut file_infos = Vec::new();
    for item in items {
        let path = PathBuf::from(&item.path);
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .and_then(|s| ctx.file_classifier.normalize_extension(s));
        let size_bytes = match tokio::fs::metadata(&path).await {
            Ok(meta) => meta.len() as i64,
            Err(_) => 0,
        };
        file_infos.push((item.path, extension, size_bytes, path));
    }

    let sniff_enabled = file_infos.len() <= 200;
    let mut sniff_budget = if sniff_enabled { 50usize } else { 0usize };

    let mut groups: HashMap<Option<String>, Vec<usize>> = HashMap::new();
    for (idx, info) in file_infos.iter().enumerate() {
        groups.entry(info.1.clone()).or_default().push(idx);
    }

    let mut resolved_types = vec![FileType::Unknown; file_infos.len()];
    let mut ext_cache: HashMap<String, FileType> = HashMap::new();

    for (ext_opt, indices) in groups {
        match ext_opt {
            Some(ext) => {
                if let Some(ft) = ctx.file_classifier.file_type_from_extension(&ext) {
                    for idx in indices {
                        resolved_types[idx] = ft;
                    }
                    continue;
                }
                if let Some(ft) = ext_cache.get(&ext).copied() {
                    for idx in indices {
                        resolved_types[idx] = ft;
                    }
                    continue;
                }
                if sniff_budget == 0 {
                    continue;
                }
                let mut detected = None;
                for idx in &indices {
                    if sniff_budget == 0 {
                        break;
                    }
                    sniff_budget -= 1;
                    detected = sniff_file_type(&*ctx.file_classifier, &file_infos[*idx].3).await;
                    if detected.is_some() {
                        break;
                    }
                }
                let final_type = detected.unwrap_or(FileType::Unknown);
                ext_cache.insert(ext.clone(), final_type);
                for idx in indices {
                    resolved_types[idx] = final_type;
                }
            }
            None => {
                for idx in indices {
                    if sniff_budget == 0 {
                        resolved_types[idx] = FileType::Unknown;
                        continue;
                    }
                    sniff_budget -= 1;
                    resolved_types[idx] = sniff_file_type(&*ctx.file_classifier, &file_infos[idx].3)
                        .await
                        .unwrap_or(FileType::Unknown);
                }
            }
        }
    }

    let profiles = ctx
        .upload_profile_repo
        .get_upload_profiles()
        .await
        .map_err(|e| UploadError::db("Failed to load upload profiles.", e))?;
    let rules = ctx
        .upload_profile_repo
        .get_upload_profile_rules(None)
        .await
        .map_err(|e| UploadError::db("Failed to load upload rules.", e))?;
    let default_profile_id = profiles.first().and_then(|p| p.id);

    let mut valid_profile_ids = std::collections::HashSet::new();
    for p in &profiles {
        if let Some(id) = p.id {
            valid_profile_ids.insert(id);
        }
    }

    let normalized_rules = prepare_upload_rules(&*ctx.ext_normalizer, rules, &valid_profile_ids);

    let mut resolved = Vec::new();
    for (idx, info) in file_infos.iter().enumerate() {
        let file_type = resolved_types[idx];
        let candidate = UploadProfileCandidate {
            file_type,
            extension: info.1.clone(),
            size_bytes: info.2,
        };
        let selection = select_upload_profile(&candidate, &normalized_rules, default_profile_id);
        resolved.push(UploadBatchResolvedItem {
            path: info.0.clone(),
            file_type,
            extension: info.1.clone(),
            profile_id: selection.profile_id,
            rule_id: selection.rule_id,
        });
    }

    Ok(resolved)
}
