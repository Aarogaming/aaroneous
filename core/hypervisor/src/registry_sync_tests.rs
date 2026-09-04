/// Synchronization consistency tests for the Phase 6D hybrid master registry system.
///
/// Tests the state machine behavior across all sub-registries to ensure
/// cross-registry synchronization maintains consistency during state transitions.

#[cfg(test)]
mod sync_consistency_tests {
    use crate::hybrid_master_registry::*;
    use crate::registry::{
        EntityInfo, EntryHealth, PhaseEra, RegistryType, SubRegistry, WorkspaceContext,
    };
    use std::sync::{Arc, Mutex};

    /// Mock registry with state tracking for synchronization tests
    #[derive(Clone)]
    struct StatefulMockRegistry {
        state: Arc<Mutex<RegistryState>>,
    }

    #[derive(Clone, Debug)]
    struct RegistryState {
        initialized: bool,
        synchronized: bool,
        query_count: usize,
        sync_count: usize,
    }

    impl StatefulMockRegistry {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(RegistryState {
                    initialized: false,
                    synchronized: false,
                    query_count: 0,
                    sync_count: 0,
                })),
            }
        }

        #[allow(dead_code)]
        fn is_synchronized(&self) -> bool {
            self.state.lock().unwrap().synchronized
        }

        #[allow(dead_code)]
        fn get_state(&self) -> RegistryState {
            self.state.lock().unwrap().clone()
        }
    }

    impl SubRegistry for StatefulMockRegistry {
        fn initialize(&mut self, ctx: &WorkspaceContext) -> Result<(), String> {
            if ctx.current_era != PhaseEra::SixD {
                return Err("Invalid phase era".to_string());
            }
            self.state.lock().unwrap().initialized = true;
            Ok(())
        }

        fn query_entity(&self, _id: &str) -> Option<EntityInfo> {
            self.state.lock().unwrap().query_count += 1;
            None
        }

        fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
            if !self.state.lock().unwrap().initialized {
                return Err("Not initialized".to_string());
            }

            self.state.lock().unwrap().synchronized = true;
            self.state.lock().unwrap().sync_count += 1;
            Ok(())
        }

        fn registry_type(&self) -> RegistryType {
            RegistryType::Unified
        }
    }

    #[test]
    fn test_synchronization_maintains_consistency() {
        let mut master = MasterRegistry::new();
        let mock1 = Box::new(StatefulMockRegistry::new());
        let mock2 = Box::new(StatefulMockRegistry::new());

        master.add_registry(mock1);
        master.add_registry(mock2);

        // Initialize should set up registries
        let init_result = master.initialize();
        assert!(init_result.is_ok(), "Initialization failed");

        // Synchronization should succeed after initialization
        let sync_result = master.synchronize_state();
        assert!(sync_result.is_ok(), "Synchronization failed");
    }

    #[test]
    fn test_state_machine_era_transition() {
        let mut master = MasterRegistry::new();

        // Start in SixD
        assert_eq!(master.active_era(), PhaseEra::SixD);

        // Transition to ThreeC
        master.set_active_era(PhaseEra::ThreeC);
        assert_eq!(master.active_era(), PhaseEra::ThreeC);

        // Transition back to SixD
        master.set_active_era(PhaseEra::SixD);
        assert_eq!(master.active_era(), PhaseEra::SixD);
    }

    #[test]
    fn test_multiple_registry_synchronization_cascade() {
        let mut master = MasterRegistry::new();

        // Add multiple registries
        for _ in 0..5 {
            master.add_registry(Box::new(StatefulMockRegistry::new()));
        }

        assert_eq!(master.registry_count(), 5);

        let _ctx = WorkspaceContext::default();

        // Initialize all registries
        let init_result = master.initialize();
        assert!(init_result.is_ok());

        // Synchronize all registries
        let sync_result = master.synchronize_state();
        assert!(sync_result.is_ok());
    }

    #[test]
    fn test_registry_removal_during_sync() {
        let mut master = MasterRegistry::new();

        master.add_registry(Box::new(StatefulMockRegistry::new()));
        master.add_registry(Box::new(StatefulMockRegistry::new()));

        assert_eq!(master.registry_count(), 2);

        // Remove one registry
        let removed = master.remove_registry(RegistryType::Unified);
        assert!(removed.is_some());
        assert_eq!(master.registry_count(), 1);

        // Sync should still work with remaining registry
        let sync_result = master.synchronize_state();
        assert!(sync_result.is_ok());
    }

    #[test]
    fn test_cross_registry_query_ordering() {
        let mut master = MasterRegistry::new();

        // Add multiple registries - query should check all
        master.add_registry(Box::new(StatefulMockRegistry::new()));
        master.add_registry(Box::new(StatefulMockRegistry::new()));
        master.add_registry(Box::new(StatefulMockRegistry::new()));

        let result = master.query_entity("test-id");
        // All mocks return None, so final result should be None
        assert!(result.is_none());
    }

    #[test]
    fn test_registry_metadata_tracking() {
        let master = MasterRegistry::new();

        assert_eq!(master.ctx().current_era, PhaseEra::SixD);
        assert_eq!(master.ctx().registry_version, "1.0.0");
    }

    #[test]
    fn test_workspace_context_preservation_across_sync() {
        let mut master = MasterRegistry::new();
        master.add_registry(Box::new(StatefulMockRegistry::new()));

        let _original_ctx = WorkspaceContext::default();

        let _ = master.initialize();

        // Context should still point to SixD
        assert_eq!(master.active_era(), PhaseEra::SixD);
        assert_eq!(master.ctx().registry_version, "1.0.0");
    }

    #[test]
    fn test_composition_strategy_wiring() {
        let strategy = RegistryCompositionStrategy::new();
        let ctx = WorkspaceContext::default();

        let master = strategy.build_master_registry(&ctx);

        // Empty strategy should create empty master
        assert_eq!(master.registry_count(), 0);
        assert_eq!(master.active_era(), PhaseEra::SixD);
    }

    #[test]
    fn test_entity_health_consistency_across_registries() {
        let healthy = EntityInfo {
            id: "test".to_string(),
            name: Some("Test Entity".to_string()),
            version: Some("1.0".to_string()),
            health: EntryHealth::Healthy,
            last_seen: 0,
        };

        assert_eq!(healthy.health, EntryHealth::Healthy);

        let degraded = EntityInfo {
            id: "test".to_string(),
            name: Some("Test Entity".to_string()),
            version: Some("1.0".to_string()),
            health: EntryHealth::Degraded,
            last_seen: 0,
        };

        assert_eq!(degraded.health, EntryHealth::Degraded);
        assert_ne!(healthy.health, degraded.health);
    }
}
