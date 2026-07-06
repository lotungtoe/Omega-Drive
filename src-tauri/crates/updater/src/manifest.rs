use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterManifest {
    pub platforms: HashMap<String, PlatformInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub version: String,
    pub app: BinaryEntry,
    #[serde(default)]
    pub binaries: HashMap<String, BinaryEntry>,
    pub changelog: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryEntry {
    pub url: String,
    pub checksum: String,
}

impl UpdaterManifest {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| format!("Invalid updater manifest JSON: {e}"))
    }

    pub async fn fetch(url: &str) -> Result<Self, String> {
        let resp = reqwest::get(url)
            .await
            .map_err(|e| format!("Failed to fetch manifest from {url}: {e}"))?;
        let text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read manifest body: {e}"))?;
        Self::from_json(&text)
    }

    pub fn current_platform() -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let arch = match arch {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            "x86" => "i686",
            other => other,
        };
        format!("{os}-{arch}")
    }

    pub fn for_current_platform(&self) -> Option<&PlatformInfo> {
        self.platforms.get(&Self::current_platform())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_manifest() {
        let json = r#"{
            "platforms": {
                "windows-x86_64": {
                    "version": "1.0.1",
                    "app": {
                        "url": "https://example.com/omega-drive.msi",
                        "checksum": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                    },
                    "binaries": {
                        "ffmpeg": {
                            "url": "https://example.com/ffmpeg.zip",
                            "checksum": "deadbeef"
                        }
                    },
                    "changelog": "https://example.com/changelog"
                }
            }
        }"#;
        let manifest = UpdaterManifest::from_json(json).unwrap();
        let platform = manifest.platforms.get("windows-x86_64").unwrap();
        assert_eq!(platform.version, "1.0.1");
        assert_eq!(platform.binaries.len(), 1);
        assert_eq!(platform.binaries["ffmpeg"].url, "https://example.com/ffmpeg.zip");
    }

    #[test]
    fn test_missing_binaries_defaults_empty() {
        let json = r#"{
            "platforms": {
                "linux-x86_64": {
                    "version": "1.0.0",
                    "app": { "url": "x", "checksum": "y" },
                    "changelog": "z"
                }
            }
        }"#;
        let manifest = UpdaterManifest::from_json(json).unwrap();
        let platform = manifest.platforms.get("linux-x86_64").unwrap();
        assert!(platform.binaries.is_empty());
    }
}
