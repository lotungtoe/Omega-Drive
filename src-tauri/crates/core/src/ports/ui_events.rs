use serde::Serialize;
use serde_json::Value;

pub use omega_drive_gateway::provider::ui_events::UiEventEmitter;

#[derive(Debug, Default)]
pub struct NoopUiEventEmitter;

impl UiEventEmitter for NoopUiEventEmitter {
    fn emit_value(&self, _event_name: &str, _payload: Value) {}
}

pub fn emit_serialized(emitter: &dyn UiEventEmitter, event_name: &str, payload: &impl Serialize) {
    if let Ok(value) = serde_json::to_value(payload) {
        emitter.emit_value(event_name, value);
    }
}
