pub use omega_drive_gateway::core::file_types::FileType;
pub use omega_drive_gateway::provider::debug_logger::DebugLogger;
pub use omega_drive_gateway::provider::error_reporter::ErrorReporter;
pub use omega_drive_gateway::core::services::{
    ExtensionNormalizer, FileTypeClassifier, MediaParser, SystemProfileProvider,
};

use crate::data::parse_media_summary;
use crate::file_types::{
    is_audio_file, is_image_file, is_video_file, media_child_kind, normalize_extension,
    normalize_storage_kind, sniff_magic_bytes, storage_kind_from_filename,
};
use crate::file_types::{file_type_from_extension, file_type_from_filename};
use crate::upload_plan::default_system_profiles;
use crate::upload_plan::validate_upload_plan;
use crate::upload_rules::normalize_extensions;

pub fn init_file_classifier() {
    omega_drive_gateway::core::services::set_file_classifier(Box::new(DefaultFileTypeClassifier));
}

pub struct DefaultFileTypeClassifier;

impl FileTypeClassifier for DefaultFileTypeClassifier {
    fn normalize_storage_kind(&self, kind: &str) -> &'static str {
        normalize_storage_kind(kind)
    }
    fn media_child_kind(&self, kind: &str) -> &'static str {
        media_child_kind(kind)
    }
    fn storage_kind_from_filename(&self, filename: &str) -> &'static str {
        storage_kind_from_filename(filename)
    }
    fn is_video_file(&self, filename: &str) -> bool {
        is_video_file(filename)
    }
    fn is_audio_file(&self, filename: &str) -> bool {
        is_audio_file(filename)
    }
    fn is_image_file(&self, filename: &str) -> bool {
        is_image_file(filename)
    }
    fn file_type_from_extension(&self, ext: &str) -> Option<FileType> {
        file_type_from_extension(ext)
    }
    fn normalize_extension(&self, ext: &str) -> Option<String> {
        normalize_extension(ext)
    }
    fn sniff_magic_bytes(&self, buf: &[u8]) -> Option<FileType> {
        sniff_magic_bytes(buf)
    }
    fn file_type_from_filename(&self, filename: &str) -> FileType {
        file_type_from_filename(filename)
    }
}

pub struct DefaultExtensionNormalizer;

impl ExtensionNormalizer for DefaultExtensionNormalizer {
    fn normalize_extensions(&self, list: &[String]) -> Vec<String> {
        normalize_extensions(list)
    }
    fn validate_upload_plan(&self, plan: &omega_drive_gateway::upload::upload_plan::UploadPlan) -> Result<(), String> {
        validate_upload_plan(plan)
    }
}

pub struct DefaultSystemProfileProvider;

impl SystemProfileProvider for DefaultSystemProfileProvider {
    fn default_system_profiles(&self) -> Vec<omega_drive_gateway::upload::upload_plan::UploadProfile> {
        default_system_profiles()
    }
}

pub struct DefaultMediaParser;

impl MediaParser for DefaultMediaParser {
    fn parse_media_summary(&self, raw_json: &str) -> Option<omega_drive_gateway::core::data::ParsedMediaSummary> {
        parse_media_summary(raw_json)
    }
}

pub struct DefaultErrorReporter;

impl ErrorReporter for DefaultErrorReporter {
    fn report(&self, feature: &str, err: omega_drive_gateway::core::error::AppError) -> omega_drive_gateway::core::error::AppError {
        crate::error::report(feature, err)
    }
    fn wrap_error(
        &self,
        feature: &str,
        code: &str,
        message: impl Into<String>,
        context: serde_json::Value,
        err: impl Into<anyhow::Error>,
    ) -> omega_drive_gateway::core::error::AppError {
        crate::error::wrap_error(feature, code, message, context, err)
    }
}

pub struct DefaultDebugLogger;

impl DebugLogger for DefaultDebugLogger {
    fn debug_write(&self, channel: &str, message: &str) {
        crate::debug_log::debug_write(channel, message);
    }
    fn debug_core_init(&self, base_dir: &str) {
        crate::debug_log::debug_core_init(std::path::Path::new(base_dir));
    }
}
