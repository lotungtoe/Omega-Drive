use std::collections::HashMap;
use std::sync::Arc;
use omega_drive_gateway::provider::storage::StorageProvider;

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn StorageProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self { Self { providers: HashMap::new() } }
    pub fn register(&mut self, provider: Arc<dyn StorageProvider>) {
        self.providers.insert(provider.metadata().id.clone(), provider);
    }
    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn StorageProvider>> {
        self.providers.get(provider_id).cloned()
    }
    pub fn list(&self) -> Vec<Arc<dyn StorageProvider>> {
        self.providers.values().cloned().collect()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self { Self::new() }
}
