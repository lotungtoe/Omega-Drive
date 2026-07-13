use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// db::files::FileMetadata
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub id: i64,
    pub filename: String,
    pub size: i64,
    pub thread_id: String,
    pub folder_id: Option<i64>,
    pub drive_scope: String,
    pub checksum: Option<String>,
    pub status: String,
    pub created_at: String,
    pub deleted_at: Option<String>,
    pub starred: bool,
    pub local_path: Option<String>,
    pub kind: String,
    pub duration_sec: Option<f64>,
    pub is_hidden: bool,
    pub last_accessed_at: Option<i64>,
}

// ---------------------------------------------------------------------------
// db::files::media_children — video / audio metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoFileMetadata {
    pub file_id: i64,
    pub duration_sec: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,
    pub bitrate_bps: Option<i64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub container: Option<String>,
    pub resume_position_sec: Option<f64>,
    pub resume_part_index: Option<u32>,
    pub completed: bool,
    pub playback_updated_at: Option<String>,
    pub audio: Option<String>,
    pub default_audio: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioFileMetadata {
    pub file_id: i64,
    pub duration_sec: Option<f64>,
    pub bitrate_bps: Option<i64>,
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u32>,
    pub audio_codec: Option<String>,
    pub container: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedMediaSummary {
    pub duration_sec: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bitrate_bps: Option<i64>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_bitrate_bps: Option<i64>,
    pub audio_codec_only: Option<String>,
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u32>,
    pub container: Option<String>,
}

// ---------------------------------------------------------------------------
// db::files::VideoPlaybackProgress
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VideoPlaybackProgress {
    pub file_id: i64,
    pub position_sec: f64,
    pub duration_sec: Option<f64>,
    pub resume_part_index: u32,
}

// ---------------------------------------------------------------------------
// db::folders::FolderMetadata
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FolderMetadata {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub starred: bool,
    pub drive_scope: String,
}

// ---------------------------------------------------------------------------
// UploadJob
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadJob {
    pub id: i64,
    pub file_id: i64,
    pub source_path: String,
    pub state: String,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub done_parts: i64,
    pub total_parts: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

// ---------------------------------------------------------------------------
// DownloadJob
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct DownloadJob {
    pub id: i64,
    pub file_id: i64,
    pub target_path: String,
    pub state: String,
    pub error: Option<String>,
    pub error_code: Option<String>,
    pub total_parts: i64,
    pub done_parts: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

// ---------------------------------------------------------------------------
// db::drive_stats_cache::DriveStats
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct DriveStats {
    pub total_files: i64,
    pub total_folders: i64,
    pub total_size: i64,
    pub trash_count: i64,
}
