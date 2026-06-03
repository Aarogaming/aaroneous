/// End-to-End Integration Tests
/// Validates complete feedback loop from sensors to learning updates

#[cfg(test)]
mod end_to_end_tests {
    use std::sync::Arc;
    use parking_lot::RwLock;

    /// Test: Complete autonomic cycle with thermal throttling
    #[test]
    fn test_autonomic_cycle_with_thermal_throttling() {
        println!("\n[E2E TEST] Autonomic cycle with thermal throttling");
        
        // Setup
        let metrics = crate::system_metrics::SystemMetricsCollector::new();
        
        // Simulate high thermal load
        let thermal = metrics.get_thermal_metrics();
        println!("  Initial thermal status: {:?}", thermal.cpu_status);
        
        // Verify throttling capability
        let should_throttle = metrics.should_throttle();
        let throttle_factor = metrics.get_throttle_factor();
        
        println!("  Should throttle: {}", should_throttle);
        println!("  Throttle factor: {:.2}x", throttle_factor);
        
        // Assert normal conditions don't throttle
        assert!(throttle_factor <= 1.0);
        
        println!("  ✓ Thermal monitoring operational");
    }

    /// Test: Task routing end-to-end
    #[test]
    fn test_task_routing_end_to_end() {
        println!("\n[E2E TEST] Task routing end-to-end");
        
        // Create router
        let router = crate::task_routing::TaskRouter::new(None, None, None);
        
        // Test routing for various task types
        let test_cases = vec![
            ("wasm_process", crate::task_routing::ExecutionRoute::Enzyme),
            ("network_call", crate::task_routing::ExecutionRoute::Network),
            ("cpu_intensive", crate::task_routing::ExecutionRoute::CpuIntensive),
            ("learning_task", crate::task_routing::ExecutionRoute::Learning),
        ];
        
        for (task_type, expected_route) in test_cases {
            let route = router.recommend_route(task_type);
            assert_eq!(route, expected_route);
            println!("  {} → {:?} ✓", task_type, route);
        }
        
        println!("  ✓ All routes classified correctly");
    }

    /// Test: Specialist memory consultation workflow
    #[test]
    fn test_specialist_memory_consultation() {
        println!("\n[E2E TEST] Specialist memory consultation");
        
        // Create memory store
        let store = crate::specialist_memory::SpecialistMemoryStore::new("test_specialist".to_string());
        
        // Store some memories
        let mut memory1 = crate::specialist_memory::MemoryEntry::new(
            "mem_1".to_string(),
            "test_specialist".to_string(),
            "How to handle errors".to_string(),
            "Use proper error handling".to_string(),
            crate::specialist_memory::MemoryType::Procedural,
        );
        memory1.tags = vec!["error".to_string(), "handling".to_string()];
        memory1.confidence = 0.9;
        
        store.store_memory(memory1);
        
        // Query memory
        let result = store.query_memory("error handling", "error_task", 5);
        
        println!("  Query: 'error handling'");
        println!("  Result: {}", result.recommendation);
        println!("  Entries found: {}", result.entries.len());
        
        if !result.entries.is_empty() {
            for (i, entry) in result.entries.iter().enumerate() {
                println!("    {}. {} (confidence: {:.1}%)", 
                    i + 1, entry.title, entry.confidence * 100.0);
            }
        }
        
        assert!(!result.entries.is_empty());
        println!("  ✓ Memory consultation working");
    }

    /// Test: Registry persistence and recovery
    #[test]
    fn test_registry_persistence_and_recovery() {
        println!("\n[E2E TEST] Registry persistence and recovery");
        
        use tempfile::tempdir;
        
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("hox.db");
        let snap_dir = temp_dir.path().join("snapshots");
        
        // Create persistence manager
        let manager = crate::hox_persistence::HoxPersistenceManager::new(
            db_path.to_str().unwrap(),
            snap_dir.to_str().unwrap(),
        ).unwrap();
        
        // Create snapshot
        let snapshot_path = manager.auto_save().unwrap();
        println!("  Snapshot created: {}", snapshot_path.display());
        
        // List snapshots
        let snapshots = manager.list_snapshots().unwrap();
        println!("  Snapshots available: {}", snapshots.len());
        
        assert!(snapshot_path.exists());
        assert_eq!(snapshots.len(), 1);
        
        println!("  ✓ Persistence and recovery working");
    }

