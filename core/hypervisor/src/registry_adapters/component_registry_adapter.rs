use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::federation::component_registry::ComponentRegistry;

/// ComponentRegistryAdapter - adapts federation/component_registry::ComponentRegistry to SubRegistry trait
pub struct ComponentRegistryAdapter {
    inner: ComponentRegistry,
}

impl ComponentRegistryAdapter {
    pub fn new(registry: ComponentRegistry) -> Self {
        Self { inner: registry }
    }
    
    /// Set a component in the underlying registry
    pub fn set_component(&mut self, name: String, version: String, path: String) {
        self.inner.set_component(name.clone(), version.clone(), path.clone());
    }
    
    /// Get a component from the underlying registry
    pub fn get_component(&self, name: &str) -> Option<crate::federation::component_registry::RegistryComponent> {
        self.inner.get_component(name)
    }
    
    /// Set latest version for a component
    pub fn set_latest_version(&mut self, name: String, version: String) {
        self.inner.set_latest_version(name.clone(), version.clone());
    }
}

impl SubRegistry for ComponentRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by component name
        if let Some(component) = self.inner.get_component(id) {
            return Some(EntityInfo {
                id: component.name.clone(),
                name: Some(component.name),
                version: Some(component.latest.version.clone()),
                health: EntryHealth::Healthy,
                last_seen: 0,
            });
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        // ComponentRegistry is async-based, so we return empty for now
        // In a production scenario, this would need to be refactored to support async operations
        Vec::new()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::Component
    }
}
