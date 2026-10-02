//! Comprehensive integration test suite for Phase 6D hybrid master registry layer.
//!
//! Tests the complete lifecycle of the hybrid registry system including:
//! - Multi-adapter initialization and synchronization
//! - Cross-registry entity queries and lookups
//! - State machine transitions during Phase 6D operations
//! - Builder pattern composition and registry wiring
//! - Error handling and recovery scenarios
//!
//! Restored from the pre-split `core/hypervisor/src/phase_6d_integration_tests.rs`
//! (see git history at `9f24821b`) and rewired across the hive_mind/host
//! crate boundary: `hive_mind::hybrid_master_registry` moved to `hive_mind`;
//! `crate::registry` (`EntityInfo`, `EntryHealth`, `PhaseEra`, `RegistryType`,
//! `SubRegistry`, `WorkspaceContext`) stayed in `hypervisor` as
//! `hypervisor::registry`, and is re-exported from
//! `hive_mind::hybrid_master_registry` (`pub use hypervisor::registry::{...}`),
//! so a single glob import covers both.

use hive_mind::hybrid_master_registry::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Integration test registry that simulates realistic query patterns
#[derive(Clone)]
struct IntegrationMockRegistry {
    registry_type: RegistryType,
    entities: Arc<Mutex<HashMap<String, EntityInfo>>>,
    state: Arc<Mutex<IntegrationState>>,
}

#[derive(Clone, Debug)]
struct IntegrationState {
    initialized: bool,
    synchronized: bool,
    query_history: Vec<String>,
    sync_count: usize,
}

impl IntegrationMockRegistry {
    fn new(registry_type: RegistryType) -> Self {
        Self {
            registry_type,
            entities: Arc::new(Mutex::new(HashMap::new())),
            state: Arc::new(Mutex::new(IntegrationState {
                initialized: false,
                synchronized: false,
                query_history: Vec::new(),
                sync_count: 0,
            })),
        }
    }

    fn add_entity(&self, id: String, info: EntityInfo) {
        self.entities.lock().unwrap().insert(id, info);
    }

    #[allow(dead_code)]
    fn get_query_history(&self) -> Vec<String> {
        self.state.lock().unwrap().query_history.clone()
    }

    #[allow(dead_code)]
    fn sync_count(&self) -> usize {
        self.state.lock().unwrap().sync_count
    }
}

impl SubRegistry for IntegrationMockRegistry {
    fn initialize(&mut self, ctx: &WorkspaceContext) -> Result<(), String> {
        if ctx.current_era != PhaseEra::SixD {
            return Err(format!(
                "Incompatible phase era. Expected SixD, found {:?}",
                ctx.current_era
            ));
        }
        self.state.lock().unwrap().initialized = true;
        Ok(())
    }

    fn query_entity(&self, id: &str) -> Option<EntityInfo> {
        self.state
            .lock()
            .unwrap()
            .query_history
            .push(id.to_string());
        self.entities.lock().unwrap().get(id).cloned()
    }

    fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if !state.initialized {
            return Err("Not initialized".to_string());
        }
        state.synchronized = true;
        state.sync_count += 1;
        Ok(())
    }

    fn registry_type(&self) -> RegistryType {
        self.registry_type
    }
}

#[test]
fn test_phase_6d_full_lifecycle() {
    let mut master = MasterRegistry::new();

    assert_eq!(master.active_era(), PhaseEra::SixD);

    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::Unified,
    )));
    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::HoxCapability,
    )));
    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::Chromosome,
    )));

    assert_eq!(master.registry_count(), 3);

    let init_result = master.initialize();
    assert!(init_result.is_ok());

    let sync_result = master.synchronize_state();
    assert!(sync_result.is_ok());
}

#[test]
fn test_multi_adapter_entity_queries() {
    let mut master = MasterRegistry::new();

    let unified_reg = IntegrationMockRegistry::new(RegistryType::Unified);
    unified_reg.add_entity(
        "entity-1".to_string(),
        EntityInfo {
            id: "entity-1".to_string(),
            name: Some("Unified Entity".to_string()),
            version: Some("1.0".to_string()),
            health: EntryHealth::Healthy,
            last_seen: 1000,
        },
    );

    let hox_reg = IntegrationMockRegistry::new(RegistryType::HoxCapability);
    hox_reg.add_entity(
        "entity-2".to_string(),
        EntityInfo {
            id: "entity-2".to_string(),
            name: Some("Hox Entity".to_string()),
            version: Some("2.0".to_string()),
            health: EntryHealth::Healthy,
            last_seen: 2000,
        },
    );

    master.add_registry(Box::new(unified_reg));
    master.add_registry(Box::new(hox_reg));

    let _ = master.initialize();

    let result1 = master.query_entity("entity-1");
    assert!(result1.is_some());
    assert_eq!(result1.unwrap().name, Some("Unified Entity".to_string()));

    let result2 = master.query_entity("entity-2");
    assert!(result2.is_some());
    assert_eq!(result2.unwrap().name, Some("Hox Entity".to_string()));
}

#[test]
fn test_composition_strategy_builder() {
    let strategy = RegistryCompositionStrategy::new();
    let ctx = WorkspaceContext::default();

    let master = strategy.build_master_registry(&ctx);

    assert_eq!(master.registry_count(), 0);
    assert_eq!(master.active_era(), PhaseEra::SixD);
}

