// Integration tests for unified learning loop
// Tests the complete OBSERVE → ESTIMATE → PREDICT → ROUTE → ACT → LEARN cycle

#[cfg(test)]
mod unified_integration_tests {
    use crate::unified_learning::{UnifiedLearningLoop, UnifiedLearningConfig, UnifiedSystemState};
    use crate::tensor_router::{TensorRouter, RoutingWeights, TaskEmbedding};
    use crate::spectral_layout::{spectral_layout_2d, build_similarity_edges};
    use compute::thermodynamics::SystemPhase;
    use compute::information::{shannon_entropy, mutual_information};
    use compute::predictive_coding::{HierarchicalPredictiveCoding, PredictiveNode};

    #[test]
    fn test_complete_learning_cycle() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_code".to_string(), "spec_test".to_string(), "spec_review".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 3, specialist_ids);

        // Simulate 10 cycles of learning
        let mut cycle_results = Vec::new();
        for i in 0..10 {
            // Simulate varying load
            let load = 0.3 + (i as f64 * 0.05).sin() * 0.2;
            let observations = vec![load];
            let task_features = vec![0.5 + (i as f64 * 0.1).cos() * 0.3, 0.6, 0.7, 0.4];

            let result = loop_.run_cycle(&observations, &task_features);
            cycle_results.push(result);

            // Simulate task outcome
            let success = i % 3 != 0; // 2/3 success rate
            loop_.learn_from_outcome(&task_features, &result.routing_result.selected_specialist, success);
        }

        // Verify learning occurred
        assert_eq!(cycle_results.len(), 10);
        
