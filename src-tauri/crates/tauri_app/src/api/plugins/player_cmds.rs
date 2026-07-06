use std::path::PathBuf;

use tauri::State;
use tokio_util::sync::CancellationToken;

use crate::app_wiring::app_runtime::AppState;
use omega_drive_upload::coordinator::{run_upload, UploadDataSource};
use omega_drive_gateway::{core::scope::DriveScope, upload::upload_plan::UploadPlan};
use omega_drive_player::nativeplayer::{MpvSessionType, MpvStatus};

#[tauri::command]
pub async fn get_file_part(
    st: State<'_, AppState>,
    file_id: i64,
    part_num: u32,
) -> Result<Vec<u8>, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::get_file_part_internal(st.player_ctx.as_ref(), file_id, part_num).await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn open_in_native_player(
    st: State<'_, AppState>,
    file_id: i64,
    title: String,
    start_position_sec: Option<f64>,
    session_type: Option<MpvSessionType>,
) -> Result<(), String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::open_in_native_player(
            st.player_ctx.as_ref(),
            file_id,
            title,
            start_position_sec,
            session_type,
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn player_update_playback_progress(
    st: State<'_, AppState>,
    file_id: i64,
    position: f64,
) -> Result<(), String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::player_update_playback_progress(
            st.player_ctx.as_ref(),
            file_id,
            position,
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn player_clear_playback_history(
    st: State<'_, AppState>,
    file_id: i64,
) -> Result<(), String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::player_clear_playback_history(
            st.player_ctx.as_ref(),
            file_id,
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn mpv_get_status(
    st: State<'_, AppState>,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::mpv_get_status(st.player_ctx.as_ref(), session_type).await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn mpv_play_pause(
    st: State<'_, AppState>,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::mpv_play_pause(st.player_ctx.as_ref(), session_type).await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn mpv_seek(
    st: State<'_, AppState>,
    position: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::mpv_seek(st.player_ctx.as_ref(), position, session_type)
            .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn mpv_set_volume(
    st: State<'_, AppState>,
    volume: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::mpv_set_volume(
            st.player_ctx.as_ref(),
            volume,
            session_type,
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn mpv_set_speed(
    st: State<'_, AppState>,
    speed: f64,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::mpv_set_speed(
            st.player_ctx.as_ref(),
            speed,
            session_type,
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn mpv_toggle_fullscreen(
    st: State<'_, AppState>,
    session_type: MpvSessionType,
) -> Result<MpvStatus, String> {
    #[cfg(feature = "player")]
    {
        omega_drive_player::nativeplayer::mpv_toggle_fullscreen(
            st.player_ctx.as_ref(),
            session_type,
        )
        .await
    }
    #[cfg(not(feature = "player"))]
    {
        Err("player feature disabled".to_string())
    }
}

#[tauri::command]
pub async fn attach_audio_to_video(
    st: tauri::State<'_, AppState>,
    video_file_id: i64,
    audio_file_ids: Vec<i64>,
    default_audio: Option<i64>,
) -> Result<(), String> {
    crate::features::upload::audio_attach::attach_audio_files(
        &st.upload_context(),
        video_file_id,
        audio_file_ids,
        default_audio,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn upload_audio_attachment(
    st: tauri::State<'_, AppState>,
    video_file_id: i64,
    file_path: String,
) -> Result<i64, String> {
    let video_file = st
        .file_repo
        .get_file_by_id(video_file_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Video file not found".to_string())?;

    let folder_id = video_file.folder_id;
    let drive_scope = video_file
        .drive_scope
        .parse::<DriveScope>()
        .map_err(|e| format!("Invalid drive scope: {}", e))?;

    let session_id = format!(
        "audio-upload-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    let audio_file_id = run_upload(
        st.upload_context(),
        UploadDataSource::File(PathBuf::from(&file_path)),
        folder_id,
        drive_scope,
        session_id,
        UploadPlan::default(),
        Some((video_file_id, "audio".to_string())),
        CancellationToken::new(),
        None,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    st.file_repo
        .toggle_hidden(audio_file_id, true)
        .await
        .map_err(|e| e.to_string())?;

    let existing_audio: Vec<i64> = st
        .file_repo
        .get_video_file(video_file_id)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|vm| {
            vm.audio
                .and_then(|json| serde_json::from_str(&json).ok())
        })
        .unwrap_or_default();

    let mut all_audio = existing_audio;
    all_audio.push(audio_file_id);

    crate::features::upload::audio_attach::attach_audio_files(
        &st.upload_context(),
        video_file_id,
        all_audio,
        Some(audio_file_id),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(audio_file_id)
}

#[tauri::command]
pub async fn add_audio_track(
    st: State<'_, AppState>,
    video_file_id: i64,
    audio_file_id: i64,
) -> Result<(), String> {
    #[cfg(feature = "player")] {
        omega_drive_player::nativeplayer::mpv_add_audio_track(
            st.player_ctx.as_ref(),
            video_file_id,
            audio_file_id,
        )
        .await
    }
    #[cfg(not(feature = "player"))] {
        Err("player feature disabled".to_string())
    }
}
