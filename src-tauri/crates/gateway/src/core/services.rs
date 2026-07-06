use crate::core::file_types::FileType;
use crate::upload::upload_plan::UploadPlan;

pub trait FileTypeClassifier: Send + Sync {
    fn normalize_storage_kind(&self, kind: &str) -> &'static str;
    fn media_child_kind(&self, kind: &str) -> &'static str;
    fn storage_kind_from_filename(&self, filename: &str) -> &'static str;
    fn is_video_file(&self, filename: &str) -> bool;
    fn is_audio_file(&self, filename: &str) -> bool;
    fn is_image_file(&self, filename: &str) -> bool;
    fn file_type_from_extension(&self, ext: &str) -> Option<FileType>;
    fn normalize_extension(&self, ext: &str) -> Option<String>;
    fn sniff_magic_bytes(&self, buf: &[u8]) -> Option<FileType>;
    fn file_type_from_filename(&self, filename: &str) -> FileType;
}

pub trait ExtensionNormalizer: Send + Sync {
    fn normalize_extensions(&self, list: &[String]) -> Vec<String>;
    fn validate_upload_plan(&self, plan: &UploadPlan) -> Result<(), String>;
}

pub trait SystemProfileProvider: Send + Sync {
    fn default_system_profiles(&self) -> Vec<crate::upload::upload_plan::UploadProfile>;
}

pub trait MediaParser: Send + Sync {
    fn parse_media_summary(&self, raw_json: &str) -> Option<crate::core::data::ParsedMediaSummary>;
}
