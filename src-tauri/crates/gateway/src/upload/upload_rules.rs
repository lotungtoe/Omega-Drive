use serde::{Deserialize, Serialize};

use crate::core::file_types::FileType;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProfileRule {
    pub id: Option<i64>,
    pub profile_id: i64,
    pub priority: i64,
    pub file_type: Option<FileType>,
    pub extensions: Vec<String>,
    pub min_size_bytes: Option<i64>,
    pub max_size_bytes: Option<i64>,
}
