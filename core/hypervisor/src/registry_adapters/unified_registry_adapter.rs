use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::unified_registry::Registry;
use std::time::{SystemTime, UNIX_EPOCH};

/// UnifiedRegistryAdapter - adapts unified_registry::Registry to SubRegistry trait
pub struct UnifiedRegistryAdapter<K> {
    inner: Registry<K>,
}

impl<K: Clone + Send + Sync + 'static> UnifiedRegistryAdapter<K> {
    pub fn new(registry: Registry<K>) -> Self {
        Self { inner: registry }
    }
}

impl<K: Clone + Send + Sync + 'static> SubRegistry for UnifiedRegistryAdapter<K> {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        self.inner.get(id).map(|entry| EntityInfo {
            id: entry.id.clone(),
            name: Some(entry.id.clone()),
            version: Some(entry.meta.version.clone()),
            health: match entry.meta.health {
                crate::unified_registry::EntryHealth::Healthy => EntryHealth::Healthy,
                crate::unified_registry::EntryHealth::Degraded => EntryHealth::Degraded,
                crate::unified_registry::EntryHealth::Failed => EntryHealth::Failed,
                crate::unified_registry::EntryHealth::Unknown => EntryHealth::Unknown,
            },
            last_seen: entry.meta.last_seen,
        })
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        self.inner
            .list()
            .into_iter()
            .map(|entry| EntityInfo {
                id: entry.id.clone(),
                name: Some(entry.id.clone()),
                version: Some(entry.meta.version.clone()),
                health: match entry.meta.health {
                    crate::unified_registry::EntryHealth::Healthy => EntryHealth::Healthy,
                    crate::unified_registry::EntryHealth::Degraded => EntryHealth::Degraded,
                    crate::unified_registry::EntryHealth::Failed => EntryHealth::Failed,
                    crate::unified_registry::EntryHealth::Unknown => EntryHealth::Unknown,
                },
                last_seen: entry.meta.last_seen,
            })
            .collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        // Unified registry is in-memory, no special sync needed
        // Just validate that registry is accessible
        Ok(())
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::Unified
    }
}
