use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};

pub use omega_drive_gateway::core::data::{AudioFileMetadata, ParsedMediaSummary, VideoFileMetadata};

use crate::services;

use super::get_parts_for_file;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageFileMetadata {
    pub file_id: i64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub color_space: Option<String>,
    pub orientation: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypedFileAudit {
    pub missing_video_child: i64,
    pub missing_audio_child: i64,
    pub missing_image_child: i64,
    pub unexpected_video_child: i64,
    pub unexpected_audio_child: i64,
    pub unexpected_image_child: i64,
    pub invalid_video_progress: i64,
}

pub fn infer_file_kind(filename: &str) -> &'static str {
    services::file_classifier().storage_kind_from_filename(filename)
}

fn normalize_kind(kind: &str) -> &'static str {
    services::file_classifier().normalize_storage_kind(kind)
}

fn format_resolution(width: Option<u32>, height: Option<u32>) -> Option<String> {
    match (width, height) {
        (Some(width), Some(height)) if width > 0 && height > 0 => {
            Some(format!("{width} x {height}"))
        }
        _ => None,
    }
}

fn parse_resolution(value: Option<String>) -> (Option<u32>, Option<u32>) {
    let Some(value) = value else {
        return (None, None);
    };
    let normalized = value.replace('×', "x");
    let mut parts = normalized.split('x').map(str::trim);
    let width = parts.next().and_then(|value| value.parse::<u32>().ok());
    let height = parts.next().and_then(|value| value.parse::<u32>().ok());
    (width, height)
}

fn count_query(conn: &Connection, sql: &str) -> Result<i64> {
    conn.query_row(sql, [], |row| row.get(0))
}