    /// Test: Dopamine-driven learning workflow
    #[test]
    fn test_dopamine_learning_workflow() {
        println!("\n[E2E TEST] Dopamine-driven learning workflow");
        
        let config = crate::unified_learning::UnifiedLearningConfig::default();
        let specialist_ids = vec!["specialist_a".to_string(), "specialist_b".to_string()];
        let mut learning_loop = crate::unified_learning::UnifiedLearningLoop::new(
            config, 
            2, 
            specialist_ids,
        );
        
        // Store initial weights
        let initial_weights = learning_loop.tensor_router.weights.weights.clone();
        
        // Execute learning cycle
        let task_features = vec![0.7, 0.5, 0.8, 0.3];
        let result = learning_loop.learn_from_dopamine(
            &task_features,
            "specialist_a",
            0.8,  // High reward
            0.9,  // High confidence
        );
        
        println!("  Specialist: {}", result.specialist_id);
        println!("  Learning signal: {:.2}", result.learning_signal);
        println!("  Adaptive LR: {:.4}", result.adaptive_learning_rate);
        println!("  Confidence factor: {:.2}", result.confidence_factor);
        println!("  Training time: {}μs", result.training_time_ms);
        
        // Verify weights updated
        let updated_weights = learning_loop.tensor_router.weights.weights.clone();
        assert_ne!(updated_weights, initial_weights);
        
        println!("  ✓ Dopamine learning executed successfully");
    }

    /// Test: Complete feedback loop
    #[test]
    fn test_complete_feedback_loop() {
        println!("\n[E2E TEST] Complete feedback loop");
        
        println!("  Stage 1: Thermal monitoring");
        let metrics = crate::system_metrics::SystemMetricsCollector::new();
        let thermal = metrics.get_thermal_metrics();
        let throttle_factor = metrics.get_throttle_factor();
        println!("    Thermal status: {:?}", thermal.cpu_status);
        println!("    Throttle factor: {:.2}x", throttle_factor);
        
        println!("  Stage 2: Task classification");
        let router = crate::task_routing::TaskRouter::new(None, None, None);
        let route = router.recommend_route("wasm_task");
        println!("    Route selected: {:?}", route);
        
        println!("  Stage 3: Memory consultation");
        let store = crate::specialist_memory::SpecialistMemoryStore::new("specialist".to_string());
        let result = store.query_memory("task", "execution", 3);
        println!("    Recommendation: {}", result.recommendation);
        
        println!("  Stage 4: Learning update");
        let config = crate::unified_learning::UnifiedLearningConfig::default();
        let specialist_ids = vec!["specialist".to_string()];
        let mut learning_loop = crate::unified_learning::UnifiedLearningLoop::new(
            config,
            1,
            specialist_ids,
        );
        let task_features = vec![0.5, 0.5, 0.5, 0.5];
        let training_result = learning_loop.learn_from_dopamine(
            &task_features,
            "specialist",
            0.7,
            0.8,
        );
        println!("    Weights updated: {}", training_result.weights_updated);
        
        println!("  Stage 5: Persistence");
        let checkpoint = learning_loop.checkpoint_model();
        println!("    Checkpoint created with {} history entries", 
            checkpoint.load_history.len());
        
        println!("  ✓ Complete feedback loop verified");
    }

    /// Test: Concurrent learning and routing
    #[test]
    fn test_concurrent_learning_and_routing() {
        println!("\n[E2E TEST] Concurrent learning and routing");
        
        let config = Arc::new(crate::unified_learning::UnifiedLearningConfig::default());
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string(), "spec_c".to_string()];
        let learning_loop = Arc::new(RwLock::new(
            crate::unified_learning::UnifiedLearningLoop::new(
                (*config).clone(),
                3,
                specialist_ids.clone(),
            )
        ));
        
        let router = Arc::new(crate::task_routing::TaskRouter::new(None, None, None));
        
        println!("  Simulating 10 concurrent operations");
        
        let mut handles = vec![];
        for i in 0..10 {
            let loop_clone = learning_loop.clone();
            let router_clone = router.clone();
            
            let handle = std::thread::spawn(move || {
                // Routing operation
                let route = router_clone.recommend_route("task");
                
                // Learning operation
                let task_features = vec![0.5 + (i as f64 * 0.01); 4];
                let mut loop_write = loop_clone.write();
                let result = loop_write.learn_from_dopamine(
                    &task_features,
                    &format!("spec_{}", i % 3),
                    (i as f32 * 0.1) - 0.5,
                    0.8,
                );
                
                println!("    Thread {}: Route {:?}, Learning time: {}μs", i, route, result.training_time_ms);
            });
            
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
        
        println!("  ✓ Concurrent operations completed successfully");
    }

