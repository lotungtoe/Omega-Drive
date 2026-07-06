use crate::PlayerContext;
use omega_drive_gateway::core::data::FileMetadata;

/// Kiem tra va dam bao video da san sang de phat.
/// Tra ve metadata cua file neu trang thai la ready.
pub async fn ensure_video_playback_ready(
    state: &PlayerContext,
    file_id: i64,
) -> Result<FileMetadata, String> {
    let file = state.file_repo.get_file_by_id(file_id)
        .await
        .map_err(|e| format!("Loi DB File: {e}"))?
        .ok_or_else(|| format!("Khong tim thay file ID {file_id}"))?;

    if file.status != "ready" {
        return Err(format!(
            "Video \"{}\" chua san sang de phat. Trang thai: {}. Hay doi upload hoan tat.",
            file.filename, file.status
        ));
    }

    Ok(file)
}
