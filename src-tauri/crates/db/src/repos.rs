use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::RwLock as TokioRwLock;

use omega_drive_gateway::core::error::AppResult;
use omega_drive_gateway::provider::download_job_repository::DownloadJobRepository;
use omega_drive_gateway::provider::drive_stats_cache_repository::DriveStatsCacheRepository;
use omega_drive_gateway::provider::feature_log::FeatureLog;
use omega_drive_gateway::provider::file_repository::FileRepository;
use omega_drive_gateway::provider::folder_repository::FolderRepository;
use omega_drive_gateway::provider::transfer_progress::TransferProgress;
use omega_drive_gateway::provider::upload_job_repository::UploadJobRepository;
use omega_drive_gateway::provider::upload_profile_repository::UploadProfileRepository;
use omega_drive_gateway::core::types::ProgressInfo;
use omega_drive_gateway::upload::upload_plan::UploadProfile;
use omega_drive_gateway::upload::upload_rules::UploadProfileRule;
use omega_drive_gateway::provider::storage::PartMetadata;

use omega_drive_gateway::core::data::{DownloadJob, UploadJob};
use crate::drive_stats_cache::DriveStats;
use crate::files::{AudioFileMetadata, FileMetadata, VideoFileMetadata, VideoPlaybackProgress};
use crate::{DbWriteQueue, ReadDbPool};

pub struct DbFileRepository {
    db_read: Arc<ReadDbPool>,
    db_write: Arc<DbWriteQueue>,
    cache_db: Option<Mutex<rusqlite::Connection>>,
}

impl DbFileRepository {
    pub fn new(db_read: Arc<ReadDbPool>, db_write: Arc<DbWriteQueue>) -> Self {
        Self { db_read, db_write, cache_db: None }
    }

    pub fn with_cache_db(self, path: PathBuf) -> Self {
        let conn = rusqlite::Connection::open(&path).ok();
        Self { cache_db: conn.map(|c| Mutex::new(c)), ..self }
    }

    fn fetch_local_path(&self, file_id: i64) -> Option<String> {
        let conn = self.cache_db.as_ref()?;
        let conn = conn.lock().ok()?;
        let row: Option<String> = conn.query_row(
            "SELECT source_path FROM upload_jobs WHERE file_id = ?",
            rusqlite::params![file_id],
            |r| r.get(0),
        ).ok()?;
        if row.as_deref() == Some("") { None } else { row }
    }
}

#[async_trait]
impl FileRepository for DbFileRepository {
    async fn get_file_by_id(&self, id: i64) -> AppResult<Option<FileMetadata>> {
        let db = self.db_read.lock().await;
        let mut meta = crate::files::get_file_by_id(db.conn(), id)?;
        if let Some(ref mut m) = meta {
            if m.local_path.is_none() {
                m.local_path = self.fetch_local_path(m.id);
            }
        }
        Ok(meta)
    }

    async fn get_file_by_thread_id(&self, thread_id: &str) -> AppResult<Option<FileMetadata>> {
        let db = self.db_read.lock().await;
        let mut meta = crate::files::get_file_by_thread_id(db.conn(), thread_id)?;
        if let Some(ref mut m) = meta {
            if m.local_path.is_none() {
                m.local_path = self.fetch_local_path(m.id);
            }
        }
        Ok(meta)
    }

