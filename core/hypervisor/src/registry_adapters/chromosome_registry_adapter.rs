use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::chromosome_registry::ChromosomeRegistry;

/// ChromosomeRegistryAdapter - adapts chromosome_registry::ChromosomeRegistry to SubRegistry trait
pub struct ChromosomeRegistryAdapter {
    inner: ChromosomeRegistry,
}

impl ChromosomeRegistryAdapter {
    pub fn new(registry: ChromosomeRegistry) -> Self {
        Self { inner: registry }
    }
    
    /// Initialize the chromosome registry
    pub fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// Synchronize state with workspace context
    pub fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
}

impl SubRegistry for ChromosomeRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        self.initialize()
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by agent ID or profile key
        if let Some(chromosome) = self.inner.profiles.get(id) {
            return Some(EntityInfo {
                id: chromosome.agent_id.clone(),
                name: Some(chromosome.base_model_path.clone()),
                version: None,
                health: EntryHealth::Healthy,
                last_seen: 0,
            });
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        // List all chromosome profiles
        self.inner.profiles.iter().map(|(_, chromosome)| EntityInfo {
            id: chromosome.agent_id.clone(),
            name: Some(chromosome.base_model_path.clone()),
            version: None,
            health: EntryHealth::Healthy,
            last_seen: 0,
        }).collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        self.synchronize_state(&_ctx)
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::Chromosome
    }
}
