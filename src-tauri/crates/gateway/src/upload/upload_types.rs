use crate::provider::provider_types::RemoteUploadTarget;
use crate::provider::storage::PartMetadata;

#[derive(Clone)]
pub struct UploadRecordContext {
    pub file_sqlite_id: i64,
    pub upload_target: RemoteUploadTarget,
    pub thread_id_str: String,
    pub thread_to_archive: Option<u64>,
    pub existing_parts: Vec<PartMetadata>,
}

pub struct UploadedPart {
    pub message_id: i64,
    pub platform: String,
    pub attachment_name: Option<String>,
    pub part_index: u32,
    pub size: u64,
    pub logical_size: Option<u64>,
    pub checksum: Option<String>,
}
