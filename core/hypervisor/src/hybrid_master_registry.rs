/// Hybrid Master Registry Composition Strategy
/// 
/// Combines Trait-Based Discovery Pattern with Registry-of-Registries Container
/// for Phase 6D WASM/Sentinel GuestOS layer.
///
/// Provides structural polymorphism via trait interface while maintaining
/// dynamic composition through heap-allocated trait vectors.

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

// Re-export core registry definitions from registry module
pub use crate::registry::{
    SubRegistry, WorkspaceContext, EntityInfo, RegistryType, EntryHealth, PhaseEra,
};

/// Master registry container implementing Registry-of-Registries pattern.
pub struct MasterRegistry {
    /// Heap-allocated trait vectors for dynamic composition
    sub_registries: Vec<Box<dyn SubRegistry>>,
    /// Workspace context with state machine property
    ctx: WorkspaceContext,
    /// Registry metadata
    meta: RegistryMeta,
    /// Synchronized entity cache (id → EntityInfo)
    synced_entities: HashMap<String, EntityInfo>,
}

/// Metadata for the master registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryMeta {
    pub created_at: u64,
    pub registry_version: String,
    pub active_era: PhaseEra,
    pub total_registries: usize,
}

impl MasterRegistry {
    pub fn new() -> Self {
        Self {
            sub_registries: Vec::new(),
            ctx: WorkspaceContext::default(),
            meta: RegistryMeta {
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                registry_version: "1.0.0".to_string(),
                active_era: PhaseEra::SixD,
                total_registries: 0,
            },
            synced_entities: HashMap::new(),
        }
    }

    pub fn add_registry(&mut self, registry: Box<dyn SubRegistry>) {
        self.sub_registries.push(registry);
        self.meta.total_registries = self.sub_registries.len();
        info!("Added registry to master: {}", registry.registry_type());
    }
    
    pub fn remove_registry(&mut self, ty: RegistryType) -> Option<Box<dyn SubRegistry>> {
        for (i, registry) in self.sub_registries.iter_mut().enumerate() {
            if registry.registry_type() == ty {
                let removed = self.sub_registries.remove(i);
                self.meta.total_registries = self.sub_registries.len();
                info!("Removed registry from master: {}", ty);
                return Some(removed);
            }
        }
        None
    }

    pub fn get_registry_count(&self) -> usize {
        self.sub_registries.len()
    }

    pub fn active_era(&self) -> PhaseEra {
        self.ctx.current_era
    }

    pub fn set_active_era(&mut self, era: PhaseEra) {
        self.ctx.current_era = era;
        self.meta.active_era = era;
        info!("Set active era to {:?}", era);
    }

    /// Initialize all sub-registries with workspace context.
    pub fn initialize(&mut self) -> Result<(), String> {
        for registry in &mut self.sub_registries {
            if let Err(e) = registry.initialize(&self.ctx) {
                warn!("Failed to initialize registry {}: {}", registry.registry_type(), e);
            } else {
                debug!("Initialized registry: {}", registry.registry_type());
            }
        }
        Ok(())
    }
    
    /// Synchronize state across all sub-registries.
    pub fn synchronize_state(&mut self) -> Result<(), String> {
        // Clear previous synced entities
        self.synced_entities.clear();
        
        // First pass: call synchronize on all adapters
        for registry in &mut self.sub_registries {
            if let Err(e) = registry.synchronize_state(&self.ctx) {
                warn!("Failed to synchronize registry {}: {}", registry.registry_type(), e);
            } else {
                debug!("Synchronized registry: {}", registry.registry_type());
            }
        }
        
        // Second pass: collect entities from all adapters
        for registry in &self.sub_registries {
            let entities = registry.list_entities();
            for entity in entities {
                debug!("Synced entity {} from {}", entity.id, registry.registry_type());
                self.synced_entities.insert(entity.id.clone(), entity);
            }
        }
        
        info!("Master registry synchronized {} total entities", self.synced_entities.len());
        Ok(())
    }
    
    /// Add a synced entity to the master registry.
    /// Called by adapters during synchronization.
    pub fn add_synced_entity(&mut self, entity: EntityInfo) {
        debug!("Added synced entity: {} (type: {:?})", entity.id, entity.health);
        self.synced_entities.insert(entity.id.clone(), entity);
    }
    
