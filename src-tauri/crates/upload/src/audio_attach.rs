use crate::context::UploadContext;
use crate::error::{UploadError, UploadResult};

pub async fn attach_audio_files(
    state: &UploadContext,
    video_file_id: i64,
    audio_file_ids: Vec<i64>,
    default_audio: Option<i64>,
) -> UploadResult<()> {
    for &id in &audio_file_ids {
        let file = state.file_repo.get_file_by_id(id)
            .await
            .map_err(|e| UploadError::internal("DB error checking audio file", e))?
            .ok_or_else(|| UploadError::internal("Audio file not found", format!("file_id={}", id)))?;
        if !file.is_hidden {
            return Err(UploadError::create_validation_error(format!("Audio file must be hidden: file_id={}", id)));
        }
    }

    let audio_json = serde_json::to_string(&audio_file_ids)
        .map_err(|e| UploadError::internal("Failed to serialize audio list", e))?;

    state.file_repo.update_video_audio(video_file_id, &audio_json, default_audio)
        .await
        .map_err(|e| UploadError::internal("Failed to update video audio", e))?;

    Ok(())
}
