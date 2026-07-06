use serde_json::{Map, Value};

use crate::core::config::ProviderConfigDescriptor;

pub mod discord;
pub mod telegram;

pub fn builtin_provider_config_descriptors() -> Vec<ProviderConfigDescriptor> {
    vec![discord::descriptor(), telegram::descriptor()]
}

pub(super) fn cloned_nested(root: &Value, path: &[&str]) -> Option<Value> {
    let mut current = root;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current.clone())
}

pub(super) fn set_nested_if_missing(root: &mut Value, path: &[&str], value: Value) {
    if path.is_empty() || cloned_nested(root, path).is_some() {
        return;
    }

    let mut current = root;
    for segment in &path[..path.len() - 1] {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }

        let Some(object) = current.as_object_mut() else {
            return;
        };
        current = object
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }

    if !current.is_object() {
        *current = Value::Object(Map::new());
    }

    if let Some(object) = current.as_object_mut() {
        object
            .entry(path[path.len() - 1].to_string())
            .or_insert(value);
    }
}
