//! crates/governance/src/resource_governor.rs
//! Autonomous Multi-Factor Closed-Loop Feedback & Dynamic Equilibrium Governor
//! inspired by Linux cgroups v2 resource controllers and adaptive control theory.

use serde::{Deserialize, Serialize};

/// Operational degradation tier under resource stress
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DegradationTier {
    /// Full capabilities enabled
    Nominal = 0,
    /// Non-essential background tasks slowed (50% throttle)
    SoftThrottle = 1,
    /// Background tasks paused, speculative execution disabled (80% throttle)
    HeavyThrottle = 2,
    /// Runtime frozen to prevent out-of-memory or resource runaway (100% halt)
    EmergencyHalt = 3,
}

/// Current Dynamic Equilibrium State across the sovereign runtime
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DynamicEquilibriumState {
    pub global_token_reserve: f32, // 0.0 to max tokens
    pub token_regen_rate: f32,     // Tokens regenerated per second
    pub active_compute_load: f32,  // Current burn rate
    pub memory_pressure_mb: f32,   // Tracked memory allocation footprint
    pub is_throttled: bool,
    pub throttle_factor: f32, // 1.0 = normal, 0.5 = 50% throttle, 0.0 = emergency halt
    pub degradation_tier: DegradationTier,
    pub total_tasks_executed: u64,
}

/// Master Feedback Regulator governing token budgets and runtime execution safety
#[derive(Debug, Clone)]
pub struct FeedbackRegulator {
    state: DynamicEquilibriumState,
    max_token_reserve: f32,
    overheat_threshold: f32,
    max_memory_mb: f32,
}

pub type ResourceGovernor = FeedbackRegulator;

impl Default for FeedbackRegulator {
    fn default() -> Self {
        Self::new(1000.0, 50.0, 80.0)
    }
}

impl FeedbackRegulator {
    pub fn new(max_tokens: f32, regen_rate: f32, overheat_threshold: f32) -> Self {
        Self {
            state: DynamicEquilibriumState {
                global_token_reserve: max_tokens,
                token_regen_rate: regen_rate,
                active_compute_load: 0.0,
                memory_pressure_mb: 0.0,
                is_throttled: false,
                throttle_factor: 1.0,
                degradation_tier: DegradationTier::Nominal,
                total_tasks_executed: 0,
            },
            max_token_reserve: max_tokens,
            overheat_threshold,
            max_memory_mb: 4096.0,
        }
    }

    pub fn with_memory_limit(mut self, max_mb: f32) -> Self {
        self.max_memory_mb = max_mb.max(128.0);
        self
    }

    pub fn state(&self) -> &DynamicEquilibriumState {
        &self.state
    }

    /// Checks if a task with estimated token cost can be admitted without breaching safety
    pub fn can_admit_task(&self, estimated_cost: f32) -> bool {
        if self.state.degradation_tier == DegradationTier::EmergencyHalt {
            return false;
        }
        self.state.global_token_reserve >= estimated_cost
    }

    /// Records memory footprint changes (MB)
    pub fn update_memory_pressure(&mut self, current_mb: f32) {
        self.state.memory_pressure_mb = current_mb.max(0.0);
        self.update_throttle_policy();
    }

    /// Deducts tokens for a task and checks for resource exhaustion
    pub fn expend_tokens(&mut self, token_cost: f32) -> bool {
        if self.state.global_token_reserve >= token_cost {
            self.state.global_token_reserve -= token_cost;
            self.state.active_compute_load += token_cost * 0.1;
            self.state.total_tasks_executed += 1;
            self.update_throttle_policy();
            true
        } else {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.0;
            self.state.degradation_tier = DegradationTier::EmergencyHalt;
            false
        }
    }

    /// Advances time and regenerates token reserve
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        let regenerated = self.state.token_regen_rate * dt;
        self.state.global_token_reserve =
            (self.state.global_token_reserve + regenerated).min(self.max_token_reserve);
        self.state.active_compute_load = (self.state.active_compute_load - (10.0 * dt)).max(0.0);
        self.update_throttle_policy();
    }

    /// Returns true if equilibrium is in nominal operational state
    pub fn is_healthy(&self) -> bool {
        !self.state.is_throttled && self.state.degradation_tier == DegradationTier::Nominal
    }

    /// Updates dynamic throttling policy based on compute load and memory pressure
    fn update_throttle_policy(&mut self) {
        let memory_critical = self.state.memory_pressure_mb > self.max_memory_mb * 0.95;
        let memory_warning = self.state.memory_pressure_mb > self.max_memory_mb * 0.80;

        if memory_critical || self.state.global_token_reserve <= 0.0 {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.0;
            self.state.degradation_tier = DegradationTier::EmergencyHalt;
        } else if memory_warning || self.state.global_token_reserve < (self.max_token_reserve * 0.1)
        {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.2; // 80% reduction
            self.state.degradation_tier = DegradationTier::HeavyThrottle;
        } else if self.state.active_compute_load > self.overheat_threshold {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.5; // 50% reduction
            self.state.degradation_tier = DegradationTier::SoftThrottle;
        } else {
            self.state.is_throttled = false;
            self.state.throttle_factor = 1.0; // Full speed
            self.state.degradation_tier = DegradationTier::Nominal;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_expenditure_and_regeneration() {
        let mut gov = FeedbackRegulator::new(100.0, 10.0, 50.0);
        assert_eq!(gov.state().global_token_reserve, 100.0);
        assert!(!gov.state().is_throttled);
        assert!(gov.is_healthy());

        // Expend 40 tokens
        let success = gov.expend_tokens(40.0);
        assert!(success);
        assert_eq!(gov.state().global_token_reserve, 60.0);
        assert_eq!(gov.state().total_tasks_executed, 1);

        // Regenerate for 2 seconds (+20 tokens)
        gov.tick(2.0);
        assert_eq!(gov.state().global_token_reserve, 80.0);
    }

    #[test]
    fn test_throttling() {
        let mut gov = FeedbackRegulator::new(1000.0, 10.0, 30.0);

        // Trigger overheat (> 30.0 compute load)
        gov.expend_tokens(350.0);
        assert!(gov.state().is_throttled);
        assert_eq!(gov.state().throttle_factor, 0.5);
        assert_eq!(gov.state().degradation_tier, DegradationTier::SoftThrottle);

        // Cool down
        gov.tick(5.0);
        assert!(!gov.state().is_throttled);
        assert_eq!(gov.state().throttle_factor, 1.0);
        assert_eq!(gov.state().degradation_tier, DegradationTier::Nominal);
    }

    #[test]
    fn test_memory_pressure_and_admission() {
        let mut gov = FeedbackRegulator::new(1000.0, 10.0, 50.0).with_memory_limit(1000.0);
        assert!(gov.can_admit_task(100.0));

        gov.update_memory_pressure(850.0);
        assert_eq!(gov.state().degradation_tier, DegradationTier::HeavyThrottle);

        gov.update_memory_pressure(960.0);
        assert_eq!(gov.state().degradation_tier, DegradationTier::EmergencyHalt);
        assert!(!gov.can_admit_task(10.0));
    }
}