    async fn get_file_by_name(&self, name: &str, folder_id: Option<i64>) -> AppResult<Option<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_file_by_name(db.conn(), name, folder_id)?)
    }

    async fn get_all_files(&self) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_all_files(db.conn())?)
    }

    async fn get_files_by_parent(&self, folder_id: Option<i64>, drive_scope: Option<&str>) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_files_by_parent(db.conn(), folder_id, drive_scope)?)
    }

    async fn get_recent_files_paginated(&self, cursor: Option<i64>, limit: i64, role_filter: Option<&str>, drive_scope: Option<&str>) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_recent_files_paginated(db.conn(), cursor, limit, role_filter, drive_scope)?)
    }

    async fn get_files_paginated(&self, folder_id: Option<i64>, cursor: Option<i64>, limit: i64, role_filter: Option<&str>, drive_scope: Option<&str>) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_files_paginated(db.conn(), folder_id, cursor, limit, role_filter, drive_scope)?)
    }

    async fn get_all_files_paginated(&self, cursor: Option<i64>, limit: i64, role_filter: Option<&str>, drive_scope: Option<&str>) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_all_files_paginated(db.conn(), cursor, limit, role_filter, drive_scope)?)
    }

    async fn get_trash_paginated(&self, cursor: Option<i64>, limit: i64, role_filter: Option<&str>, drive_scope: Option<&str>) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_trash_paginated(db.conn(), cursor, limit, role_filter, drive_scope)?)
    }

    async fn get_transfers_paginated(&self, cursor: Option<i64>, limit: i64) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_transfers_paginated(db.conn(), cursor, limit)?)
    }

    async fn get_file_stats(&self, drive_scope: Option<&str>) -> AppResult<(i64, i64, i64)> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_file_stats(db.conn(), drive_scope)?)
    }

    async fn insert_file(&self, filename: &str, size: i64, thread_id: &str, folder_id: Option<i64>, drive_scope: &str, checksum: Option<&str>, local_path: Option<&str>) -> AppResult<i64> {
        self.db_write.with_write(|conn| {
            Ok(crate::files::insert_file(conn, filename, size, thread_id, folder_id, drive_scope, checksum, local_path)?)
        }).await
    }

    async fn insert_attachment_file(&self, filename: &str, size: i64, thread_id: &str, folder_id: Option<i64>, drive_scope: &str, checksum: Option<&str>) -> AppResult<i64> {
        self.db_write.with_write(|conn| {
            Ok(crate::files::insert_file(conn, filename, size, thread_id, folder_id, drive_scope, checksum, None)?)
        }).await
    }

    async fn update_file_status(&self, id: i64, status: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::update_file_status(conn, id, status)?)).await
    }

    async fn set_files_error_by_thread_id(&self, thread_id: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| {
            conn.execute(
                "UPDATE files SET status = 'error', deleted_at = CURRENT_TIMESTAMP WHERE thread_id = ?",
                rusqlite::params![thread_id],
            )?;
            Ok(())
        }).await
    }

    async fn rename_file_by_thread_id(&self, thread_id: &str, new_name: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| {
            conn.execute(
                "UPDATE files SET filename = ? WHERE thread_id = ?",
                rusqlite::params![new_name, thread_id],
            )?;
            Ok(())
        }).await
    }

    async fn update_file_checksum(&self, id: i64, checksum: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::update_file_checksum(conn, id, checksum)?)).await
    }

    async fn update_file_folder(&self, id: i64, folder_id: Option<i64>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::update_file_folder(conn, id, folder_id)?)).await
    }

    async fn update_file_name(&self, id: i64, filename: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::update_file_name(conn, id, filename)?)).await
    }

    async fn update_file_local_path(&self, id: i64, local_path: Option<&str>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::update_file_local_path(conn, id, local_path)?)).await
    }

    async fn move_to_trash(&self, id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::move_file_to_trash(conn, id)?)).await
    }

    async fn restore_trash(&self, id: i64) -> AppResult<bool> {
        self.db_write.with_write(|conn| Ok(crate::files::restore_trashed_file(conn, id)?)).await
    }

    async fn toggle_hidden(&self, id: i64, is_hidden: bool) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::toggle_file_hidden(conn, id, is_hidden)?)).await
    }

    async fn toggle_star(&self, id: i64, starred: bool) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::toggle_file_star(conn, id, starred)?)).await
    }

    async fn delete_file(&self, id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::delete_file(conn, id)?)).await
    }

    async fn search_files(&self, query: &str, limit: i64) -> AppResult<Vec<FileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::search_files_limited(db.conn(), query, limit)?)
    }

    async fn get_parts_for_file(&self, file_id: i64) -> AppResult<Vec<PartMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_parts_for_file(db.conn(), file_id)?)
    }

    async fn get_part_counts_for_files(&self, file_ids: &[i64]) -> AppResult<HashMap<i64, usize>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_part_counts_for_files(db.conn(), file_ids)?)
    }

    async fn get_video_file(&self, file_id: i64) -> AppResult<Option<VideoFileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_video_file(db.conn(), file_id)?)
    }

    async fn get_audio_file(&self, file_id: i64) -> AppResult<Option<AudioFileMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_audio_file(db.conn(), file_id)?)
    }

    async fn update_video_audio(&self, video_file_id: i64, audio_json: &str, default_audio: Option<i64>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::set_video_audio(conn, video_file_id, audio_json, default_audio)?)).await
    }

    async fn get_platform_usage(&self, platform: &str) -> AppResult<u64> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_platform_usage(db.conn(), platform)?)
    }

    async fn get_part_by_index(&self, file_id: i64, part_index: u32) -> AppResult<Option<PartMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_part_by_index(db.conn(), file_id, part_index)?)
    }

    async fn get_parts_for_file_by_type(&self, file_id: i64, part_type: &str) -> AppResult<Vec<PartMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_parts_for_file_by_type(db.conn(), file_id, part_type)?)
    }

    async fn mark_file_accessed(&self, file_id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| {
            Ok(crate::files::mark_file_accessed(conn, file_id)?)
        }).await
    }

    async fn save_playback_history(&self, file_id: i64, position_sec: f64, duration_sec: Option<f64>, completed: bool) -> AppResult<()> {
        self.db_write.with_write(|conn| {
            Ok(crate::files::save_playback_history(conn, file_id, position_sec, duration_sec, completed)?)
        }).await
    }

    async fn clear_playback_history(&self, file_id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| {
            Ok(crate::files::clear_playback_history(conn, file_id)?)
        }).await
    }

    async fn get_effective_video_playback(&self, file_id: i64) -> AppResult<Option<VideoPlaybackProgress>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_effective_video_playback(db.conn(), file_id)?)
    }

    async fn get_original_parts_for_file(&self, file_id: i64) -> AppResult<Vec<PartMetadata>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::get_original_parts_for_file(db.conn(), file_id)?)
    }

    async fn insert_part(&self, file_id: i64, platform: &str, message_id: &str, attachment_name: Option<&str>, part_index: u32, size: i64, part_type: &str, checksum: Option<String>) -> AppResult<()> {
        self.db_write.with_write(|conn| {
            crate::files::insert_part(conn, file_id, platform, message_id, attachment_name, part_index, size, part_type, checksum)?;
            Ok(())
        }).await
    }

    async fn delete_parts_by_type(&self, file_id: i64, part_type: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::delete_parts_by_type(conn, file_id, part_type)?)).await
    }

    async fn upsert_video_file(&self, file_id: i64, duration_sec: Option<f64>, width: Option<u32>, height: Option<u32>, fps: Option<f64>, bitrate_bps: Option<i64>, video_codec: Option<&str>, audio_codec: Option<&str>, container: Option<&str>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::upsert_video_file(conn, file_id, duration_sec, width, height, fps, bitrate_bps, video_codec, audio_codec, container)?)).await
    }

    async fn upsert_audio_file(&self, file_id: i64, duration_sec: Option<f64>, bitrate_bps: Option<i64>, sample_rate_hz: Option<u32>, channels: Option<u32>, audio_codec: Option<&str>, container: Option<&str>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::upsert_audio_file(conn, file_id, duration_sec, bitrate_bps, sample_rate_hz, channels, audio_codec, container)?)).await
    }

    async fn upsert_image_file(&self, file_id: i64, width: Option<u32>, height: Option<u32>, format: Option<&str>, color_space: Option<&str>, orientation: Option<&str>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::upsert_image_file(conn, file_id, width, height, format, color_space, orientation)?)).await
    }

    async fn set_file_kind(&self, file_id: i64, kind: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::files::set_file_kind(conn, file_id, kind)?)).await
    }

    async fn get_file_kind(&self, file_id: i64) -> AppResult<Option<String>> {
        let db = self.db_read.lock().await;
        use rusqlite::OptionalExtension;
        let kind: Option<String> = db.conn().query_row(
            "SELECT kind FROM files WHERE id = ?",
            rusqlite::params![file_id],
            |row| row.get(0),
        ).optional()?;
        Ok(kind)
    }

    async fn find_filenames_like(
        &self,
        folder_id: Option<i64>,
        exact: &str,
        like_pattern: &str,
    ) -> AppResult<Vec<String>> {
        let db = self.db_read.lock().await;
        Ok(crate::files::find_filenames_like(db.conn(), folder_id, exact, like_pattern)?)
    }
}

