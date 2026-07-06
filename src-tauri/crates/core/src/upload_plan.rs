pub use omega_drive_gateway::upload::upload_plan::{
    AdvancedLimits, BitrateMode, DerivativesPlan, HashAlgorithm, HashPlan, OriginalUploadPlan,
    PreviewCodec, PriorityMode, ProviderType, ResolutionMode, UploadPlan, UploadProfile,
    UploadStrategy, WebPreviewPlan, ZipPackagePlan,
};

fn apply_upload_defaults(plan: &mut UploadPlan) {
    let advanced = plan.advanced.get_or_insert_with(AdvancedLimits::default);
    if advanced.discord_batch_size.unwrap_or(0) == 0 {
        advanced.discord_batch_size =
            Some(omega_drive_gateway::core::config::DEFAULT_DISCORD_PARTS_PER_MESSAGE as u32);
    }
}

fn build_upload_plan(
    strategy: UploadStrategy,
    providers: Vec<ProviderType>,
    priority: PriorityMode,
    web_preview_codec: Option<PreviewCodec>,
    zip_level: Option<u32>,
) -> UploadPlan {
    let mut plan = UploadPlan {
        original_upload: OriginalUploadPlan {
            enabled: true,
            strategy,
            providers,
            priority_mode: priority,
        },
        derivatives: DerivativesPlan {
            web_preview: web_preview_codec.map(|codec| WebPreviewPlan {
                enabled: true,
                codec,
                bitrate_mode: Some(BitrateMode::Auto),
                bitrate_mbps: None,
                resolution_mode: Some(ResolutionMode::Auto),
                max_width: None,
                max_height: None,
            }),
            zip_package: zip_level.map(|level| ZipPackagePlan {
                enabled: true,
                zip_level: Some(level),
            }),
            hashes: Some(HashPlan::default()),
        },
        advanced: None,
        audio_attachments: vec![],
    };
    apply_upload_defaults(&mut plan);
    plan
}

pub fn default_system_profiles() -> Vec<UploadProfile> {
    vec![
        UploadProfile { id: None, name: "Fast Upload".to_string(), plan: build_upload_plan(UploadStrategy::Fast, vec![ProviderType::Discord, ProviderType::Telegram], PriorityMode::Foreground, None, None) },
        UploadProfile { id: None, name: "Balanced".to_string(), plan: build_upload_plan(UploadStrategy::Fast, vec![ProviderType::Discord], PriorityMode::Foreground, Some(PreviewCodec::Auto), None) },
        UploadProfile { id: None, name: "Archive (Safe Mode)".to_string(), plan: build_upload_plan(UploadStrategy::Safe, vec![ProviderType::Discord, ProviderType::Telegram], PriorityMode::Background, None, Some(6)) },
    ]
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
        audio_attachments: vec![],
    };
    apply_upload_defaults(&mut plan);
    plan
}

pub fn validate_upload_plan(plan: &UploadPlan) -> Result<(), String> {
    if !plan.original_upload.enabled {
        return Err("Original upload must be enabled.".to_string());
    }

    if let Some(web) = &plan.derivatives.web_preview {
        if web.enabled {
            if let Some(BitrateMode::Custom) = web.bitrate_mode {
                if web.bitrate_mbps.unwrap_or(0) == 0 {
                    return Err("Custom preview bitrate must be > 0.".to_string());
                }
            }
            if let Some(ResolutionMode::Custom) = web.resolution_mode {
                if web.max_width.unwrap_or(0) == 0 && web.max_height.unwrap_or(0) == 0 {
                    return Err("Custom preview resolution requires max width or height.".to_string());
                }
            }
        }
    }

    if let Some(hashes) = &plan.derivatives.hashes {
        if hashes.enabled && hashes.algorithms.is_empty() {
            return Err("Hash algorithms must not be empty when enabled.".to_string());
        }
    }

    Ok(())
}
