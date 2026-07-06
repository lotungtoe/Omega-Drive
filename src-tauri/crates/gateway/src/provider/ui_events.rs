use serde::Serialize;
use serde_json::Value;

pub trait UiEventEmitter: Send + Sync {
    fn emit_value(&self, event_name: &str, payload: Value);
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