pub struct DbFolderRepository {
    db_write: Arc<DbWriteQueue>,
}

impl DbFolderRepository {
    pub fn new(db_write: Arc<DbWriteQueue>) -> Self { Self { db_write } }
}

#[async_trait]
impl FolderRepository for DbFolderRepository {
    async fn insert_folder(&self, name: &str, parent_id: Option<i64>) -> AppResult<i64> {
        Ok(self.db_write.with_write(|conn| crate::folders::insert_folder(conn, name, parent_id)).await?)
    }

    async fn get_folder_by_id(&self, id: i64) -> AppResult<Option<crate::folders::FolderMetadata>> {
        let db = self.db_write.lock().await;
        Ok(crate::folders::get_folder_by_id(db.conn(), id)?)
    }

    async fn get_all_folders(&self, drive_scope: Option<&str>) -> AppResult<Vec<crate::folders::FolderMetadata>> {
        let db = self.db_write.lock().await;
        Ok(crate::folders::get_all_folders(db.conn(), drive_scope)?)
    }

    async fn get_folders_by_parent(&self, parent_id: Option<i64>, drive_scope: Option<&str>) -> AppResult<Vec<crate::folders::FolderMetadata>> {
        let db = self.db_write.lock().await;
        Ok(crate::folders::get_folders_by_parent(db.conn(), parent_id, drive_scope)?)
    }

