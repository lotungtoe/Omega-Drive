use rusqlite::{Connection, Result};

use crate::files::{
    clear_video_progress, derive_resume_part_index, get_video_file, save_video_progress,
    VideoPlaybackProgress,
};

// map_playback_history and get_playback_history were removed because the table was deleted.

pub fn get_effective_video_playback(
    conn: &Connection,
    file_id: i64,
) -> Result<Option<VideoPlaybackProgress>> {
    if let Some(video) = get_video_file(conn, file_id)? {
        if video.completed {
            return Ok(None);
        }

        if let (Some(position_sec), Some(resume_part_index)) = (
            video
                .resume_position_sec
                .filter(|value| value.is_finite() && *value > 0.0),
            video.resume_part_index.filter(|value| *value > 0),
        ) {
            return Ok(Some(VideoPlaybackProgress {
                file_id,
                position_sec,
                duration_sec: video.duration_sec,
                resume_part_index,
            }));
        }
    }

    Ok(None)
}

pub fn save_playback_history(
    conn: &Connection,
    file_id: i64,
    position_sec: f64,
    duration_sec: Option<f64>,
    completed: bool,
) -> Result<()> {
    let normalized_position = if position_sec.is_finite() {
        position_sec.max(0.0)
    } else {
        0.0
    };
    let normalized_duration = duration_sec.filter(|value| value.is_finite() && *value > 0.0);

    if completed || normalized_position <= 0.0 {
        clear_playback_history(conn, file_id)?;
        return Ok(());
    }

    let derived_part_index =
        derive_resume_part_index(conn, file_id, normalized_position)?.or_else(|| {
            derive_resume_part_index_from_duration(
                conn,
                file_id,
                normalized_position,
                normalized_duration,
            )
        });
    save_video_progress(
        conn,
        file_id,
        normalized_position,
        normalized_duration,
        derived_part_index,
        completed,
    )?;

    Ok(())
}

pub fn clear_playback_history(conn: &Connection, file_id: i64) -> Result<()> {
    clear_video_progress(conn, file_id)?;
    Ok(())
}

fn derive_resume_part_index_from_duration(
    conn: &Connection,
    file_id: i64,
    position_sec: f64,
    duration_sec: Option<f64>,
) -> Option<u32> {
    let duration_sec = duration_sec.filter(|value| value.is_finite() && *value > 0.0)?;
    if !position_sec.is_finite() || position_sec <= 0.0 {
        return None;
    }

    let parts = crate::files::get_parts_for_file(conn, file_id).ok()?;
    if parts.is_empty() {
        return None;
    }

    let total_parts = parts.len() as f64;
    let normalized = (position_sec / duration_sec).clamp(0.0, 1.0);
    let part_index = (normalized * total_parts).ceil() as u32;
    Some(part_index.max(1))
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::*;
    use crate::{files as db_files, migrations::run_migrations};

    fn open_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        run_migrations(&conn).expect("run migrations");
        conn
    }

    fn seed_video_file(conn: &Connection, filename: &str) -> i64 {
        let file_id = db_files::insert_file(
            conn,
            filename,
            2_048,
            "channel-1",
            None,
            "my",
            Some("checksum-1"),
            Some("C:\\video.mp4"),
        )
        .expect("insert file");
        db_files::update_file_status(conn, file_id, "ready").expect("mark ready");
        file_id
    }

    fn seed_zero_based_chunk_parts(conn: &Connection, file_id: i64) {
        let _part0 = db_files::insert_part(
            conn,
            file_id,
            "discord",
            "msg-0",
            None,
            0,
            1_024,
            "chunk",
            None,
        )
        .expect("insert first chunk");
        let _part1 = db_files::insert_part(
            conn,
            file_id,
            "discord",
            "msg-1",
            None,
            1,
            1_024,
            "chunk",
            None,
        )
        .expect("insert second chunk");
    }

    #[test]
    fn save_playback_history_dual_writes_with_one_based_resume_ordinal() {
        let conn = open_conn();
        let file_id = seed_video_file(&conn, "sample.mp4");
        seed_zero_based_chunk_parts(&conn, file_id);

        save_playback_history(&conn, file_id, 3.5, Some(20.0), false).expect("save playback");

        // playback_history table acts as a shadow now removed.
        // We only check video_files via get_video_file.

        let video = db_files::get_video_file(&conn, file_id)
            .expect("get video file")
            .expect("video row exists");
        assert_eq!(video.resume_position_sec, Some(3.5));
        assert_eq!(video.resume_part_index, Some(1));

        let effective = get_effective_video_playback(&conn, file_id)
            .expect("get effective playback")
            .expect("effective playback exists");
        assert_eq!(effective.resume_part_index, 1);
        assert_eq!(effective.position_sec, 3.5);
    }

    #[test]
    fn playback_progress_requires_derivable_resume_part() {
        let conn = open_conn();
        let file_id = seed_video_file(&conn, "sample.mp4");

        // Manually insert into video_files with a position but no parts yet
        // to simulate when derive_resume_part_index fails.
        conn.execute(
            "UPDATE video_files SET resume_position_sec = ?, duration_sec = ? WHERE file_id = ?",
            params![7.0_f64, 20.0_f64, file_id],
        )
        .expect("simulated update");

        assert!(
            get_effective_video_playback(&conn, file_id)
                .expect("query effective playback")
                .is_none() // Since resume_part_index is NULL
        );
    }

    #[test]
    fn clear_playback_history_clears_legacy_and_normalized_progress() {
        let conn = open_conn();
        let file_id = seed_video_file(&conn, "sample.mp4");
        seed_zero_based_chunk_parts(&conn, file_id);
        save_playback_history(&conn, file_id, 5.0, Some(20.0), false).expect("save playback");

        clear_playback_history(&conn, file_id).expect("clear playback");

        // Table deleted, check is no longer needed.

        let video = db_files::get_video_file(&conn, file_id)
            .expect("get video row")
            .expect("video row exists");
        assert_eq!(video.resume_position_sec, None);
        assert_eq!(video.resume_part_index, None);
    }
}

