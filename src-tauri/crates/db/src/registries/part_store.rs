use std::collections::HashMap;
use std::sync::Arc;
use omega_drive_gateway::provider::part_store::PartStoreGateway;

pub struct PartStoreRegistry {
    gateways: HashMap<String, Arc<dyn PartStoreGateway>>,
}

impl PartStoreRegistry {
    pub fn new() -> Self { Self { gateways: HashMap::new() } }
    pub fn register(&mut self, gateway: Arc<dyn PartStoreGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn PartStoreGateway>> {
        self.gateways.get(provider_id).cloned()
    }
    pub fn list(&self) -> Vec<Arc<dyn PartStoreGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for PartStoreRegistry {
    fn default() -> Self { Self::new() }
}
