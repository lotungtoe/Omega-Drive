use std::collections::HashMap;
use std::sync::Arc;
use omega_drive_gateway::provider::remote_folder::RemoteFolderGateway;

pub struct RemoteFolderRegistry {
    gateways: HashMap<String, Arc<dyn RemoteFolderGateway>>,
}

impl RemoteFolderRegistry {
    pub fn new() -> Self { Self { gateways: HashMap::new() } }
    pub fn register(&mut self, gateway: Arc<dyn RemoteFolderGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn RemoteFolderGateway>> {
        self.gateways.get(provider_id).cloned()
    }
    pub fn list(&self) -> Vec<Arc<dyn RemoteFolderGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for RemoteFolderRegistry {
    fn default() -> Self { Self::new() }
}
