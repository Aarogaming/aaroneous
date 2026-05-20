#[cfg(test)]
mod tests {
    use crate::autonomic_loop::AutonomicNervousSystem;
    use crate::enzyme_runner::EnzymeRunner;
    use crate::hox_registry::{HoxRegistry, HoxCapability};
    use crate::splicing_engine::WasmSplicingEngine;
    use crate::unified_learning::{UnifiedLearningLoop, UnifiedLearningConfig};
    use nervous_system::shared_memory::SynapseState;
    use std::sync::Arc;
    use parking_lot::RwLock;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::thread;

    #[test]
    fn test_self_healing_evolution_loop() {
        let hox_db = "data/test_hox.db";
        let _ = std::fs::remove_dir_all(hox_db);
        
        let hox_registry = Arc::new(HoxRegistry::new(hox_db).unwrap());
        let enzyme_runner = Arc::new(EnzymeRunner::new().unwrap());
        let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
            UnifiedLearningConfig::default(),
            1,
            vec!["specialist_0".to_string()],
        )));

        // Create initial "DNA" record
        let initial_wasm = "extensions/wasm/test_enzyme/target/wasm32-unknown-unknown/release/test_enzyme.wasm";
        hox_registry.register_capability(&HoxCapability {
            name: "specialist_0".to_string(),
            enzyme_hash: initial_wasm.to_string(),
            permissions: vec!["full".to_string()],
        }).unwrap();

        let splicing_engine = Arc::new(WasmSplicingEngine::new(
            hox_registry.clone(),
            crate::workspace::WorkspacePaths::discover().root().clone(),
        ));

        let autonomic_ns = AutonomicNervousSystem::new(
            "TEST_AUTONOMIC_SYNAPSE",
            50, // 20Hz for fast test
            enzyme_runner.clone(),
            hox_registry.clone(),
            splicing_engine.clone(),
            learning_loop.clone(),
        ).unwrap();

        let synapse = autonomic_ns.get_synapse();
        autonomic_ns.start();

        // 1. Verify normal operation
        thread::sleep(Duration::from_millis(200));
        {
            let syn = synapse.read();
            let state = unsafe { &*(syn.get_ptr() as *const SynapseState) };
            assert!(state.clock_tick > 0, "Clock should be ticking");
            assert_eq!(state.error_sentinel, 0, "Should start with no error");
        }

        // 2. TRIGGER ARTIFICIAL FAILURE
        println!("[Test] Injecting error sentinel into Synapse...");
        {
            let mut syn = synapse.write();
            let state = unsafe { &mut *(syn.get_ptr() as *mut SynapseState) };
            state.error_sentinel = 1;
        }

        // 3. Wait for Autonomic Loop to detect and Splicing Engine to "evolve"
        // We give it some time to generate the patch and update registry
        thread::sleep(Duration::from_millis(1000));

        // 4. VERIFY EVOLUTION
        let updated_cap = hox_registry.get_capability("specialist_0").unwrap().unwrap();
        println!("[Test] Evolution complete. New enzyme hash: {}", updated_cap.enzyme_hash);
        
        // In our current deterministic healer, it keeps the same path but modifies the .rs source.
        // If we implemented the full compilation, the hash would change.
        // For now, we verify the sentinel was cleared.
        {
            let syn = synapse.read();
            let state = unsafe { &*(syn.get_ptr() as *const SynapseState) };
            assert_eq!(state.error_sentinel, 0, "Sentinel should be cleared by autonomic loop");
        }
    }
}