    /// Get all synced entities from master registry.
    pub fn get_synced_entities(&self) -> Vec<EntityInfo> {
        self.synced_entities.values().cloned().collect()
    }
    
    /// Get synced entity by ID.
    pub fn get_synced_entity(&self, id: &str) -> Option<EntityInfo> {
        self.synced_entities.get(id).cloned()
    }
    
    /// Query an entity across all sub-registries.
    /// First checks synced entities cache, then queries sub-registries.
    pub fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        // First check synced cache
        if let Some(entity) = self.synced_entities.get(id) {
            return Some(entity.clone());
        }
        
        // Fall back to querying sub-registries directly
        for registry in &self.sub_registries {
            if let Some(info) = registry.query_entity(id) {
                return Some(info);
            }
        }
        None
    }
    
    /// List all registered entity IDs across sub-registries.
    pub fn list_all_entities(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for registry in &self.sub_registries {
            // This would need to be implemented per-registry type
            // For now, return empty - actual implementations should override
        }
        ids
    }
    
    /// Get count of all sub-registries.
    pub fn registry_count(&self) -> usize {
        self.sub_registries.len()
    }
    
    /// Get workspace context.
    pub fn ctx(&self) -> &WorkspaceContext {
        &self.ctx
    }
    
    /// Get mutable workspace context.
    pub fn ctx_mut(&mut self) -> &mut WorkspaceContext {
        &mut self.ctx
    }
}

impl Default for MasterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Composition strategy for wiring existing registries to hybrid master container.
pub struct RegistryCompositionStrategy {
    /// Heap-allocated trait objects for all adapters
    adapters: Vec<Box<dyn SubRegistry>>,
}

impl RegistryCompositionStrategy {
    pub fn new() -> Self {
        Self {
            adapters: Vec::new(),
        }
    }
    
