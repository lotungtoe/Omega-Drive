use std::collections::HashMap;
use std::sync::Arc;
use omega_drive_gateway::provider::remote_object::RemoteObjectGateway;

pub struct RemoteObjectRegistry {
    gateways: HashMap<String, Arc<dyn RemoteObjectGateway>>,
}

impl RemoteObjectRegistry {
    pub fn new() -> Self { Self { gateways: HashMap::new() } }
    pub fn register(&mut self, gateway: Arc<dyn RemoteObjectGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn RemoteObjectGateway>> {
        self.gateways.get(provider_id).cloned()
    }
    pub fn list(&self) -> Vec<Arc<dyn RemoteObjectGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for RemoteObjectRegistry {
    fn default() -> Self { Self::new() }
}
