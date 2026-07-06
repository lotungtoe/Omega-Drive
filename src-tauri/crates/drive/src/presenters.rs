use std::collections::HashMap;

use omega_drive_gateway::core::data::FileMetadata;
use omega_drive_gateway::core::data::FolderMetadata;
use omega_drive_gateway::core::services::FileTypeClassifier;
use serde_json::{json, Value};

pub fn file_to_client_value(f: &FileMetadata) -> Value {
    json!({
        "id": f.id,
        "filename": f.filename,
        "size": f.size,
        "threadId": f.thread_id,
        "folderId": f.folder_id,
        "driveScope": f.drive_scope,
        "checksum": f.checksum,
        "status": f.status,
        "createdAt": f.created_at,
        "deletedAt": f.deleted_at,
        "starred": f.starred,
        "localPath": f.local_path,
        "kind": f.kind,
        "durationSec": f.duration_sec,
        "isHidden": f.is_hidden,
        "lastAccessedAt": f.last_accessed_at,
    })
}

pub fn folder_to_client_value(f: &FolderMetadata) -> Value {
    json!({
        "id": f.id,
        "name": f.name,
        "parentId": f.parent_id,
        "starred": f.starred,
        "driveScope": f.drive_scope,
    })
}

pub fn map_files_with_progress(
    files: Vec<FileMetadata>,
    classifier: &dyn FileTypeClassifier,
    part_counts: &HashMap<i64, usize>,
) -> Vec<Value> {
    files
        .into_iter()
        .map(|f| {
            let mut v = file_to_client_value(&f);
            if let Some(count) = part_counts.get(&f.id) {
                let kind = classifier.normalize_storage_kind(&f.kind);
                v["partCount"] = json!(*count);
                v["storageKind"] = json!(kind);
            }
            v
        })
        .collect()
}