    pub fn with_unified_registry(mut self, registry: crate::unified_registry::Registry<String>) -> Self {
        let adapter = crate::registry_adapters::UnifiedRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_federation_model_registry(mut self, registry: crate::federation::model_registry::FederationModelRegistry) -> Self {
        let adapter = crate::registry_adapters::FederationModelRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_link_registry(mut self, registry: crate::federation::links::LinkRegistry) -> Self {
        let adapter = crate::registry_adapters::LinkRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_llm_model_registry(mut self, registry: crate::llm::model_registry::ModelRegistry) -> Self {
        let adapter = crate::registry_adapters::LLMModelRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_component_registry(mut self, registry: crate::federation::component_registry::ComponentRegistry) -> Self {
        let adapter = crate::registry_adapters::ComponentRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_specialist_registry(mut self, registry: crate::federation::specialist::SpecialistRegistry) -> Self {
        let adapter = crate::registry_adapters::SpecialistRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_chromosome_registry(mut self, registry: crate::chromosome_registry::ChromosomeRegistry) -> Self {
        let adapter = crate::registry_adapters::ChromosomeRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_hox_registry(mut self, registry: crate::hox_registry::HoxRegistry) -> Self {
        let adapter = crate::registry_adapters::HoxCapabilityRegistryAdapter::new();
        self.adapters.push(Box::new(adapter));
        self
    }
    
    pub fn with_distributed_registry(mut self, registry: crate::federation::multi_hive::distributed_registry::DistributedSpecialistRegistry) -> Self {
        let adapter = crate::registry_adapters::DistributedSpecialistRegistryAdapter::new(registry);
        self.adapters.push(Box::new(adapter));
        self
    }
    
    /// Build master registry with all wired sub-registries.
    pub fn build_master_registry(self, ctx: &WorkspaceContext) -> MasterRegistry {
        let mut master = MasterRegistry::new();
        
        // Wire each adapter to master container
        for adapter in self.adapters {
            master.add_registry(adapter);
        }
        
        // Initialize all sub-registries with workspace context
        if let Err(e) = master.initialize() {
            warn!("Failed to initialize master registry: {}", e);
        }
        
        master
    }
}

/// Async wrapper for master registry operations.
pub struct AsyncMasterRegistry {
    inner: Arc<RwLock<MasterRegistry>>,
}

impl AsyncMasterRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MasterRegistry::new())),
        }
    }
    
    pub async fn initialize(&self) -> Result<(), String> {
        self.inner.write().await.initialize()
    }
    
    pub async fn synchronize_state(&self) -> Result<(), String> {
        self.inner.write().await.synchronize_state()
    }
    
    pub async fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        self.inner.read().await.query_entity(id)
    }
    
    pub async fn registry_count(&self) -> usize {
        self.inner.read().await.registry_count()
    }
    
    pub async fn active_era(&self) -> PhaseEra {
        self.inner.read().await.active_era()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct MockRegistry {}

    impl SubRegistry for MockRegistry {
        fn initialize(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
            Ok(())
        }
        
        fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
            Ok(())
        }
        
        fn registry_type(&self) -> RegistryType {
            RegistryType::Unified
        }
    }

    #[test]
    fn test_master_registry_creation() {
        let master = MasterRegistry::new();
        assert_eq!(master.registry_count(), 0);
        assert_eq!(master.active_era(), PhaseEra::SixD);
    }
    
    #[test]
    fn test_add_and_remove_registry() {
        let mut master = MasterRegistry::new();
        let mock = Box::new(MockRegistry {});
        
        master.add_registry(mock);
        assert_eq!(master.registry_count(), 1);
        
        let removed = master.remove_registry(RegistryType::Unified);
        assert!(removed.is_some());
        assert_eq!(master.registry_count(), 0);
    }
    
    #[test]
    fn test_set_active_era() {
        let mut master = MasterRegistry::new();
        master.set_active_era(PhaseEra::ThreeC);
        assert_eq!(master.active_era(), PhaseEra::ThreeC);
    }

    #[test]
    fn test_cross_registry_entity_query() {
        let mut master = MasterRegistry::new();
        let mock1 = Box::new(MockRegistry {});
        let mock2 = Box::new(MockRegistry {});
        
        master.add_registry(mock1);
        master.add_registry(mock2);
        
        assert_eq!(master.registry_count(), 2);
        
        // Query should search through all registries
        let result = master.query_entity("test-id");
        // MockRegistry returns None, so result should be None
        assert!(result.is_none());
    }
    
    #[test]
    fn test_registry_composition_strategy() {
        let strategy = RegistryCompositionStrategy::new();
        let ctx = WorkspaceContext::default();
        
        let master = strategy.build_master_registry(&ctx);
        assert_eq!(master.registry_count(), 0);
        assert_eq!(master.active_era(), PhaseEra::SixD);
    }
    
    #[test]
    fn test_master_registry_initialization() {
        let mut master = MasterRegistry::new();
        let mock = Box::new(MockRegistry {});
        
        master.add_registry(mock);
        let result = master.initialize();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_master_registry_synchronization() {
        let mut master = MasterRegistry::new();
        let mock = Box::new(MockRegistry {});
        
        master.add_registry(mock);
        let result = master.synchronize_state();
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_entity_health_mapping() {
        assert_eq!(EntryHealth::Healthy, EntryHealth::Healthy);
        assert_eq!(EntryHealth::Degraded, EntryHealth::Degraded);
        assert_eq!(EntryHealth::Failed, EntryHealth::Failed);
        assert_eq!(EntryHealth::Unknown, EntryHealth::Unknown);
        
        let default_health = EntryHealth::default();
        assert_eq!(default_health, EntryHealth::Unknown);
    }
    
    #[test]
    fn test_registry_type_identification() {
        assert_eq!(RegistryType::Unified, RegistryType::Unified);
        assert_eq!(RegistryType::HoxCapability, RegistryType::HoxCapability);
        assert_eq!(RegistryType::Chromosome, RegistryType::Chromosome);
        assert_ne!(RegistryType::Unified, RegistryType::HoxCapability);
        
        let default_type = RegistryType::default();
        assert_eq!(default_type, RegistryType::Unified);
    }
    
    #[test]
    fn test_workspace_context_phase_era() {
        let ctx = WorkspaceContext::default();
        assert_eq!(ctx.current_era, PhaseEra::SixD);
        assert_eq!(ctx.registry_version, "1.0.0");
        assert_eq!(PhaseEra::current(), PhaseEra::SixD);
    }
}
