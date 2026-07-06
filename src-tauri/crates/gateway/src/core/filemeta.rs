use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FullMetadata {
    pub format: FormatInfo,
    pub streams: Vec<StreamInfo>,
    pub chapters: Vec<ChapterInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FormatInfo {
    pub duration: Option<String>,
    pub size: Option<String>,
    pub bit_rate: Option<String>,
    pub format_name: Option<String>,
    pub encoder: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamInfo {
    pub index: u32,
    pub codec_type: String,
    pub codec_name: Option<String>,
    pub language: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub bit_rate: Option<String>,
    pub sample_rate: Option<String>,
    pub channels: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChapterInfo {
    pub start_time: String,
    pub end_time: String,
    pub title: Option<String>,
}
