pub use omega_drive_gateway::core::events::OmegaEvent;
use std::sync::Arc;
use tokio::sync::broadcast;

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
