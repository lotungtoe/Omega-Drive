use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::AsyncReadExt;

use crate::blake3;
use crate::upload::upload_plan::HashAlgorithm;

pub const BLAKE3_PREFIX: &str = "";

pub async fn calculate_file_blake3<P: AsRef<Path>>(path: P) -> Result<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .context("Unable to open file for BLAKE3 hashing")?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 65536];

    loop {
        let count = file
            .read(&mut buffer)
            .await
            .context("Failed to read file while BLAKE3 hashing")?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

pub fn calculate_bytes_blake3(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

pub fn verify_bytes_integrity(data: &[u8], expected_hash: &str) -> bool {
    let actual = calculate_bytes_blake3(data);
    actual == expected_hash
}

pub async fn verify_file_integrity<P: AsRef<Path>>(path: P, expected_hash: &str) -> Result<bool> {
    let actual_hash = calculate_file_blake3(path).await?;
    Ok(actual_hash == expected_hash)
}

pub async fn calculate_file_hash<P: AsRef<Path>>(
    path: P,
    _algorithm: HashAlgorithm,
) -> Result<String> {
    calculate_file_blake3(path).await
}

pub fn calculate_bytes_hash(data: &[u8], _algorithm: HashAlgorithm) -> String {
    calculate_bytes_blake3(data)
}

pub struct EngineIntegrityService;

#[async_trait::async_trait]
impl crate::core::engine_context::IntegrityService for EngineIntegrityService {
    fn calculate_bytes_blake3(&self, data: &[u8]) -> String {
        calculate_bytes_blake3(data)
    }

    fn verify_bytes_integrity(&self, data: &[u8], expected_hash: &str) -> bool {
        verify_bytes_integrity(data, expected_hash)
    }

    async fn verify_file_integrity(
        &self,
        path: &Path,
        expected_hash: &str,
    ) -> Result<bool, String> {
        verify_file_integrity(path, expected_hash)
            .await
            .map_err(|e| e.to_string())
    }
}
