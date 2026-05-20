#[cfg(test)]
mod tests {
    use crate::autonomic_loop::AutonomicNervousSystem;
    use crate::enzyme_runner::EnzymeRunner;
    use crate::hox_registry::HoxRegistry;
    use crate::splicing_engine::WasmSplicingEngine;
    use crate::unified_learning::{UnifiedLearningLoop, UnifiedLearningConfig};
    use nervous_system::shared_memory::SynapseState;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::thread;

    #[test]
    fn test_nlm_sentinel_and_hitl_handshake() {
        let hox_db = "data/test_hox_sentinel.db";
        let _ = std::fs::remove_dir_all(hox_db);
        
        let hox_registry = Arc::new(HoxRegistry::new(hox_db).unwrap());
        let enzyme_runner = Arc::new(EnzymeRunner::new().unwrap());
        let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
            UnifiedLearningConfig::default(),
            1,
            vec!["specialist_0".to_string()],
        )));

        let splicing_engine = Arc::new(WasmSplicingEngine::new(
            hox_registry.clone(),
            crate::workspace::WorkspacePaths::discover().root().clone(),
        ));

        let autonomic_ns = AutonomicNervousSystem::new(
            "TEST_SENTINEL_SYNAPSE",
            50,
            enzyme_runner.clone(),
            hox_registry.clone(),
            splicing_engine.clone(),
            learning_loop.clone(),
        ).unwrap();

        let synapse = autonomic_ns.get_synapse();
        autonomic_ns.start();

        // 1. TEST TIER 1 TASK (Requires Approval)
        println!("[Test] Injecting Tier 1 Intent (Search)...");
        {
            let mut syn = synapse.write();
            let state = unsafe { &mut *(syn.get_ptr() as *mut SynapseState) };
            state.intent_vector_id = [0xAA; 16]; // Mock task
            state.approval_granted = 0;
            state.approval_required = 0;
        }

        thread::sleep(Duration::from_millis(150));

        {
            let syn = synapse.read();
            let state = unsafe { &*(syn.get_ptr() as *const SynapseState) };
            assert_eq!(state.sovereignty_tier, 1, "Task should be allocated to Tier 1");
            assert_eq!(state.approval_required, 1, "User approval should be requested");
            assert_eq!(state.intent_vector_id, [0xAA; 16], "Task should be stalled, not cleared");
        }

        // 2. GRANT APPROVAL
        println!("[Test] Granting approval via Synapse...");
        {
            let mut syn = synapse.write();
            let state = unsafe { &mut *(syn.get_ptr() as *mut SynapseState) };
            state.approval_granted = 1;
        }

        thread::sleep(Duration::from_millis(150));

        {
            let syn = synapse.read();
            let state = unsafe { &*(syn.get_ptr() as *const SynapseState) };
            assert_eq!(state.intent_vector_id, [0; 16], "Task should be executed and cleared");
            assert_eq!(state.approval_required, 0, "Approval flag should be reset");
        }

        // 3. TEST VIOLATION
        println!("[Test] Injecting VIOLATION Intent...");
        // In real use, we'd change the intent text read by the sentinel. 
        // For the test, we mock the classification result or use keywords in the mock text.
        // The mock in autonomic_loop uses a hardcoded string currently.
        // Let's verify the sentinel clears the vector.
    }
}
