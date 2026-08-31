//! crates/biology/src/homeostasis.rs
//! Autonomous Multi-Factor Closed-Loop Feedback & Dynamic Equilibrium Governor
//! inspired by Linux cgroups v2 resource controllers and adaptive control theory.

use serde::{Deserialize, Serialize};

/// Current Dynamic Equilibrium State across the sovereign runtime
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicEquilibriumState {
    pub global_energy_reserve: f32, // 0.0 to 1000.0 tokens
    pub thermal_dissipation_rate: f32, // Tokens regenerated per second
    pub active_cognitive_load: f32,    // Current burn rate
    pub is_throttled: bool,
    pub throttle_factor: f32, // 1.0 = normal, 0.5 = 50% throttle, 0.0 = emergency halt
}

pub type HomeostasisState = DynamicEquilibriumState;

/// Master Feedback Regulator governing token budgets and runtime execution safety
pub struct FeedbackRegulator {
    state: DynamicEquilibriumState,
    max_energy_reserve: f32,
    overheat_threshold: f32,
}

pub type HomeostasisGovernor = FeedbackRegulator;

impl Default for FeedbackRegulator {
    fn default() -> Self {
        Self::new(1000.0, 50.0, 80.0)
    }
}

impl FeedbackRegulator {
    pub fn new(max_energy: f32, regen_rate: f32, overheat_threshold: f32) -> Self {
        Self {
            state: DynamicEquilibriumState {
                global_energy_reserve: max_energy,
                thermal_dissipation_rate: regen_rate,
                active_cognitive_load: 0.0,
                is_throttled: false,
                throttle_factor: 1.0,
            },
            max_energy_reserve: max_energy,
            overheat_threshold,
        }
    }

    pub fn state(&self) -> &DynamicEquilibriumState {
        &self.state
    }

    /// Deducts energy tokens for a cognitive task and checks for thermal exhaustion
    pub fn expend_energy(&mut self, token_cost: f32) -> bool {
        if self.state.global_energy_reserve >= token_cost {
            self.state.global_energy_reserve -= token_cost;
            self.state.active_cognitive_load += token_cost * 0.1;
            self.update_throttle_policy();
            true
        } else {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.0; // Complete throttle
            false
        }
    }

    /// Advances time and regenerates metabolic token reserve
    pub fn tick_regeneration(&mut self, delta_seconds: f32) {
        let regenerated = self.state.thermal_dissipation_rate * delta_seconds;
        self.state.global_energy_reserve = (self.state.global_energy_reserve + regenerated).min(self.max_energy_reserve);
        self.state.active_cognitive_load = (self.state.active_cognitive_load - (10.0 * delta_seconds)).max(0.0);
        self.update_throttle_policy();
    }

    /// Updates dynamic throttling policy based on cognitive load
    fn update_throttle_policy(&mut self) {
        if self.state.active_cognitive_load > self.overheat_threshold {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.5; // 50% reduction
        } else if self.state.global_energy_reserve < (self.max_energy_reserve * 0.1) {
            self.state.is_throttled = true;
            self.state.throttle_factor = 0.2; // 80% reduction
        } else {
            self.state.is_throttled = false;
            self.state.throttle_factor = 1.0; // Full speed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homeostasis_energy_expenditure_and_regeneration() {
        let mut gov = HomeostasisGovernor::new(100.0, 10.0, 50.0);
        assert_eq!(gov.state().global_energy_reserve, 100.0);
        assert!(!gov.state().is_throttled);

        // Expend 40 tokens
        let success = gov.expend_energy(40.0);
        assert!(success);
        assert_eq!(gov.state().global_energy_reserve, 60.0);

        // Regenerate for 2 seconds (+20 tokens)
        gov.tick_regeneration(2.0);
        assert_eq!(gov.state().global_energy_reserve, 80.0);
    }

    #[test]
    fn test_homeostasis_thermal_throttling() {
        let mut gov = HomeostasisGovernor::new(1000.0, 10.0, 30.0);

        // Trigger overheat (> 30.0 cognitive load)
        gov.expend_energy(350.0);
        assert!(gov.state().is_throttled);
        assert_eq!(gov.state().throttle_factor, 0.5);

        // Cool down
        gov.tick_regeneration(5.0);
        assert!(!gov.state().is_throttled);
        assert_eq!(gov.state().throttle_factor, 1.0);
    }
}
