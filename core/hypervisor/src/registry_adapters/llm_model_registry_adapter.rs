use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::llm::model_registry::ModelRegistry;

/// LLMModelRegistryAdapter - adapts llm/model_registry::ModelRegistry to SubRegistry trait
pub struct LLMModelRegistryAdapter {
    inner: ModelRegistry,
}

impl LLMModelRegistryAdapter {
    pub fn new(registry: ModelRegistry) -> Self {
        Self { inner: registry }
    }
    
    /// Set a model in the underlying registry
    pub fn set_model(&mut self, name: String, path: String, size_bytes: u64, model_type: crate::llm::model_registry::ModelType) {
        self.inner.set_model(name.clone(), path.clone(), size_bytes, model_type);
    }
    
    /// Get a model from the underlying registry
    pub fn get_model(&self, name: &str) -> Option<crate::llm::model_registry::ModelInfo> {
        self.inner.get_model(name)
    }
}

impl SubRegistry for LLMModelRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by model name
        if let Some(model_info) = self.inner.get_by_name(id) {
            return Some(EntityInfo {
                id: model_info.name.clone(),
                name: Some(model_info.name.clone()),
                version: None,
                health: EntryHealth::Healthy,
                last_seen: 0,
            });
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        self.inner.all_models().iter().map(|model_info| EntityInfo {
            id: model_info.name.clone(),
            name: Some(model_info.name.clone()),
            version: None,
            health: EntryHealth::Healthy,
            last_seen: 0,
        }).collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::LLMModel
    }
}