    async fn get_folder_by_name(&self, name: &str, parent_id: Option<i64>, drive_scope: Option<&str>) -> AppResult<Option<crate::folders::FolderMetadata>> {
        let db = self.db_write.lock().await;
        Ok(crate::folders::get_folder_by_name(db.conn(), name, parent_id, drive_scope)?)
    }

    async fn update_folder_name(&self, id: i64, name: &str) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::folders::update_folder_name(conn, id, name)?)).await
    }

    async fn update_folder_parent(&self, id: i64, parent_id: Option<i64>) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::folders::update_folder_parent(conn, id, parent_id)?)).await
    }

    async fn delete_folder(&self, id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::folders::delete_folder(conn, id)?)).await
    }

    async fn toggle_folder_star(&self, id: i64, starred: bool) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::folders::toggle_folder_star(conn, id, starred)?)).await
    }
}

pub struct CacheDbUploadJobRepository {
    cache_db: Arc<TokioMutex<rusqlite::Connection>>,
}

impl CacheDbUploadJobRepository {
    pub fn new(cache_db: Arc<TokioMutex<rusqlite::Connection>>) -> Self { Self { cache_db } }
}

#[async_trait]
impl UploadJobRepository for CacheDbUploadJobRepository {
    async fn upsert_job(&self, file_id: i64, source_path: &str, state: &str, total_parts: i64) -> AppResult<i64> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::upsert_job(&conn, file_id, source_path, state, total_parts)?)
    }

    async fn get_active_job_by_source_path(&self, source_path: &str) -> AppResult<Option<UploadJob>> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::get_active_job_by_source_path(&conn, source_path)?)
    }

    async fn get_job_by_file_id(&self, file_id: i64) -> AppResult<Option<UploadJob>> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::get_job_by_file_id(&conn, file_id)?)
    }

    async fn update_source_path(&self, file_id: i64, source_path: &str) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::update_source_path(&conn, file_id, source_path)?)
    }

    async fn update_progress(&self, file_id: i64, done_parts: i64, total_parts: i64) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::update_progress(&conn, file_id, done_parts, total_parts)?)
    }

    async fn update_state(&self, file_id: i64, state: &str, error: Option<&str>, error_code: Option<&str>) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::update_state(&conn, file_id, state, error, error_code)?)
    }

    async fn delete_job_by_file_id(&self, file_id: i64) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::upload_cache::delete_job_by_file_id(&conn, file_id)?)
    }
}

pub struct CacheDbDownloadJobRepository {
    cache_db: Arc<TokioMutex<rusqlite::Connection>>,
}

impl CacheDbDownloadJobRepository {
    pub fn new(cache_db: Arc<TokioMutex<rusqlite::Connection>>) -> Self { Self { cache_db } }
}

#[async_trait]
impl DownloadJobRepository for CacheDbDownloadJobRepository {
    async fn create_job(&self, file_id: i64, target_path: &str, total_parts: i64) -> AppResult<i64> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::create_job(&conn, file_id, target_path, total_parts)?)
    }

    async fn update_progress(&self, id: i64, done_parts: i64) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::update_progress(&conn, id, done_parts)?)
    }

    async fn update_state(&self, id: i64, state: &str, error: Option<&str>, error_code: Option<&str>) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::update_state(&conn, id, state, error, error_code)?)
    }

    async fn get_job(&self, id: i64) -> AppResult<Option<DownloadJob>> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::get_job(&conn, id)?)
    }

    async fn list_jobs_by_state(&self, states: &[&str]) -> AppResult<Vec<DownloadJob>> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::list_jobs_by_state(&conn, states)?)
    }

    async fn get_next_queued(&self) -> AppResult<Option<DownloadJob>> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::get_next_queued(&conn)?)
    }

    async fn exists_active_job_for_file(&self, file_id: i64) -> AppResult<bool> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::exists_active_job_for_file(&conn, file_id)?)
    }

    async fn delete_job(&self, id: i64) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::delete_job(&conn, id)?)
    }

    async fn pause_all_active_jobs(&self, error_code: &str) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::pause_all_active_jobs(&conn, error_code)?)
    }

    async fn resume_shutdown_jobs(&self) -> AppResult<()> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::resume_shutdown_jobs(&conn)?)
    }

    async fn purge_old_jobs(&self, days: i64, states: &[&str]) -> AppResult<usize> {
        let conn = self.cache_db.lock().await;
        Ok(crate::download_cache::purge_old_jobs(&conn, days, states)?)
    }
}

