use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    Video,
    Image,
    Audio,
    Document,
    Archive,
    Code,
    Sheet,
    #[serde(alias = "other")]
    Unknown,
}

impl FileType {
    pub fn storage_kind(self) -> &'static str {
        match self {
            FileType::Video => "video",
            FileType::Image => "image",
            FileType::Audio => "audio",
            FileType::Document => "document",
            FileType::Archive => "archive",
            FileType::Code => "code",
            FileType::Sheet => "sheet",
            FileType::Unknown => "unknown",
        }
    }

    pub fn shared_drive_channel(self) -> &'static str {
        self.storage_kind()
    }
}
