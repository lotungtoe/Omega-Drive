use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use omega_drive_gateway::provider::ui_events::UiEventEmitter;

pub struct NoopUiEventEmitter;

impl UiEventEmitter for NoopUiEventEmitter {
    fn emit_value(&self, _event_name: &str, _payload: Value) {}
}

pub fn emit_serialized(
    emitter: &dyn UiEventEmitter,
    event_name: &str,
    payload: &impl Serialize,
) {
    if let Ok(value) = serde_json::to_value(payload) {
        emitter.emit_value(event_name, value);
    }
}

pub type SharedUiEventEmitter = Arc<dyn UiEventEmitter>;
