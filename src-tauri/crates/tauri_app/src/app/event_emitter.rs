use std::sync::Arc;

use tauri::Emitter;
use omega_drive_player::AppEventEmitter;

pub struct TauriEventEmitter(pub Arc<std::sync::Mutex<Option<tauri::AppHandle>>>);

impl AppEventEmitter for TauriEventEmitter {
    fn emit(&self, event: &str, payload: serde_json::Value) {
        if let Some(handle) = self.0.lock().ok().and_then(|g| g.clone()) {
            let _ = handle.emit(event, payload);
        }
    }
}

unsafe impl Send for TauriEventEmitter {}
unsafe impl Sync for TauriEventEmitter {}
