use serde_json::Value;

use crate::core::config::{
    GroupSettings, ProviderConfigDefaults, ProviderConfigDescriptor,
};

use super::{cloned_nested, set_nested_if_missing};

pub(crate) fn descriptor() -> ProviderConfigDescriptor {
    ProviderConfigDescriptor::new("telegram", defaults, Some(apply_legacy_json))
}

fn defaults(_general: &GroupSettings) -> ProviderConfigDefaults {
    ProviderConfigDefaults::TELEGRAM
}

fn apply_legacy_json(root: &mut Value) {
    if let Some(value) = cloned_nested(root, &["telegram", "file_limit_mb"]) {
        set_nested_if_missing(
            root,
            &["providers", "telegram", "limits", "file_limit_mb"],
            value,
        );
    }
}
