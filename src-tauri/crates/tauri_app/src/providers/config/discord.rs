use serde_json::Value;

use crate::core::config::{
    GroupSettings, ProviderConfigDefaults, ProviderConfigDescriptor,
};

use super::{cloned_nested, set_nested_if_missing};

pub(crate) fn descriptor() -> ProviderConfigDescriptor {
    ProviderConfigDescriptor::new("discord", defaults, Some(apply_legacy_json))
}

fn defaults(_general: &GroupSettings) -> ProviderConfigDefaults {
    ProviderConfigDefaults::DISCORD
}

fn apply_legacy_json(root: &mut Value) {
    if let Some(value) = cloned_nested(root, &["upload", "discord_send_retries"]) {
        set_nested_if_missing(
            root,
            &["providers", "discord", "retry", "send_retries"],
            value,
        );
    }

    if let Some(value) = cloned_nested(root, &["upload", "discord_retry_base_delay_s"]) {
        set_nested_if_missing(
            root,
            &["providers", "discord", "retry", "retry_base_delay_s"],
            value,
        );
    }

    if let Some(value) = cloned_nested(root, &["upload", "discord_hard_limit_mb"]) {
        set_nested_if_missing(
            root,
            &["providers", "discord", "limits", "hard_limit_mb"],
            value.clone(),
        );
        set_nested_if_missing(
            root,
            &["providers", "discord", "limits", "file_limit_mb"],
            value,
        );
    }
}
