use std::{collections::HashSet, sync::Arc};

use futures_util::future::join_all;
use futures_util::stream::{self, StreamExt};
use omega_drive_gateway::core::engine_context::EngineContext;
use omega_drive_gateway::core::events::SharedEventBus;
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::provider::folder_repository::FolderRepository;
use omega_drive_gateway::core::provider_runtime::ProviderRuntime;
use omega_drive_gateway::core::scope::DriveScope;
use tokio::sync::Mutex;
use tracing::{error, info};

pub struct DriveService {
    pub file_repo: Arc<dyn FileRepository>,
    pub folder_repo: Arc<dyn FolderRepository>,
    pub provider_runtime: Arc<ProviderRuntime>,
    pub events: SharedEventBus,
    pub engine: EngineContext,
}

impl DriveService {
    pub fn new(
        file_repo: Arc<dyn FileRepository>,
        folder_repo: Arc<dyn FolderRepository>,
        provider_runtime: Arc<ProviderRuntime>,
        events: SharedEventBus,
        engine: EngineContext,
    ) -> Self {
        Self { file_repo, folder_repo, provider_runtime, events, engine }
    }

    pub async fn create_folder(
        &self,
        name: &str,
        parent_id: Option<i64>,
        drive_scope: DriveScope,
    ) -> Result<i64, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Folder name cannot be empty.".to_string());
        }

        if let Some(parent_id) = parent_id {
            let parent = self.folder_repo
                .get_folder_by_id(parent_id).await
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Parent folder not found".to_string())?;
            if parent.drive_scope != drive_scope.as_str() {
                return Err("Cannot create subfolder in a different drive scope.".to_string());
            }
        }

        let gateway = self
            .provider_runtime
            .remote_folder_registry
            .get(drive_scope.remote_folder_provider_id())
            .ok_or_else(|| "Discord remote folder gateway is unavailable".to_string())?;
        let _cat = gateway
            .create_folder(name, None)
            .await
            .map_err(|e| format!("Failed to create Discord category: {e}"))?;

        let folder_id = self.folder_repo
            .insert_folder(name, parent_id).await
            .map_err(|e| e.to_string())?;

        info!("Created folder '{}' (ID: {})", name, folder_id);
        Ok(folder_id)
    }

    pub async fn rename_folder(&self, folder_id: i64, new_name: &str) -> Result<(), String> {
        let new_name = new_name.trim();
        if new_name.is_empty() {
            return Err("Folder name cannot be empty.".to_string());
        }

        self.folder_repo
            .update_folder_name(folder_id, new_name).await
            .map_err(|e| format!("db: {e}"))?;
        info!("Renamed folder ID {} to '{}'", folder_id, new_name);
        Ok(())
    }

    pub async fn delete_folder(&self, folder_id: i64) -> Result<(), String> {
        self.folder_repo
            .delete_folder(folder_id).await
            .map_err(|e| format!("db: {e}"))?;
        Ok(())
    }

    pub async fn move_folder(&self, folder_id: i64, parent_id: Option<i64>) -> Result<(), String> {
        if matches!(parent_id, Some(pid) if pid == folder_id) {
            return Err("cannot move folder into itself".to_string());
        }

        let folder = self.folder_repo
            .get_folder_by_id(folder_id).await
            .map_err(|e| format!("db: {e}"))?
            .ok_or_else(|| "folder not found".to_string())?;
        let folder_scope = folder.drive_scope;

        if let Some(pid) = parent_id {
            let mut curr = pid;
            let mut visited = HashSet::new();
            visited.insert(folder_id);

            while let Some(f) = self.folder_repo
                .get_folder_by_id(curr).await
                .map_err(|e| format!("db: {e}"))?
            {
                if f.drive_scope != folder_scope {
                    return Err("cannot move folder across drive scopes".to_string());
                }
                if f.id == folder_id {
                    return Err("cannot move folder into descendant".to_string());
                }
                if !visited.insert(f.id) {
                    break;
                }
                if let Some(next_pid) = f.parent_id {
                    curr = next_pid;
                } else {
                    break;
                }
            }
        }

        self.folder_repo
            .update_folder_parent(folder_id, parent_id).await
            .map_err(|e| format!("db: {e}"))?;
        Ok(())
    }

    pub async fn purge_file(&self, file_id: i64) -> Result<(), String> {
        let parts = self.file_repo
            .get_parts_for_file(file_id).await
            .map_err(|e| e.to_string())?;

        let mut parts_by_platform = std::collections::HashMap::<String, Vec<_>>::new();
        for part in parts.iter().cloned() {
            parts_by_platform
                .entry(part.platform.clone())
                .or_default()
                .push(part);
        }

        let delete_tasks: Vec<_> = parts_by_platform
            .into_iter()
            .filter_map(|(platform, provider_parts)| {
                let name = platform.clone();
                let gateway = self
                    .provider_runtime
                    .remote_object_registry
                    .get(&platform)?
                    .clone();
                Some(async move {
                    gateway
                        .delete_file_artifacts(file_id, &provider_parts)
                        .await
                        .map_err(|e| (name, e))
                })
            })
            .collect();

        for result in join_all(delete_tasks).await {
            if let Err((platform, e)) = result {
                error!("Failed to delete artifacts for '{}' on file {}: {}", platform, file_id, e);
            }
        }

        self.file_repo.delete_file(file_id).await
            .map_err(|e| e.to_string())?;

        info!("Purged file ID: {}", file_id);
        Ok(())
    }

    pub async fn retrieve_full_file(&self, file_id: i64) -> Result<Vec<u8>, String> {
        let file_info = self.file_repo
            .get_file_by_id(file_id).await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "File not found in DB".to_string())?;

        if file_info.size > 100 * 1024 * 1024 {
            return Err("File too large to preview (Max 100MB)".to_string());
        }

        self.file_repo.update_file_status(file_id, "accessed").await.ok();

        let parts = self.file_repo
            .get_parts_for_file(file_id).await
            .map_err(|e| e.to_string())?;

        let parts: Vec<_> = parts.into_iter().filter(|p| p.part_type == "chunk").collect();

        let mut unique_parts_map = std::collections::BTreeMap::new();
        for p in parts {
            let entry = unique_parts_map.entry(p.part_index).or_insert_with(|| p.clone());
            if p.platform == "telegram" && entry.platform == "discord" {
                *entry = p;
            }
        }

        let mut parts_with_offsets = Vec::new();
        let mut curr_offset = 0;
        for p in unique_parts_map.into_values() {
            parts_with_offsets.push((p.clone(), curr_offset));
            curr_offset += p.size as usize;
        }

        let total_size = file_info.size as usize;
        let final_buffer = Arc::new(Mutex::new(vec![0u8; total_size]));

        {
            let mut stream = stream::iter(parts_with_offsets)
                .map(|(part, offset)| {
                    let provider_runtime = Arc::clone(&self.provider_runtime);
                    let final_buffer = Arc::clone(&final_buffer);
                    async move {
                        let gateway = provider_runtime
                            .stream_registry
                            .get(&part.platform)
                            .ok_or_else(|| format!("Gateway '{}' is not ready", part.platform))?;

                        let raw_bytes = gateway.download_part_bytes(&part).await
                            .map_err(|e| format!("Error loading part {}: {}", part.part_index, e))?;

                        let data = self.engine.zip.unzip_or_raw(raw_bytes)
                            .map_err(|e| format!("Error decompressing part {}: {}", part.part_index, e))?;

                        let mut buffer = final_buffer.lock().await;
                        let end = (offset + data.len()).min(buffer.len());
                        if end > offset {
                            buffer[offset..end].copy_from_slice(&data[..(end - offset)]);
                        }
                        Ok::<(), String>(())
                    }
                })
                .buffer_unordered(8);

            while let Some(res) = stream.next().await {
                res?;
            }
        }

        let mutex = Arc::try_unwrap(final_buffer)
            .map_err(|_| "Failed to unwrap buffer".to_string())?;
        Ok(mutex.into_inner())
    }
}
