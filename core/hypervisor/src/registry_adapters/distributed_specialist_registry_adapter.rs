use crate::registry::{SubRegistry, EntityInfo, EntryHealth, WorkspaceContext, RegistryType};
use crate::federation::multi_hive::distributed_registry::DistributedSpecialistRegistry;

/// DistributedSpecialistRegistryAdapter - adapts federation/multi_hive/distributed_registry::DistributedSpecialistRegistry to SubRegistry trait
pub struct DistributedSpecialistRegistryAdapter {
    inner: DistributedSpecialistRegistry,
}

impl DistributedSpecialistRegistryAdapter {
    pub fn new(registry: DistributedSpecialistRegistry) -> Self {
        Self { inner: registry }
    }
    
    /// Initialize the distributed specialist registry
    pub fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// Synchronize state with workspace context
    pub fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        Ok(())
    }
}

impl SubRegistry for DistributedSpecialistRegistryAdapter {
    fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        self.initialize()
    }
    
    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // Query by specialist ID or node address
        let specialists = self.inner.specialists();
        for (key, remote_specialist) in specialists.iter() {
            let key_str = format!("{:?}::{}", key.0, key.1);
            if key_str == id || remote_specialist.address == id {
                return Some(EntityInfo {
                    id: key_str.clone(),
                    name: Some(remote_specialist.address.clone()),
                    version: None,
                    health: if remote_specialist.available { EntryHealth::Healthy } else { EntryHealth::Degraded },
                    last_seen: remote_specialist.last_seen_ms,
                });
            }
        }
        None
    }
    
    fn list_entities(&self) -> Vec<EntityInfo> {
        // List all distributed specialists
        let specialists = self.inner.specialists();
        specialists.iter().map(|(key, remote_specialist)| {
            let key_str = format!("{:?}::{}", key.0, key.1);
            EntityInfo {
                id: key_str,
                name: Some(remote_specialist.address.clone()),
                version: None,
                health: if remote_specialist.available { EntryHealth::Healthy } else { EntryHealth::Degraded },
                last_seen: remote_specialist.last_seen_ms,
            }
        }).collect()
    }
    
    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        self.synchronize_state(&_ctx)
    }
    
    fn registry_type(&self) -> RegistryType {
        RegistryType::DistributedSpecialist
    }
}