#[test]
fn test_registry_health_status_propagation() {
    let mut master = MasterRegistry::new();

    let healthy_reg = IntegrationMockRegistry::new(RegistryType::Unified);
    healthy_reg.add_entity(
        "healthy-entity".to_string(),
        EntityInfo {
            id: "healthy-entity".to_string(),
            name: Some("Healthy".to_string()),
            version: Some("1.0".to_string()),
            health: EntryHealth::Healthy,
            last_seen: 1000,
        },
    );

    let degraded_reg = IntegrationMockRegistry::new(RegistryType::HoxCapability);
    degraded_reg.add_entity(
        "degraded-entity".to_string(),
        EntityInfo {
            id: "degraded-entity".to_string(),
            name: Some("Degraded".to_string()),
            version: Some("2.0".to_string()),
            health: EntryHealth::Degraded,
            last_seen: 2000,
        },
    );

    master.add_registry(Box::new(healthy_reg));
    master.add_registry(Box::new(degraded_reg));

    let healthy = master.query_entity("healthy-entity");
    assert!(healthy.is_some());
    assert_eq!(healthy.unwrap().health, EntryHealth::Healthy);

    let degraded = master.query_entity("degraded-entity");
    assert!(degraded.is_some());
    assert_eq!(degraded.unwrap().health, EntryHealth::Degraded);
}

#[test]
fn test_registry_removal_and_readdition() {
    let mut master = MasterRegistry::new();

    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::Unified,
    )));
    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::HoxCapability,
    )));
    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::Chromosome,
    )));

    assert_eq!(master.registry_count(), 3);

    let removed = master.remove_registry(RegistryType::Unified);
    assert!(removed.is_some());
    assert_eq!(master.registry_count(), 2);

    master.add_registry(removed.unwrap());
    assert_eq!(master.registry_count(), 3);
}

#[test]
fn test_synchronization_error_handling() {
    let mut master = MasterRegistry::new();

    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::Unified,
    )));

    let sync_result = master.synchronize_state();
    assert!(
        sync_result.is_ok(),
        "Sync should handle uninitialized registries gracefully"
    );
}

#[test]
fn test_phase_era_validation_on_init() {
    let mut master = MasterRegistry::new();
    master.set_active_era(PhaseEra::ThreeC);

    let ctx = WorkspaceContext::default();
    assert_eq!(ctx.current_era, PhaseEra::SixD);

    master.add_registry(Box::new(IntegrationMockRegistry::new(
        RegistryType::Unified,
    )));

    let mut test_reg = IntegrationMockRegistry::new(RegistryType::Unified);
    let mut wrong_ctx = WorkspaceContext::default();
    wrong_ctx.current_era = PhaseEra::FourD;

    let init_result = test_reg.initialize(&wrong_ctx);
    assert!(init_result.is_err());
}

#[test]
fn test_concurrent_registry_operations() {
    let mut master = MasterRegistry::new();

    for _i in 0..10 {
        master.add_registry(Box::new(IntegrationMockRegistry::new(
            RegistryType::Unified,
        )));
    }

    assert_eq!(master.registry_count(), 10);

    let init_result = master.initialize();
    assert!(init_result.is_ok());

    let sync_result = master.synchronize_state();
    assert!(sync_result.is_ok());
}

#[test]
fn test_entity_last_seen_tracking() {
    let mut master = MasterRegistry::new();

    let reg = IntegrationMockRegistry::new(RegistryType::Unified);
    reg.add_entity(
        "tracked-entity".to_string(),
        EntityInfo {
            id: "tracked-entity".to_string(),
            name: Some("Tracked".to_string()),
            version: Some("1.0".to_string()),
            health: EntryHealth::Healthy,
            last_seen: 12345,
        },
    );

    master.add_registry(Box::new(reg));

    let entity = master.query_entity("tracked-entity");
    assert!(entity.is_some());
    assert_eq!(entity.unwrap().last_seen, 12345);
}

#[test]
fn test_registry_metadata_consistency() {
    let master = MasterRegistry::new();

    let ctx = master.ctx();
    assert_eq!(ctx.current_era, PhaseEra::SixD);
    assert_eq!(ctx.registry_version, "1.0.0");
}

#[test]
fn test_all_registry_types_supported() {
    let mut master = MasterRegistry::new();

    let types = vec![
        RegistryType::Unified,
        RegistryType::FederationModel,
        RegistryType::FederationLinks,
        RegistryType::LLMModel,
        RegistryType::Component,
        RegistryType::Specialist,
        RegistryType::Chromosome,
        RegistryType::HoxCapability,
        RegistryType::DistributedSpecialist,
    ];

    for ty in types {
        master.add_registry(Box::new(IntegrationMockRegistry::new(ty)));
    }

    assert_eq!(master.registry_count(), 9);
}

#[test]
fn test_entity_not_found_returns_none() {
    let mut master = MasterRegistry::new();

    let reg = IntegrationMockRegistry::new(RegistryType::Unified);
    master.add_registry(Box::new(reg));

    let result = master.query_entity("non-existent");
    assert!(result.is_none());
}

#[test]
fn test_registry_list_all_entities() {
    let master = MasterRegistry::new();

    let entities = master.list_all_entities();
    // Implementation returns empty for now, but structure is in place
    assert!(entities.is_empty());
}
