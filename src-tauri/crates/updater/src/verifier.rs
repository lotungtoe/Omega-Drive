use std::path::Path;

pub fn verify_blake3(file: &Path, expected_hex: &str) -> Result<(), String> {
    let data = std::fs::read(file)
        .map_err(|e| format!("Cannot read {} for checksum: {e}", file.display()))?;
    let hash = blake3::hash(&data);
    let actual_hex = hash.to_hex().to_string();
    if actual_hex.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(format!(
            "BLAKE3 mismatch for {}: expected {expected_hex}, got {actual_hex}",
            file.display()
        ))
    }
}