pub struct DbDriveStatsCacheRepository {
    db_write: Arc<DbWriteQueue>,
}

impl DbDriveStatsCacheRepository {
    pub fn new(db_write: Arc<DbWriteQueue>) -> Self { Self { db_write } }
}

#[async_trait]
impl DriveStatsCacheRepository for DbDriveStatsCacheRepository {
    async fn refresh_drive_stats_cache(&self) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::drive_stats_cache::refresh_drive_stats_cache(conn)?)).await
    }

    async fn get_drive_stats_cache(&self, drive_scope: &str) -> AppResult<Option<DriveStats>> {
        let db = self.db_write.lock().await;
        Ok(crate::drive_stats_cache::get_drive_stats_cache(db.conn(), drive_scope)?)
    }
}

pub struct DbUploadProfileRepository {
    db_write: Arc<DbWriteQueue>,
}

impl DbUploadProfileRepository {
    pub fn new(db_write: Arc<DbWriteQueue>) -> Self { Self { db_write } }
}

#[async_trait]
impl UploadProfileRepository for DbUploadProfileRepository {
    async fn get_upload_profiles(&self) -> AppResult<Vec<UploadProfile>> {
        let db = self.db_write.lock().await;
        Ok(crate::upload_profiles::get_upload_profiles(db.conn())?)
    }

    async fn get_profile_by_id(&self, id: i64) -> AppResult<Option<UploadProfile>> {
        let db = self.db_write.lock().await;
        Ok(crate::upload_profiles::get_profile_by_id(db.conn(), id)?)
    }

    async fn save_upload_profile(&self, profile: &UploadProfile) -> AppResult<UploadProfile> {
        self.db_write.with_write(|conn| Ok(crate::upload_profiles::save_upload_profile(conn, profile)?)).await
    }

    async fn delete_upload_profile(&self, id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::upload_profiles::delete_upload_profile(conn, id)?)).await
    }

    async fn restore_default_profiles(&self) -> AppResult<Vec<UploadProfile>> {
        self.db_write.with_write(|conn| Ok(crate::upload_profiles::restore_default_profiles(conn)?)).await
    }

    async fn get_upload_profile_rules(&self, profile_id: Option<i64>) -> AppResult<Vec<UploadProfileRule>> {
        let db = self.db_write.lock().await;
        Ok(crate::upload_profile_rules::get_upload_profile_rules(db.conn(), profile_id)?)
    }

    async fn get_rule_by_id(&self, id: i64) -> AppResult<Option<UploadProfileRule>> {
        let db = self.db_write.lock().await;
        Ok(crate::upload_profile_rules::get_rule_by_id(db.conn(), id)?)
    }

    async fn save_upload_profile_rule(&self, rule: &UploadProfileRule) -> AppResult<UploadProfileRule> {
        self.db_write.with_write(|conn| Ok(crate::upload_profile_rules::save_upload_profile_rule(conn, rule)?)).await
    }

    async fn delete_upload_profile_rule(&self, id: i64) -> AppResult<()> {
        self.db_write.with_write(|conn| Ok(crate::upload_profile_rules::delete_upload_profile_rule(conn, id)?)).await
    }

    async fn save_upload_profile_rules_bulk(&self, profile_id: i64, ordered_rule_ids: &[i64]) -> AppResult<Vec<UploadProfileRule>> {
        self.db_write.with_write(|conn| Ok(crate::upload_profile_rules::save_upload_profile_rules_bulk(conn, profile_id, ordered_rule_ids)?)).await
    }
}

pub struct ProgressMapTransferProgress {
    progress_map: Arc<TokioRwLock<HashMap<String, ProgressInfo>>>,
}

impl ProgressMapTransferProgress {
    pub fn new(progress_map: Arc<TokioRwLock<HashMap<String, ProgressInfo>>>) -> Self { Self { progress_map } }
}

#[async_trait]
impl TransferProgress for ProgressMapTransferProgress {
    async fn report_progress(&self, transfer_id: &str, progress: ProgressInfo) {
        self.progress_map.write().await.insert(transfer_id.to_string(), progress);
    }

    async fn report_completion(&self, transfer_id: &str, _file_id: i64) {
        self.progress_map.write().await.remove(transfer_id);
    }
}

pub struct StateFeatureLog;

impl FeatureLog for StateFeatureLog {
    fn log(&self, _feature: &str, _level: &str, _message: &str) {}

    fn query(&self, _feature: Option<&str>, _limit: usize) -> Vec<String> {
        Vec::new()
    }
}
