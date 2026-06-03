use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::federation::links::LinkRegistry;

/// LinkRegistryAdapter - adapts federation/links::LinkRegistry to SubRegistry trait
pub struct LinkRegistryAdapter {
    inner: LinkRegistry,
}

impl LinkRegistryAdapter {
    pub fn new(registry: LinkRegistry) -> Self {
        Self { inner: registry }
    }
}

impl SubRegistry for LinkRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by link name or URL
        let links = self.inner.list();
        for link in links.iter() {
            if link.name == id || link.target_url == id {
                return Some(EntityInfo {
                    id: link.name.clone(),
                    name: Some(link.name.clone()),
                    version: None,
                    health: if link.enabled { EntryHealth::Healthy } else { EntryHealth::Unknown },
                    last_seen: 0,
                });
            }
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        self.inner.list().iter().map(|link| EntityInfo {
            id: link.name.clone(),
            name: Some(link.name.clone()),
            version: None,
            health: if link.enabled { EntryHealth::Healthy } else { EntryHealth::Unknown },
            last_seen: 0,
        }).collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::FederationLinks
    }
}
