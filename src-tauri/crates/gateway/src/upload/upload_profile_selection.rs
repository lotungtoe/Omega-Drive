use crate::core::file_types::FileType;

#[derive(Debug, Clone)]
pub struct UploadProfileCandidate {
    pub file_type: FileType,
    pub extension: Option<String>,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadProfileSelection {
    pub profile_id: Option<i64>,
    pub rule_id: Option<i64>,
}


