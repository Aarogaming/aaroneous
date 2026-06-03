use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::federation::specialist::SpecialistRegistry;

/// SpecialistRegistryAdapter - adapts federation/specialist::SpecialistRegistry to SubRegistry trait
pub struct SpecialistRegistryAdapter {
    inner: SpecialistRegistry,
}

impl SpecialistRegistryAdapter {
    pub fn new(registry: SpecialistRegistry) -> Self {
        Self { inner: registry }
    }
    
    /// Set a specialist in the underlying registry
    pub fn set_specialist(&mut self, id: String, name: String, capabilities: Vec<String>) {
        self.inner.set_specialist(id.clone(), name.clone(), capabilities.clone());
    }
    
    /// Get a specialist from the underlying registry
    pub fn get_specialist(&self, id: &str) -> Option<crate::federation::specialist::SpecialistInfo> {
        self.inner.get_specialist(id)
    }
}

impl SubRegistry for SpecialistRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by specialist ID
        if let Some(specialist_info) = self.inner.get_specialist(id) {
            return Some(EntityInfo {
                id: specialist_info.id.clone(),
                name: Some(specialist_info.name),
                version: None,
                health: EntryHealth::Healthy,
                last_seen: 0,
            });
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        // List all specialists in the registry
        // Note: SpecialistRegistry.all() returns Arc<dyn Specialist> which has async methods
        // We extract the ID from each specialist
        self.inner.all().iter().map(|specialist| {
            let id = specialist.id();
            EntityInfo {
                id: id.name().to_string(),
                name: Some(id.sovereign_name().to_string()),
                version: None,
                health: EntryHealth::Healthy,
                last_seen: 0,
            }
        }).collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::Specialist
    }
}
