use std::sync::OnceLock;
use omega_drive_gateway::core::services::{
    ExtensionNormalizer, FileTypeClassifier, MediaParser, SystemProfileProvider,
};

pub struct DbServices {
    file_classifier: Box<dyn FileTypeClassifier>,
    ext_normalizer: Box<dyn ExtensionNormalizer>,
    system_profiles: Box<dyn SystemProfileProvider>,
    media_parser: Box<dyn MediaParser>,
}

static SERVICES: OnceLock<DbServices> = OnceLock::new();

impl DbServices {
    pub fn new(
        file_classifier: Box<dyn FileTypeClassifier>,
        ext_normalizer: Box<dyn ExtensionNormalizer>,
        system_profiles: Box<dyn SystemProfileProvider>,
        media_parser: Box<dyn MediaParser>,
    ) -> Self {
        Self { file_classifier, ext_normalizer, system_profiles, media_parser }
    }
}

pub fn init(svc: DbServices) {
    SERVICES.set(svc).unwrap_or_else(|_| panic!("DbServices already initialized"));
}

fn services() -> &'static DbServices {
    SERVICES.get().expect("DbServices not initialized. Call db::services::init() early in startup.")
}

pub fn file_classifier() -> &'static dyn FileTypeClassifier {
    services().file_classifier.as_ref()
}

pub fn ext_normalizer() -> &'static dyn ExtensionNormalizer {
    services().ext_normalizer.as_ref()
}

pub fn system_profiles() -> &'static dyn SystemProfileProvider {
    services().system_profiles.as_ref()
}

pub fn media_parser() -> &'static dyn MediaParser {
    services().media_parser.as_ref()
}
