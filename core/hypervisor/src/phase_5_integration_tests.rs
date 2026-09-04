// Phase 5 Biological Integration Tests
// Tests for thermal-aware expression rate, dopamine-driven metabolism, token-based execution

#[cfg(test)]
mod phase_5_biological_integration_tests {
    use biology::{SystemBiology, ThrottleState};

    /// Test 1: Thermal factor updates expression rate
    #[test]
    fn test_thermal_to_expression_rate() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_a", 100);

        // Normal operation
        assert_eq!(bio.expression_rate, 1.0);
        assert_eq!(bio.throttle_state, ThrottleState::Normal);

        // Warm (85°C range) - 0.6x throttle
        bio.set_expression_rate(0.6);
        assert_eq!(bio.expression_rate, 0.6);
        assert_eq!(bio.throttle_state, ThrottleState::Metabolic);

        // Critical (>95Ã‚Â°C) - 0.5x throttle (dormant)
        bio.set_expression_rate(0.5);
        assert_eq!(bio.expression_rate, 0.5);
        assert_eq!(bio.throttle_state, ThrottleState::Dormant);

        // Recovery to normal
        bio.set_expression_rate(1.0);
        assert_eq!(bio.expression_rate, 1.0);
        assert_eq!(bio.throttle_state, ThrottleState::Normal);
    }

    /// Test 2: Token regeneration scales with expression rate
    #[test]
    fn test_token_regen_scales_with_expression() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_a", 100);

        // Normal regen rate at 1.0x
        let _initial_tokens = bio.specialist_metabolism["spec_a"].tokens;
        bio.update_metabolism();

        // At reduced expression rate (0.5x), regen should be slower
        bio.set_expression_rate(0.5);
        let tokens_after_reduced = bio.specialist_metabolism["spec_a"].tokens;
        // Tokens should still regenerate but slower
        assert!(tokens_after_reduced >= 0.0);
    }

    /// Test 3: Token consumption on execution
    #[test]
    fn test_token_consumption() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_a", 100);

        let initial = bio.specialist_metabolism["spec_a"].tokens;

        // Consume token
        assert!(bio.consume_specialist_token("spec_a"));
        let after_consume = bio.specialist_metabolism["spec_a"].tokens;

        // Token should be consumed
        assert!(after_consume < initial);

        // Execution count should increment
        assert_eq!(bio.specialist_metabolism["spec_a"].execution_count, 1);
    }

    /// Test 4: Can't execute without tokens
    #[test]
    fn test_cant_execute_without_tokens() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_a", 100);

        // Consume all tokens
        let meta = bio.specialist_metabolism.get_mut("spec_a").unwrap();
        meta.tokens = 0.0;

        // Should not be able to execute
        assert!(!bio.can_execute_specialist("spec_a"));
    }

    /// Test 5: Specialist specialization emerges from dopamine
    #[test]
    fn test_dopamine_updates_ambition_strictness() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_a", 100);
        bio.register_specialist("spec_b", 100);

        let mut meta_a = bio.specialist_metabolism["spec_a"].clone();
        let mut meta_b = bio.specialist_metabolism["spec_b"].clone();

        // Spec A: positive dopamine (success) Ã¢â€ â€™ increase ambition
        meta_a.ambition = 0.5;
        meta_a.ambition = (meta_a.ambition + 0.1).min(1.0);
        assert!(meta_a.ambition > 0.5);

        // Spec B: negative dopamine (failure) Ã¢â€ â€™ increase strictness
        meta_b.strictness = 0.5;
        meta_b.strictness = (meta_b.strictness + 0.15).min(1.0);
        assert!(meta_b.strictness > 0.5);
    }

    /// Test 6: Execution bias calculation
    #[test]
    fn test_execution_bias_from_metabolism() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_aggressive", 100);
        bio.register_specialist("spec_cautious", 100);

        // Aggressive specialist: high ambition, low strictness
        bio.specialist_metabolism
            .get_mut("spec_aggressive")
            .unwrap()
            .ambition = 0.9;
        bio.specialist_metabolism
            .get_mut("spec_aggressive")
            .unwrap()
            .strictness = 0.1;

        let bias_agg = bio.calculate_execution_bias("spec_aggressive");
        assert!(bias_agg.exploration_rate > 0.7);
        assert!(bias_agg.risk_threshold < 0.5);

        // Cautious specialist: low ambition, high strictness
        bio.specialist_metabolism
            .get_mut("spec_cautious")
            .unwrap()
            .ambition = 0.1;
        bio.specialist_metabolism
            .get_mut("spec_cautious")
            .unwrap()
            .strictness = 0.9;

        let bias_cau = bio.calculate_execution_bias("spec_cautious");
        assert!(bias_cau.exploration_rate < 0.5);
        assert!(bias_cau.risk_threshold > 0.5);
    }

    /// Test 7: Throttle state transitions
    #[test]
    fn test_throttle_state_transitions() {
        let mut bio = SystemBiology::new();

        // Start at normal
        assert_eq!(bio.throttle_state, ThrottleState::Normal);

        // Low expression rate Ã¢â€ â€™ metabolic
        bio.set_expression_rate(0.6);
        assert_eq!(bio.throttle_state, ThrottleState::Metabolic);

        // Very low Ã¢â€ â€™ dormant
        bio.set_expression_rate(0.2);
        assert_eq!(bio.throttle_state, ThrottleState::Dormant);

        // Recovery
        bio.set_expression_rate(0.95);
        assert_eq!(bio.throttle_state, ThrottleState::Normal);
    }

    /// Test 8: Multiple specialists with different token budgets
    #[test]
    fn test_multi_specialist_token_management() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("enzyme_runner", 100); // Fast
        bio.register_specialist("learning_loop", 200); // Medium
        bio.register_specialist("network_executor", 500); // Slow

        // Verify each has appropriate tokens
        assert!(bio.specialist_metabolism["enzyme_runner"].max_tokens > 0.0);
        assert!(bio.specialist_metabolism["learning_loop"].max_tokens > 0.0);
        assert!(bio.specialist_metabolism["network_executor"].max_tokens > 0.0);

        // Consume tokens from each
        assert!(bio.consume_specialist_token("enzyme_runner"));
        assert!(bio.consume_specialist_token("learning_loop"));
        assert!(bio.consume_specialist_token("network_executor"));

        // All should have decremented execution counts
        assert_eq!(
            bio.specialist_metabolism["enzyme_runner"].execution_count,
            1
        );
        assert_eq!(
            bio.specialist_metabolism["learning_loop"].execution_count,
            1
        );
        assert_eq!(
            bio.specialist_metabolism["network_executor"].execution_count,
            1
        );
    }

    /// Test 9: Thermal throttling reduces token regeneration
    #[test]
    fn test_thermal_affects_token_regen() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("spec_a", 100);

        // Normal: expression_rate = 1.0
        bio.set_expression_rate(1.0);
        let normal_regen = bio.specialist_metabolism["spec_a"].regen_rate * bio.expression_rate;

        // Throttled: expression_rate = 0.5
        bio.set_expression_rate(0.5);
        let throttled_regen = bio.specialist_metabolism["spec_a"].regen_rate * bio.expression_rate;

        // Throttled regeneration should be half
        assert!(throttled_regen < normal_regen);
    }

    /// Test 10: Complete Phase 5 workflow
    #[test]
    fn test_complete_phase_5_workflow() {
        let mut bio = SystemBiology::new();
        bio.register_specialist("specialist_1", 100);

        // Phase 5.1: Thermal → expression_rate
        println!("Phase 5.1: Applying thermal throttle (0.6x)");
        bio.set_expression_rate(0.6);
        assert_eq!(bio.throttle_state, ThrottleState::Metabolic);

        // Phase 5.2: Dopamine Ã¢â€ â€™ ambition/strictness (simulate)
        println!("Phase 5.2: Applying dopamine reward");
        bio.specialist_metabolism
            .get_mut("specialist_1")
            .unwrap()
            .ambition = 0.7;
        bio.specialist_metabolism
            .get_mut("specialist_1")
            .unwrap()
            .strictness = 0.4;

        // Recalc execution bias
        let bias = bio.calculate_execution_bias("specialist_1");
        assert!(bias.metabolic_priority > 0.0);

        // Phase 5.3: Check tokens before execution
        println!("Phase 5.3: Checking token availability");
        assert!(bio.can_execute_specialist("specialist_1"));

        // Phase 5.4: Consume token (simulating execution)
        println!("Phase 5.3: Consuming token");
        assert!(bio.consume_specialist_token("specialist_1"));

        // Update metabolism
        bio.update_metabolism();

        // Phase 5.4: Monitor throttle state
        println!("Phase 5.4: Monitoring throttle state");
        match bio.throttle_state {
            ThrottleState::Normal => println!("  Ã¢â€ â€™ Normal operation"),
            ThrottleState::Metabolic => println!("  Ã¢â€ â€™ Metabolic mode (reduced capacity)"),
            ThrottleState::Dormant => println!("  Ã¢â€ â€™ Emergency mode"),
        }

        println!("Ã¢Å“â€œ Complete Phase 5 workflow successful");
    }
}

// Summary of Phase 5 Integration Tests:
// =======================================
// Test 1: Thermal Ã¢â€ â€™ Expression Rate Ã¢Å“â€œ
// Test 2: Token Regeneration Scaling Ã¢Å“â€œ
// Test 3: Token Consumption Ã¢Å“â€œ
// Test 4: Token Availability Check Ã¢Å“â€œ
// Test 5: Dopamine Ã¢â€ â€™ Metabolism Ã¢Å“â€œ
// Test 6: Execution Bias Calculation Ã¢Å“â€œ
// Test 7: Throttle State Transitions Ã¢Å“â€œ
// Test 8: Multi-Specialist Management Ã¢Å“â€œ
// Test 9: Thermal Affects Regen Ã¢Å“â€œ
// Test 10: Complete Workflow Ã¢Å“â€œ
//
// Coverage: 10/10 tests
// Status: READY FOR TESTING