pub fn ensure_media_child_row(conn: &Connection, file_id: i64, kind: &str) -> Result<()> {
    let kind = services::file_classifier().media_child_kind(kind);

    // Optimization: Check if the child row of the same kind already exists to avoid redundant writes/deletes.
    let exists = match kind {
        "video" => conn
            .query_row(
                "SELECT 1 FROM video_files WHERE file_id = ?",
                [file_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false),
        "audio" => conn
            .query_row(
                "SELECT 1 FROM audio_files WHERE file_id = ?",
                [file_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false),
        "image" => conn
            .query_row(
                "SELECT 1 FROM image_files WHERE file_id = ?",
                [file_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false),
        _ => true,
    };
    if exists {
        return Ok(());
    }

    match kind {
        "video" => {
            conn.execute(
                "INSERT OR IGNORE INTO video_files (file_id) VALUES (?)",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM audio_files WHERE file_id = ?",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM image_files WHERE file_id = ?",
                params![file_id],
            )?;
        }
        "audio" => {
            conn.execute(
                "INSERT OR IGNORE INTO audio_files (file_id) VALUES (?)",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM video_files WHERE file_id = ?",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM image_files WHERE file_id = ?",
                params![file_id],
            )?;
        }
        "image" => {
            conn.execute(
                "INSERT OR IGNORE INTO image_files (file_id) VALUES (?)",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM video_files WHERE file_id = ?",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM audio_files WHERE file_id = ?",
                params![file_id],
            )?;
        }
        _ => {
            conn.execute(
                "DELETE FROM video_files WHERE file_id = ?",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM audio_files WHERE file_id = ?",
                params![file_id],
            )?;
            conn.execute(
                "DELETE FROM image_files WHERE file_id = ?",
                params![file_id],
            )?;
        }
    }
    Ok(())
}

pub fn set_file_kind(conn: &Connection, file_id: i64, kind: &str) -> Result<()> {
    let kind = normalize_kind(kind);
    conn.execute(
        "UPDATE files SET kind = ? WHERE id = ?",
        params![kind, file_id],
    )?;
    ensure_media_child_row(conn, file_id, kind)?;
    Ok(())
}

pub fn upsert_video_file(
    conn: &Connection,
    file_id: i64,
    duration_sec: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
    bitrate_bps: Option<i64>,
    video_codec: Option<&str>,
    audio_codec: Option<&str>,
    container: Option<&str>,
) -> Result<()> {
    ensure_media_child_row(conn, file_id, "video")?;
    let resolution = format_resolution(width, height);
    conn.execute(
        "INSERT INTO video_files (
            file_id, duration_sec, resolution, fps, bitrate_bps, video_codec, audio_codec, container,
            audio, default_audio
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(file_id) DO UPDATE SET
            duration_sec = COALESCE(excluded.duration_sec, video_files.duration_sec),
            resolution = COALESCE(excluded.resolution, video_files.resolution),
            fps = COALESCE(excluded.fps, video_files.fps),
            bitrate_bps = COALESCE(excluded.bitrate_bps, video_files.bitrate_bps),
            video_codec = COALESCE(excluded.video_codec, video_files.video_codec),
            audio_codec = COALESCE(excluded.audio_codec, video_files.audio_codec),
            container = COALESCE(excluded.container, video_files.container)",
        params![
            file_id,
            duration_sec,
            resolution,
            fps,
            bitrate_bps,
            video_codec,
            audio_codec,
            container,
            None::<String>,   // audio
            None::<i64>        // default_audio
        ],
    )?;
    Ok(())
}

pub fn upsert_audio_file(
    conn: &Connection,
    file_id: i64,
    duration_sec: Option<f64>,
    bitrate_bps: Option<i64>,
    sample_rate_hz: Option<u32>,
    channels: Option<u32>,
    audio_codec: Option<&str>,
    container: Option<&str>,
) -> Result<()> {
    ensure_media_child_row(conn, file_id, "audio")?;
    conn.execute(
        "INSERT INTO audio_files (
            file_id, duration_sec, bitrate_bps, sample_rate_hz, channels, audio_codec, container
         ) VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(file_id) DO UPDATE SET
            duration_sec = COALESCE(excluded.duration_sec, audio_files.duration_sec),
            bitrate_bps = COALESCE(excluded.bitrate_bps, audio_files.bitrate_bps),
            sample_rate_hz = COALESCE(excluded.sample_rate_hz, audio_files.sample_rate_hz),
            channels = COALESCE(excluded.channels, audio_files.channels),
            audio_codec = COALESCE(excluded.audio_codec, audio_files.audio_codec),
            container = COALESCE(excluded.container, audio_files.container)",
        params![
            file_id,
            duration_sec,
            bitrate_bps,
            sample_rate_hz,
            channels,
            audio_codec,
            container
        ],
    )?;
    Ok(())
}

pub fn upsert_image_file(
    conn: &Connection,
    file_id: i64,
    width: Option<u32>,
    height: Option<u32>,
    format: Option<&str>,
    color_space: Option<&str>,
    orientation: Option<&str>,
) -> Result<()> {
    ensure_media_child_row(conn, file_id, "image")?;
    let resolution = format_resolution(width, height);
    conn.execute(
        "INSERT INTO image_files (
            file_id, resolution, format, color_space, orientation
         ) VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(file_id) DO UPDATE SET
            resolution = COALESCE(excluded.resolution, image_files.resolution),
            format = COALESCE(excluded.format, image_files.format),
            color_space = COALESCE(excluded.color_space, image_files.color_space),
            orientation = COALESCE(excluded.orientation, image_files.orientation)",
        params![
            file_id,
            resolution,
            format,
            color_space,
            orientation,
        ],
    )?;
    Ok(())
}

pub fn get_video_file(conn: &Connection, file_id: i64) -> Result<Option<VideoFileMetadata>> {
    conn.query_row(
        "SELECT file_id, duration_sec, resolution, fps, bitrate_bps, video_codec, audio_codec, container, resume_position_sec, resume_part_index, audio, default_audio
         FROM video_files
         WHERE file_id = ?",
        params![file_id],
        |row| {
            let (width, height) = parse_resolution(row.get::<_, Option<String>>(2)?);
            Ok(VideoFileMetadata {
                file_id: row.get(0)?,
                duration_sec: row.get(1)?,
                width,
                height,
                fps: row.get(3)?,
                bitrate_bps: row.get(4)?,
                video_codec: row.get(5)?,
                audio_codec: row.get(6)?,
                container: row.get(7)?,
                resume_position_sec: row.get(8)?,
                resume_part_index: row.get(9)?,
                completed: false,
                playback_updated_at: None,
                audio: row.get(10)?,
                default_audio: row.get(11)?,
            })
        },
    )
    .optional()
}

pub fn get_audio_file(conn: &Connection, file_id: i64) -> Result<Option<AudioFileMetadata>> {
    conn.query_row(
        "SELECT file_id, duration_sec, bitrate_bps, sample_rate_hz, channels, audio_codec, container
         FROM audio_files
         WHERE file_id = ?",
        params![file_id],
        |row| {
            Ok(AudioFileMetadata {
                file_id: row.get(0)?,
                duration_sec: row.get(1)?,
                bitrate_bps: row.get(2)?,
                sample_rate_hz: row.get(3)?,
                channels: row.get(4)?,
                audio_codec: row.get(5)?,
                container: row.get(6)?,
            })
        },
    )
    .optional()
}

pub fn derive_resume_part_index(
    conn: &Connection,
    file_id: i64,
    position_sec: f64,
) -> Result<Option<u32>> {
    if !position_sec.is_finite() || position_sec <= 0.0 {
        return Ok(None);
    }

    let parts = get_parts_for_file(conn, file_id)?;
    if parts.is_empty() {
        return Ok(None);
    }

    let total_duration: Option<f64> = conn
        .query_row(
            "SELECT duration_sec FROM video_files WHERE file_id = ?",
            params![file_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten()
        .filter(|value: &f64| value.is_finite() && *value > 0.0);

    let Some(total_duration) = total_duration else {
        return Ok(None);
    };

    let total_parts = parts.len() as f64;
    if total_parts <= 0.0 {
        return Ok(None);
    }

    let normalized = (position_sec / total_duration).clamp(0.0, 1.0);
    let part_index = (normalized * total_parts).ceil() as u32;
    Ok(Some(part_index.max(1)))
}

pub fn save_video_progress(
    conn: &Connection,
    file_id: i64,
    position_sec: f64,
    duration_sec: Option<f64>,
    derived_part_index: Option<u32>,
    completed: bool,
) -> Result<()> {
    ensure_media_child_row(conn, file_id, "video")?;

    let normalized_duration = duration_sec.filter(|value| value.is_finite() && *value > 0.0);
    let normalized_progress = if position_sec.is_finite() && position_sec > 0.0 {
        derived_part_index
            .filter(|value| *value > 0)
            .map(|resume_part_index| (position_sec, resume_part_index))
    } else {
        None
    };

    conn.execute(
        "UPDATE video_files
         SET duration_sec = COALESCE(?, duration_sec),
             resume_position_sec = ?,
             resume_part_index = ?
         WHERE file_id = ?",
        params![
            normalized_duration,
            normalized_progress.map(|(position_sec, _)| position_sec),
            normalized_progress.map(|(_, resume_part_index)| resume_part_index),
            file_id
        ],
    )?;
    let _ = completed;
    Ok(())
}

pub fn clear_video_progress(conn: &Connection, file_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE video_files
         SET resume_position_sec = NULL,
             resume_part_index = NULL
         WHERE file_id = ?",
        params![file_id],
    )?;
    Ok(())
}

pub fn collect_typed_file_audit(conn: &Connection) -> Result<TypedFileAudit> {
    Ok(TypedFileAudit {
        missing_video_child: count_query(
            conn,
            "SELECT COUNT(*)
             FROM files f
             LEFT JOIN video_files vf ON vf.file_id = f.id
             WHERE f.kind = 'video' AND vf.file_id IS NULL",
        )?,
        missing_audio_child: count_query(
            conn,
            "SELECT COUNT(*)
             FROM files f
             LEFT JOIN audio_files af ON af.file_id = f.id
             WHERE f.kind = 'audio' AND af.file_id IS NULL",
        )?,
        missing_image_child: count_query(
            conn,
            "SELECT COUNT(*)
             FROM files f
             LEFT JOIN image_files img ON img.file_id = f.id
             WHERE f.kind = 'image' AND img.file_id IS NULL",
        )?,
        unexpected_video_child: count_query(
            conn,
            "SELECT COUNT(*)
             FROM video_files vf
             LEFT JOIN files f ON f.id = vf.file_id
             WHERE f.id IS NULL OR f.kind != 'video'",
        )?,
        unexpected_audio_child: count_query(
            conn,
            "SELECT COUNT(*)
             FROM audio_files af
             LEFT JOIN files f ON f.id = af.file_id
             WHERE f.id IS NULL OR f.kind != 'audio'",
        )?,
        unexpected_image_child: count_query(
            conn,
            "SELECT COUNT(*)
             FROM image_files img
             LEFT JOIN files f ON f.id = img.file_id
             WHERE f.id IS NULL OR f.kind != 'image'",
        )?,
        invalid_video_progress: count_query(
            conn,
            "SELECT COUNT(*)
             FROM video_files
             WHERE NOT (
                 (resume_position_sec IS NULL AND resume_part_index IS NULL)
                 OR (resume_position_sec > 0 AND resume_part_index > 0)
             )",
        )?,
    })
}

pub fn backfill_typed_file_tables(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.filename, f.kind
         FROM files f
         ORDER BY f.id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    for row in rows {
        let (file_id, filename, existing_kind) = row?;
        let parsed = None::<ParsedMediaSummary>;

        let inferred_kind = if let Some(kind) = existing_kind.as_deref() {
            normalize_kind(kind)
        } else {
            infer_file_kind(&filename)
        };

        set_file_kind(conn, file_id, inferred_kind)?;

        match inferred_kind {
            "video" => {
                let duration_sec = parsed.as_ref().and_then(|meta| meta.duration_sec);
                upsert_video_file(
                    conn,
                    file_id,
                    duration_sec,
                    parsed.as_ref().and_then(|meta| meta.width),
                    parsed.as_ref().and_then(|meta| meta.height),
                    None,
                    parsed.as_ref().and_then(|meta| meta.bitrate_bps),
                    parsed.as_ref().and_then(|meta| meta.video_codec.as_deref()),
                    parsed.as_ref().and_then(|meta| meta.audio_codec.as_deref()),
                    parsed.as_ref().and_then(|meta| meta.container.as_deref()),
                )?;

                // backfill_playback_history removed — table no longer exists.
                // Progress is stored directly in video_files now.
            }
            "audio" => {
                upsert_audio_file(
                    conn,
                    file_id,
                    parsed.as_ref().and_then(|meta| meta.duration_sec),
                    parsed.as_ref().and_then(|meta| meta.audio_bitrate_bps),
                    None,
                    None,
                    parsed
                        .as_ref()
                        .and_then(|meta| meta.audio_codec_only.as_deref()),
                    parsed.as_ref().and_then(|meta| meta.container.as_deref()),
                )?;
            }
            "image" => {
                upsert_image_file(conn, file_id, None, None, None, None, None)?;
            }
            _ => {}
        }
    }

    Ok(())
}

pub fn set_video_audio(conn: &Connection, video_file_id: i64, audio_json: &str, default_audio: Option<i64>) -> Result<()> {
    conn.execute(
        "UPDATE video_files SET audio = ?, default_audio = ? WHERE file_id = ?",
        params![audio_json, default_audio, video_file_id],
    )?;
    Ok(())
}

pub fn parse_media_summary(raw_json: &str) -> Option<ParsedMediaSummary> {
    services::media_parser().parse_media_summary(raw_json)
}
