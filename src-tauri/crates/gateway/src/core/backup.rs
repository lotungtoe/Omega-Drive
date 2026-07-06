use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePayload {
    pub file: Option<serde_json::Value>,
    pub parts: Vec<serde_json::Value>,
    pub video_file: Option<serde_json::Value>,
    pub audio_file: Option<serde_json::Value>,
    pub image_files: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Op {
    #[serde(rename = "file_snapshot")]
    FileSnapshot {
        seq: u64,
        file_id: i64,
        action: String,
        payload: FilePayload,
    },
    #[serde(rename = "mutation")]
    Mutation {
        seq: u64,
        priority: u8,
        table: String,
        action: String,
        row_id: i64,
    },
}

impl Op {
    pub fn seq(&self) -> u64 {
        match self {
            Op::FileSnapshot { seq, .. } => *seq,
            Op::Mutation { seq, .. } => *seq,
        }
    }
}