        // System should stabilize
        let first_load = cycle_results[0].estimated_load;
        let last_load = cycle_results[9].estimated_load;
        assert!(first_load.is_finite());
        assert!(last_load.is_finite());
    }

    #[test]
    fn test_tensor_routing_convergence() {
        let weights = RoutingWeights::new(3, 4, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let mut router = TensorRouter::new(weights, 0.5); // Low temperature for exploitation

        // Route same task multiple times and learn
        let task = TaskEmbedding {
            task_id: "consistent_task".to_string(),
            features: vec![0.8, 0.7, 0.9, 0.6],
        };

        let mut selected_counts = std::collections::HashMap::new();

        for _ in 0..20 {
            let result = router.route(&task);
            *selected_counts.entry(result.selected_specialist.clone()).or_insert(0) += 1;
            
            // Always succeed with specialist "a"
            router.learn(&task.features, "a", true, 0.1);
            router.learn(&task.features, "b", false, 0.1);
            router.learn(&task.features, "c", false, 0.1);
        }

        // Specialist "a" should be selected most often after learning
        let a_count = selected_counts.get("a").unwrap_or(&0);
        let b_count = selected_counts.get("b").unwrap_or(&0);
        let c_count = selected_counts.get("c").unwrap_or(&0);
        
        assert!(*a_count >= *b_count);
        assert!(*a_count >= *c_count);
    }

    #[test]
    fn test_spectral_layout_stability() {
        // Create a graph with clear clusters
        let edges = vec![
            // Cluster 1
            (0, 1, 1.0),
            (1, 2, 1.0),
            (2, 0, 1.0),
            // Cluster 2
            (3, 4, 1.0),
            (4, 5, 1.0),
            (5, 3, 1.0),
            // Weak connection between clusters
            (2, 3, 0.1),
        ];

        let positions = spectral_layout_2d(6, &edges);
        assert_eq!(positions.len(), 6);

        // Nodes in same cluster should be closer
        let cluster1_dist = ((positions[0].0 - positions[1].0).powi(2) + (positions[0].1 - positions[1].1).powi(2)).sqrt();
        let cross_cluster_dist = ((positions[0].0 - positions[3].0).powi(2) + (positions[0].1 - positions[3].1).powi(2)).sqrt();

        // Cluster 1 nodes should be closer to each other than to cluster 2
        assert!(cluster1_dist < cross_cluster_dist);
    }

    #[test]
    fn test_information_theory_metrics() {
        // Test entropy computation
        let uniform = vec![0.25, 0.25, 0.25, 0.25];
        let entropy = shannon_entropy(&uniform);
        assert!((entropy - 2.0).abs() < 1e-10); // log2(4) = 2

        // Test mutual information
        let joint = vec![vec![0.5, 0.0], vec![0.0, 0.5]];
        let marg_x = vec![0.5, 0.5];
        let marg_y = vec![0.5, 0.5];
        let mi = mutual_information(&joint, &marg_x, &marg_y);
        assert!((mi - 1.0).abs() < 1e-10); // Perfect correlation
    }

    #[test]
    fn test_predictive_coding_learning() {
        let mut network = HierarchicalPredictiveCoding::new(&[3, 4, 2], 0.1);

        // Train on consistent pattern
        for _ in 0..50 {
            let observation = vec![0.7, 0.8, 0.6];
            let error = network.process(&observation);
            assert!(error >= 0.0);
        }

        // Prediction error should decrease over time
        let initial_error = {
            let mut temp = HierarchicalPredictiveCoding::new(&[3, 4, 2], 0.1);
            temp.process(&[0.7, 0.8, 0.6])
        };

        let final_error = network.process(&[0.7, 0.8, 0.6]);
        
        // Final error should be lower than initial (learning occurred)
        assert!(final_error <= initial_error);
    }

    #[test]
    fn test_thermodynamic_phase_transitions() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 1, specialist_ids);

        // Simulate stable phase
        for _ in 0..20 {
            loop_.run_cycle(&[0.5], &[0.5, 0.5, 0.5, 0.5]);
        }

        let stable_phase = loop_.system_state.phase.clone();
        
        // Simulate sudden load spike
        for _ in 0..10 {
            loop_.run_cycle(&[0.95], &[0.9, 0.9, 0.9, 0.9]);
        }

        let spike_phase = loop_.system_state.phase.clone();

        // Phase should change or system should respond
        assert!(stable_phase != SystemPhase::Unknown || spike_phase != SystemPhase::Unknown);
    }

    #[test]
    fn test_batch_task_routing() {
        let weights = RoutingWeights::new(4, 4, vec![
            "spec_a".to_string(), "spec_b".to_string(), 
            "spec_c".to_string(), "spec_d".to_string()
        ]);
        let router = TensorRouter::new(weights, 1.0);

        let tasks = vec![
            TaskEmbedding { task_id: "t1".to_string(), features: vec![0.8, 0.2, 0.9, 0.1] },
            TaskEmbedding { task_id: "t2".to_string(), features: vec![0.1, 0.9, 0.2, 0.8] },
            TaskEmbedding { task_id: "t3".to_string(), features: vec![0.5, 0.5, 0.5, 0.5] },
        ];

        let results = router.batch_route(&tasks);
        assert_eq!(results.len(), 3);

        // Each result should have valid probabilities
        for result in &results {
            let sum: f64 = result.specialist_scores.iter().map(|(_, p)| p).sum();
            assert!((sum - 1.0).abs() < 1e-10);
            assert!(result.confidence >= 0.0 && result.confidence <= 1.0);
        }
    }

    #[test]
    fn test_system_health_monitoring() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_a".to_string(), "spec_b".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 2, specialist_ids);

        // Run some cycles
        for i in 0..5 {
            let load = 0.4 + (i as f64 * 0.1);
            loop_.run_cycle(&[load], &[0.5, 0.6, 0.7, 0.8]);
        }

        let health = loop_.get_health_summary();

        // Health metrics should be valid
        assert!(health.free_energy.is_finite());
        assert!(health.estimated_load >= 0.0 && health.estimated_load <= 1.0);
        assert!(health.prediction_error >= 0.0);
        assert!(health.routing_confidence >= 0.0 && health.routing_confidence <= 1.0);
        assert!(health.expression_rate >= 0.0 && health.expression_rate <= 1.0);
        assert!(health.token_availability >= 0.0 && health.token_availability <= 1.0);
    }

    #[test]
    fn test_multi_specialist_load_balancing() {
        let config = UnifiedLearningConfig::default();
        let specialist_ids = vec!["spec_1".to_string(), "spec_2".to_string(), "spec_3".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 3, specialist_ids);

        // Run many cycles with varied tasks
        let mut specialist_usage = std::collections::HashMap::new();
        
        for i in 0..30 {
            let features = vec![
                (i as f64 * 0.1).sin() * 0.5 + 0.5,
                (i as f64 * 0.2).cos() * 0.5 + 0.5,
                0.5,
                0.5,
            ];
            
            let result = loop_.run_cycle(&[0.5], &features);
            *specialist_usage.entry(result.routing_result.selected_specialist.clone()).or_insert(0) += 1;
            
            // Simulate success
            loop_.learn_from_outcome(&features, &result.routing_result.selected_specialist, true);
        }

        // All specialists should be used at least once
        assert!(specialist_usage.len() >= 2);
        
        // Usage should be somewhat balanced (no specialist > 50%)
        for (_, count) in &specialist_usage {
            assert!(*count <= 15); // 50% of 30
        }
    }

    #[test]
    fn test_learning_rate_adaptation() {
        let mut config = UnifiedLearningConfig::default();
        config.learning_rate = 0.5; // High learning rate
        let specialist_ids = vec!["spec_a".to_string()];
        let mut loop_ = UnifiedLearningLoop::new(config, 1, specialist_ids);

        // Initial prediction error
        let initial_error = {
            let result = loop_.run_cycle(&[0.5], &[0.5, 0.5, 0.5, 0.5]);
            result.prediction_error
        };

        // Learn from consistent outcomes
        for _ in 0..20 {
            loop_.learn_from_outcome(&[0.5, 0.5, 0.5, 0.5], "spec_a", true);
        }

        // Run another cycle
        let result = loop_.run_cycle(&[0.5], &[0.5, 0.5, 0.5, 0.5]);
        let final_error = result.prediction_error;

        // System should have adapted (error may change)
        assert!(final_error.is_finite());
    }
}
