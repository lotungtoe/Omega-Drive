use std::path::Path;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait IntegrityService: Send + Sync {
    fn calculate_bytes_blake3(&self, data: &[u8]) -> String;
    fn verify_bytes_integrity(&self, data: &[u8], expected_hash: &str) -> bool;
    async fn verify_file_integrity(
        &self,
        path: &Path,
        expected_hash: &str,
    ) -> Result<bool, String>;
    fn create_hasher(&self) -> Box<dyn crate::engine::Blake3Hasher>;
}

#[async_trait::async_trait]
pub trait ZipService: Send + Sync {
    fn unzip_or_raw(&self, data: Vec<u8>) -> Result<Vec<u8>, String>;
    fn zip_file_to_path(
        &self,
        src: &Path,
        dst: &Path,
        entry_name: &str,
        compress_level: u32,
        aes_password: Option<&str>,
    ) -> Result<(), String>;
}

#[derive(Clone)]
pub struct EngineContext {
    pub integrity: Arc<dyn IntegrityService>,
    pub zip: Arc<dyn ZipService>,
}