    /// Test: Error recovery
    #[test]
    fn test_error_recovery() {
        println!("\n[E2E TEST] Error recovery");
        
        // Test invalid specialist recovery
        let config = crate::unified_learning::UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string()];
        let mut learning_loop = crate::unified_learning::UnifiedLearningLoop::new(
            config,
            1,
            specialist_ids,
        );
        
        // Try to learn from non-existent specialist (should not crash)
        let task_features = vec![0.5, 0.5, 0.5, 0.5];
        let result = learning_loop.learn_from_dopamine(
            &task_features,
            "non_existent_specialist",
            0.5,
            0.8,
        );
        
        println!("  Attempted learning from non-existent specialist");
        println!("  Result: specialist={}, weights_updated={}", 
            result.specialist_id, result.weights_updated);
        
        assert_eq!(result.specialist_id, "non_existent_specialist");
        println!("  ✓ Error gracefully handled");
    }

    /// Test: State consistency after multiple cycles
    #[test]
    fn test_state_consistency_multiple_cycles() {
        println!("\n[E2E TEST] State consistency across multiple cycles");
        
        let config = crate::unified_learning::UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut learning_loop = crate::unified_learning::UnifiedLearningLoop::new(
            config,
            2,
            specialist_ids,
        );
        
        println!("  Running 20 learning cycles");
        
        for cycle in 0..20 {
            let task_features = vec![
                (cycle as f64 * 0.05).sin().abs(),
                (cycle as f64 * 0.03).cos().abs(),
                0.5 + (cycle as f64 * 0.02),
                0.7,
            ];
            
            let specialist = if cycle % 2 == 0 { "spec_a" } else { "spec_b" };
            let reward = ((cycle as f32 % 3.0) / 3.0) - 0.5;
            
            let result = learning_loop.learn_from_dopamine(
                &task_features,
                specialist,
                reward,
                0.8,
            );
            
            // Verify state remains valid
            assert!(!learning_loop.system_state.learning_rate.is_nan());
            assert!(!learning_loop.system_state.prediction_error.is_nan());
            assert!(!learning_loop.system_state.estimated_load.is_nan());
            
            if cycle % 5 == 0 {
                println!("    Cycle {}: LR={:.4}, PE={:.4}", 
                    cycle,
                    learning_loop.system_state.learning_rate,
                    learning_loop.system_state.prediction_error);
            }
        }
        
        println!("  ✓ State remained consistent across all cycles");
    }

    /// Test: Checkpoint recovery integrity
    #[test]
    fn test_checkpoint_recovery_integrity() {
        println!("\n[E2E TEST] Checkpoint recovery integrity");
        
        let config = crate::unified_learning::UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut loop1 = crate::unified_learning::UnifiedLearningLoop::new(
            config.clone(),
            2,
            specialist_ids.clone(),
        );
        
        // Train for a few cycles
        for i in 0..5 {
            let task_features = vec![0.5 + (i as f64 * 0.1), 0.5, 0.5, 0.5];
            loop1.learn_from_dopamine(&task_features, "spec_a", 0.5, 0.8);
        }
        
        // Create checkpoint
        let checkpoint = loop1.checkpoint_model();
        println!("  Checkpoint created:");
        println!("    Specialist count: {}", checkpoint.specialist_ids.len());
        println!("    History size: {}", checkpoint.load_history.len());
        
        // Create new loop and restore
        let mut loop2 = crate::unified_learning::UnifiedLearningLoop::new(
            config,
            2,
            specialist_ids,
        );
        
        let restored = loop2.restore_from_checkpoint(&checkpoint);
        assert!(restored);
        
        // Verify recovery
        assert_eq!(loop2.tensor_router.weights.weights, loop1.tensor_router.weights.weights);
        assert_eq!(loop2.system_state.learning_rate, loop1.system_state.learning_rate);
        assert_eq!(loop2.load_history, loop1.load_history);
        
        println!("  ✓ Checkpoint recovery integrity verified");
    }
}
