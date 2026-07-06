use std::collections::HashMap;
use std::sync::Arc;
use omega_drive_gateway::provider::provider_admin::ProviderAdminGateway;

pub struct ProviderAdminRegistry {
    gateways: HashMap<String, Arc<dyn ProviderAdminGateway>>,
}

impl ProviderAdminRegistry {
    pub fn new() -> Self { Self { gateways: HashMap::new() } }
    pub fn register(&mut self, gateway: Arc<dyn ProviderAdminGateway>) {
        self.gateways.insert(gateway.provider_id().to_string(), gateway);
    }
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn ProviderAdminGateway>> {
        self.gateways.get(provider_id).cloned()
    }
    pub fn list(&self) -> Vec<Arc<dyn ProviderAdminGateway>> {
        self.gateways.values().cloned().collect()
    }
}

impl Default for ProviderAdminRegistry {
    fn default() -> Self { Self::new() }
}
