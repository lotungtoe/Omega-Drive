use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadPlan {
    pub original_upload: OriginalUploadPlan,
    pub derivatives: DerivativesPlan,
    pub advanced: Option<AdvancedLimits>,
    #[serde(default)]
    pub audio_attachments: Vec<String>,
}

impl Default for UploadPlan {
    fn default() -> Self {
        Self {
            original_upload: OriginalUploadPlan {
                enabled: true,
                strategy: UploadStrategy::Fast,
                providers: vec![],
                priority_mode: PriorityMode::Foreground,
            },
            derivatives: DerivativesPlan::default(),
            advanced: None,
            audio_attachments: Vec::new(),
        }
    }
}

impl Default for OriginalUploadPlan {
    fn default() -> Self {
        Self {
            enabled: true,
            strategy: UploadStrategy::Fast,
            providers: vec![],
            priority_mode: PriorityMode::Foreground,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedLimits {
    pub hard_limit_mb: Option<u32>,
    pub file_limit_mb: Option<u32>,
    pub max_total_upload_mb: Option<u64>,
    pub concurrency_threads: Option<u32>,
    pub retry_count: Option<u32>,
    pub retry_delay_s: Option<u32>,
    pub chunk_size_mb: Option<u32>,
    pub bandwidth_limit_kbps: Option<u32>,
    pub webhook_url: Option<String>,
    pub discord_batch_size: Option<u32>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalUploadPlan {
    pub enabled: bool,
    pub strategy: UploadStrategy,
    pub providers: Vec<ProviderType>,
    pub priority_mode: PriorityMode,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UploadStrategy {
    #[default]
    Fast, // Distributed chunks
    Safe, // Mirrored chunks
    None, // Direct upload
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Discord,
    Telegram,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DerivativesPlan {
    pub web_preview: Option<WebPreviewPlan>,
    pub zip_package: Option<ZipPackagePlan>,
    pub hashes: Option<HashPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebPreviewPlan {
    pub enabled: bool,
    pub codec: PreviewCodec,
    pub bitrate_mode: Option<BitrateMode>,
    pub bitrate_mbps: Option<u32>,
    pub resolution_mode: Option<ResolutionMode>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ZipPackagePlan {
    pub enabled: bool,
    pub zip_level: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashPlan {
    pub enabled: bool,
    pub algorithms: Vec<HashAlgorithm>,
}

impl Default for HashPlan {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithms: vec![HashAlgorithm::Blake3],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PriorityMode {
    #[default]
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PreviewCodec {
    #[default]
    Auto,
    H264,
    Av1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BitrateMode {
    Auto,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolutionMode {
    Auto,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    Blake3,
}

impl UploadPlan {
    pub fn apply_defaults(&mut self) {
        let advanced = self.advanced.get_or_insert_with(AdvancedLimits::default);
        if advanced.discord_batch_size.unwrap_or(0) == 0 {
            advanced.discord_batch_size =
                Some(crate::core::config::DEFAULT_DISCORD_PARTS_PER_MESSAGE as u32);
        }
    }
}

pub fn balanced_upload_plan() -> UploadPlan {
    let mut plan = UploadPlan {
        original_upload: OriginalUploadPlan {
            enabled: true,
            strategy: UploadStrategy::Fast,
            providers: vec![ProviderType::Discord],
            priority_mode: PriorityMode::Foreground,
        },
        derivatives: DerivativesPlan {
            web_preview: None,
            zip_package: None,
            hashes: Some(HashPlan::default()),
        },
        advanced: None,
        audio_attachments: Vec::new(),
    };
    plan.apply_defaults();
    plan
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProfile {
    pub id: Option<i64>,
    pub name: String,
    pub plan: UploadPlan,
}


#[derive(Debug, Clone)]
pub struct UploadSourceInfo {
    pub total_bytes: u64,
    pub filename: String,
    pub file_path_str: String,
}

#[derive(Debug, Clone)]
pub struct ProviderExecutionSettings {
    pub parallel_sends: usize,
    pub chunk_size: u64,
    pub batch_multiplier: usize,
    pub total_parts: usize,
}

#[derive(Debug, Clone)]
pub struct PreparedUploadPlan {
    pub total_bytes: u64,
    pub filename: String,
    pub file_path_str: String,
    pub is_video: bool,
    pub is_audio: bool,
    pub is_image: bool,
    pub needs_remux: bool,
    pub base_chunk_size: u64,
    pub total_base_parts: usize,
    pub total_parts: usize,
    pub parallel_sends: usize,
    pub per_provider_bytes: std::collections::HashMap<String, u64>,
    pub strategy: UploadStrategy,
    pub providers: Vec<ProviderType>,
    pub provider_settings: std::collections::HashMap<String, ProviderExecutionSettings>,
}
