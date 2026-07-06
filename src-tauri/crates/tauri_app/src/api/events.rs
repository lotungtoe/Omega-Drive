use serde_json::Value;
use tauri::Emitter;

use crate::core::ports::ui_events::UiEventEmitter;

#[derive(Clone)]
pub struct TauriUiEventEmitter {
    handle: tauri::AppHandle,
}

impl TauriUiEventEmitter {
    pub fn new(handle: tauri::AppHandle) -> Self {
        Self { handle }
    }
}

impl UiEventEmitter for TauriUiEventEmitter {
    fn emit_value(&self, event_name: &str, payload: Value) {
        let _ = self.handle.emit(event_name, payload);
    }
}
