use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::federation::model_registry::FederationModelRegistry;

/// FederationModelRegistryAdapter - adapts federation/model_registry::FederationModelRegistry to SubRegistry trait
pub struct FederationModelRegistryAdapter {
    inner: FederationModelRegistry,
}

impl FederationModelRegistryAdapter {
    pub fn new(registry: FederationModelRegistry) -> Self {
        Self { inner: registry }
    }
}

impl SubRegistry for FederationModelRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by model name or path
        let models = self.inner.all();
        for model in models.iter() {
            if model.name == id || model.path.to_string_lossy() == id {
                return Some(EntityInfo {
                    id: model.name.clone(),
                    name: Some(model.name.clone()),
                    version: None,
                    health: EntryHealth::Healthy,
                    last_seen: model.registered_at,
                });
            }
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        self.inner.all().iter().map(|model| EntityInfo {
            id: model.name.clone(),
            name: Some(model.name.clone()),
            version: None,
            health: EntryHealth::Healthy,
            last_seen: model.registered_at,
        }).collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::FederationModel
    }
}
