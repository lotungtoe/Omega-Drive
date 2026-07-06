use rusqlite::Connection;
use omega_drive_gateway::core::backup::FilePayload;

pub trait BackupRepository: Send + Sync {
    fn capture_file_state(
        &self,
        conn: &Connection,
        file_id: i64,
    ) -> Result<FilePayload, rusqlite::Error>;
}
