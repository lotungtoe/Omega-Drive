use crate::core::backup::Op;

/// Interface for backup operations used by upload feature.
/// Concrete impl is in omega_drive_backup crate.
pub trait BackupService: Send + Sync {
    fn next_seq(&self) -> u64;
    fn push_op(&self, op: Op);
    fn flush_queues(&self);
    fn has_pending(&self) -> bool;
    /// Snapshot file state and push a FileSnapshot op.
    /// Implementor has DB access to build FilePayload internally.
    fn try_capture_file(&self, file_id: i64, status: &str);
}
