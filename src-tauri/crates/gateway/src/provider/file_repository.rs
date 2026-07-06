use async_trait::async_trait;

use crate::core::data::{AudioFileMetadata, FileMetadata, VideoFileMetadata, VideoPlaybackProgress};
use crate::core::error::AppResult;

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn get_file_by_id(&self, id: i64) -> AppResult<Option<FileMetadata>>;
    async fn get_file_by_thread_id(&self, thread_id: &str) -> AppResult<Option<FileMetadata>>;
    async fn get_file_by_name(&self, name: &str, folder_id: Option<i64>) -> AppResult<Option<FileMetadata>>;
    async fn get_all_files(&self) -> AppResult<Vec<FileMetadata>>;
    async fn get_files_by_parent(&self, folder_id: Option<i64>, drive_scope: Option<&str>) -> AppResult<Vec<FileMetadata>>;
    async fn get_recent_files_paginated(
        &self,
        cursor: Option<i64>,
        limit: i64,
        role_filter: Option<&str>,
        drive_scope: Option<&str>,
    ) -> AppResult<Vec<FileMetadata>>;
    async fn get_files_paginated(
        &self,
        folder_id: Option<i64>,
        cursor: Option<i64>,
        limit: i64,
        role_filter: Option<&str>,
        drive_scope: Option<&str>,
    ) -> AppResult<Vec<FileMetadata>>;
    async fn get_all_files_paginated(
        &self,
        cursor: Option<i64>,
        limit: i64,
        role_filter: Option<&str>,
        drive_scope: Option<&str>,
    ) -> AppResult<Vec<FileMetadata>>;
    async fn get_trash_paginated(
        &self,
        cursor: Option<i64>,
        limit: i64,
        role_filter: Option<&str>,
        drive_scope: Option<&str>,
    ) -> AppResult<Vec<FileMetadata>>;
    async fn get_transfers_paginated(
        &self,
        cursor: Option<i64>,
        limit: i64,
    ) -> AppResult<Vec<FileMetadata>>;
    async fn get_file_stats(&self, drive_scope: Option<&str>) -> AppResult<(i64, i64, i64)>;
    async fn insert_file(
        &self,
        filename: &str,
        size: i64,
        thread_id: &str,
        folder_id: Option<i64>,
        drive_scope: &str,
        checksum: Option<&str>,
        local_path: Option<&str>,
    ) -> AppResult<i64>;
    async fn insert_attachment_file(
        &self,
        filename: &str,
        size: i64,
        thread_id: &str,
        folder_id: Option<i64>,
        drive_scope: &str,
        checksum: Option<&str>,
    ) -> AppResult<i64>;
    async fn update_file_status(&self, id: i64, status: &str) -> AppResult<()>;
    async fn set_files_error_by_thread_id(&self, thread_id: &str) -> AppResult<()>;
    async fn rename_file_by_thread_id(&self, thread_id: &str, new_name: &str) -> AppResult<()>;
    async fn update_file_checksum(&self, id: i64, checksum: &str) -> AppResult<()>;
    async fn update_file_folder(&self, id: i64, folder_id: Option<i64>) -> AppResult<()>;
    async fn update_file_name(&self, id: i64, filename: &str) -> AppResult<()>;
    async fn update_file_local_path(&self, id: i64, local_path: Option<&str>) -> AppResult<()>;
    async fn move_to_trash(&self, id: i64) -> AppResult<()>;
    async fn restore_trash(&self, id: i64) -> AppResult<bool>;
    async fn toggle_star(&self, id: i64, starred: bool) -> AppResult<()>;
    async fn toggle_hidden(&self, id: i64, is_hidden: bool) -> AppResult<()>;
    async fn delete_file(&self, id: i64) -> AppResult<()>;
    async fn search_files(&self, query: &str, limit: i64) -> AppResult<Vec<FileMetadata>>;
    async fn get_parts_for_file(&self, file_id: i64) -> AppResult<Vec<crate::provider::storage::PartMetadata>>;
    async fn get_part_counts_for_files(&self, file_ids: &[i64]) -> AppResult<std::collections::HashMap<i64, usize>>;
    async fn get_video_file(&self, file_id: i64) -> AppResult<Option<VideoFileMetadata>>;
    async fn get_audio_file(&self, file_id: i64) -> AppResult<Option<AudioFileMetadata>>;
    async fn update_video_audio(&self, video_file_id: i64, audio_json: &str, default_audio: Option<i64>) -> AppResult<()>;

    // Part + usage queries
    async fn get_platform_usage(&self, platform: &str) -> AppResult<u64>;
    async fn get_part_by_index(&self, file_id: i64, part_index: u32) -> AppResult<Option<crate::provider::storage::PartMetadata>>;
    async fn get_parts_for_file_by_type(&self, file_id: i64, part_type: &str) -> AppResult<Vec<crate::provider::storage::PartMetadata>>;
    async fn mark_file_accessed(&self, file_id: i64) -> AppResult<()>;

    // Playback tracking
    async fn save_playback_history(&self, file_id: i64, position_sec: f64, duration_sec: Option<f64>, completed: bool) -> AppResult<()>;
    async fn clear_playback_history(&self, file_id: i64) -> AppResult<()>;
    async fn get_effective_video_playback(&self, file_id: i64) -> AppResult<Option<VideoPlaybackProgress>>;

    // Upload parts persistence
    async fn get_original_parts_for_file(&self, file_id: i64) -> AppResult<Vec<crate::provider::storage::PartMetadata>>;
    async fn insert_part(
        &self,
        file_id: i64,
        platform: &str,
        message_id: &str,
        attachment_name: Option<&str>,
        part_index: u32,
        size: i64,
        part_type: &str,
        checksum: Option<String>,
    ) -> AppResult<()>;
    async fn delete_parts_by_type(&self, file_id: i64, part_type: &str) -> AppResult<()>;

    // Media metadata persistence
    async fn upsert_video_file(
        &self,
        file_id: i64,
        duration_sec: Option<f64>,
        width: Option<u32>,
        height: Option<u32>,
        fps: Option<f64>,
        bitrate_bps: Option<i64>,
        video_codec: Option<&str>,
        audio_codec: Option<&str>,
        container: Option<&str>,
    ) -> AppResult<()>;
    async fn upsert_audio_file(
        &self,
        file_id: i64,
        duration_sec: Option<f64>,
        bitrate_bps: Option<i64>,
        sample_rate_hz: Option<u32>,
        channels: Option<u32>,
        audio_codec: Option<&str>,
        container: Option<&str>,
    ) -> AppResult<()>;
    async fn upsert_image_file(
        &self,
        file_id: i64,
        width: Option<u32>,
        height: Option<u32>,
        format: Option<&str>,
        color_space: Option<&str>,
        orientation: Option<&str>,
    ) -> AppResult<()>;
    async fn set_file_kind(&self, file_id: i64, kind: &str) -> AppResult<()>;
    async fn get_file_kind(&self, file_id: i64) -> AppResult<Option<String>>;
}
