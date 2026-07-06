//! events.rs — Central event bus (Internal Event Bus).
//!
//! Allows app components to communicate in a loosely-coupled (decoupled) way.
//! Future plugins will also "listen" here to perform tasks.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// All possible event types within Omega Drive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OmegaEvent {
    /// A file has started uploading.
    UploadStarted { file_id: i64, filename: String },

    /// Progress update for a running task.
    ProgressUpdate { task_id: String, percentage: f32 },

    /// A file was uploaded successfully.
    FileCreated { file_id: i64, filename: String },

    /// A file was deleted.
    FileDeleted { file_id: i64 },

    /// The 'files' table changed (used to auto-refresh UI).
    FilesTableChanged,

    /// System configuration changed.
    ConfigChanged,

    /// Discord connection status changed.
    DiscordConnectionStatusChanged(bool),

    /// Telegram connection status changed.
    TelegramConnectionStatusChanged(bool),

    /// General notification from the system or a plugin.
    SystemNotification { level: String, message: String },
}

pub struct EventBus {
    tx: broadcast::Sender<OmegaEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self { tx }
    }

    pub fn emit(&self, event: OmegaEvent) {
        let _ = self.tx.send(event);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OmegaEvent> {
        self.tx.subscribe()
    }
}

pub type SharedEventBus = Arc<EventBus>;


