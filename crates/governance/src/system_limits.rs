// Aaroneous Metabolic Biology Module
// Token-bucket expression rate governance with per-specialist metabolism

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use strum::{Display, EnumIter, EnumString};

/// Global resource & execution rate governor for the entire Aaroneous runtime
pub struct SystemHealthGovernor {
    pub expression_rate: f32, // Global multiplier for all specialist rates (0.0-1.0)
    pub tokens: f32,          // Global token pool
    pub last_regen: Instant,
    pub specialist_metabolism: HashMap<String, SpecialistMetabolism>,
    pub throttle_state: ThrottleState,
}

pub type SystemBiology = SystemHealthGovernor;

/// Per-specialist execution budget and token allocation
#[derive(Debug, Clone)]
pub struct SpecialistExecutionBudget {
    pub specialist_id: String,
    pub tokens: f32,     // Individual token allocation
    pub max_tokens: f32, // Token ceiling for this specialist
    pub regen_rate: f32, // Tokens/sec for this specialist
    pub last_regen: Instant,
    pub execution_count: u64, // Number of times this specialist has executed

    // Tension Spectrums (Control Goals)
    pub ambition: f32,   // Goal-seeking drive (0.0-1.0)
    pub strictness: f32, // Compliance/Audit drive (0.0-1.0)
    pub stability: f32,  // Risk-aversion drive (0.0-1.0)
}

pub type SpecialistMetabolism = SpecialistExecutionBudget;

impl Default for SpecialistMetabolism {
    fn default() -> Self {
        Self {
            specialist_id: String::new(),
            tokens: 0.0,
            max_tokens: 0.0,
            regen_rate: 0.0,
            last_regen: Instant::now(),
            execution_count: 0,
            ambition: 0.0,
            strictness: 0.0,
            stability: 0.0,
        }
    }
}

/// Global throttle state
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString, EnumIter,
)]
pub enum ThrottleState {
    #[strum(serialize = "normal")]
    Normal,
    #[strum(serialize = "metabolic")]
    Metabolic, // Expression rate reduced but functional
    #[strum(serialize = "dormant")]
    Dormant, // Emergency throttle: expression_rate near 0
}

impl Default for SystemBiology {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemBiology {
    /// Initialize base Aaroneous metabolism
    pub fn new() -> Self {
        SystemBiology {
            expression_rate: 1.0,
            tokens: 100.0,
            last_regen: Instant::now(),
            specialist_metabolism: HashMap::new(),
            throttle_state: ThrottleState::Normal,
        }
    }

    /// Register a specialist in the metabolic system
    pub fn register_specialist(&mut self, specialist_id: &str, interval_ms: u64) {
        let regen_rate = (1000.0 / interval_ms as f32) * self.expression_rate;
        let max_tokens = (regen_rate * 10.0).max(2.0);

        self.specialist_metabolism.insert(
            specialist_id.to_string(),
            SpecialistMetabolism {
                specialist_id: specialist_id.to_string(),
                tokens: max_tokens,
                max_tokens,
                regen_rate,
                last_regen: Instant::now(),
                execution_count: 0,
                ambition: 0.5,
                strictness: 0.5,
                stability: 0.5,
            },
        );
    }

    /// Calculates the Execution Bias (Tension) for a specialist.
    /// Derived from legacy Fabricator TensionEngine logic.
    pub fn calculate_execution_bias(&self, specialist_id: &str) -> ExecutionBias {
        if let Some(meta) = self.specialist_metabolism.get(specialist_id) {
            let tension_factor = (meta.strictness - meta.ambition).clamp(-1.0, 1.0);

            ExecutionBias {
                risk_threshold: (0.5 + (tension_factor * 0.4)).clamp(0.1, 0.9),
                exploration_rate: (0.5 - (tension_factor * 0.4)).clamp(0.1, 0.9),
                metabolic_priority: (meta.ambition * self.expression_rate).clamp(0.0, 1.0),
            }
        } else {
            ExecutionBias::default()
        }
    }

    /// Update global metabolism and regenerate tokens
    pub fn update_metabolism(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_regen).as_secs_f32();

        // Regenerate global tokens
        let global_regen_rate = 1.0 * self.expression_rate;
        self.tokens = (self.tokens + elapsed * global_regen_rate).min(100.0);
        self.last_regen = now;

        // Update specialist metabolism
        for (_, metabolism) in self.specialist_metabolism.iter_mut() {
            let elapsed_spec = now.duration_since(metabolism.last_regen).as_secs_f32();
            let spec_regen = metabolism.regen_rate * self.expression_rate;
            metabolism.tokens =
                (metabolism.tokens + elapsed_spec * spec_regen).min(metabolism.max_tokens);
            metabolism.last_regen = now;
        }

        // Update throttle state based on token availability
        self.update_throttle_state();
    }

    /// Check if a specialist can execute (token availability)
    pub fn can_execute_specialist(&self, specialist_id: &str) -> bool {
        if let Some(metabolism) = self.specialist_metabolism.get(specialist_id) {
            metabolism.tokens >= 1.0
        } else {
            false
        }
    }

    /// Consume a token from specialist's pool
    pub fn consume_specialist_token(&mut self, specialist_id: &str) -> bool {
        if let Some(metabolism) = self.specialist_metabolism.get_mut(specialist_id) {
            if metabolism.tokens >= 1.0 {
                metabolism.tokens -= 1.0;
                metabolism.execution_count += 1;
                return true;
            }
        }
        false
    }

    /// Adjust global expression rate (affects all specialists proportionally)
    pub fn set_expression_rate(&mut self, new_rate: f32) {
        let clamped_rate = new_rate.clamp(0.0, 1.0);
        self.expression_rate = clamped_rate;

        // Recalculate specialist regen rates based on new global rate
        for (_, metabolism) in self.specialist_metabolism.iter_mut() {
            // Preserve relative distribution, adjust absolute rates
            metabolism.max_tokens = (metabolism.regen_rate * 10.0 * clamped_rate).max(1.0);
        }

        // Determine throttle state based on expression rate
        if clamped_rate < 0.6 {
            self.throttle_state = ThrottleState::Dormant;
        } else if clamped_rate < 0.7 {
            self.throttle_state = ThrottleState::Metabolic;
        } else {
            self.throttle_state = ThrottleState::Normal;
        }
    }

    /// Request a mutation from the AutoFabricator (Forge)
    pub fn request_mutation(&self, specialist_id: &str, goal: &str) {
        tracing::warn!(
            "Metabolic Governor: Stagnation detected for {}. Requesting evolution for goal: {}",
            specialist_id,
            goal
        );
        // In the unified core, this will eventually trigger the AutoFabricator service
    }

    /// Update throttle state based on token availability
    fn update_throttle_state(&mut self) {
        let avg_availability = if self.specialist_metabolism.is_empty() {
            1.0
        } else {
            let total_tokens: f32 = self
                .specialist_metabolism
                .values()
                .map(|m| m.tokens / m.max_tokens)
                .sum();
            total_tokens / self.specialist_metabolism.len() as f32
        };

        if avg_availability < 0.2 {
            self.throttle_state = ThrottleState::Dormant;
        } else if avg_availability < 0.5 {
            self.throttle_state = ThrottleState::Metabolic;
        } else {
            self.throttle_state = ThrottleState::Normal;
        }
    }

    /// Get specialist's current metabolism state
    pub fn get_specialist_metabolism(&self, specialist_id: &str) -> Option<&SpecialistMetabolism> {
        self.specialist_metabolism.get(specialist_id)
    }

    /// Get system health report
    pub fn get_health_report(&self) -> SystemHealthReport {
        let specialist_health: Vec<_> = self
            .specialist_metabolism
            .values()
            .map(|m| SpecialistHealth {
                specialist_id: m.specialist_id.clone(),
                tokens: m.tokens,
                max_tokens: m.max_tokens,
                execution_count: m.execution_count,
                token_availability: m.tokens / m.max_tokens,
            })
            .collect();

        SystemHealthReport {
            global_tokens: self.tokens,
            expression_rate: self.expression_rate,
            throttle_state: self.throttle_state,
            specialist_count: self.specialist_metabolism.len(),
            specialist_health,
        }
    }
}

/// Health report structure for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthReport {
    pub global_tokens: f32,
    pub expression_rate: f32,
    pub throttle_state: ThrottleState,
    pub specialist_count: usize,
    pub specialist_health: Vec<SpecialistHealth>,
}

/// Individual specialist health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistHealth {
    pub specialist_id: String,
    pub tokens: f32,
    pub max_tokens: f32,
    pub execution_count: u64,
    pub token_availability: f32,
}

/// Bias configuration for execution logic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionBias {
    pub risk_threshold: f32,
    pub exploration_rate: f32,
    pub metabolic_priority: f32,
}

impl Default for ExecutionBias {
    fn default() -> Self {
        Self {
            risk_threshold: 0.5,
            exploration_rate: 0.5,
            metabolic_priority: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_registration() {
        let mut biology = SystemBiology::new();
        biology.register_specialist("specialist_presenter", 20000);
        assert!(biology
            .get_specialist_metabolism("specialist_presenter")
            .is_some());
    }

    #[test]
    fn test_token_consumption() {
        let mut biology = SystemBiology::new();
        biology.register_specialist("test_specialist", 30000);
        assert!(biology.can_execute_specialist("test_specialist"));
        assert!(biology.consume_specialist_token("test_specialist"));
        assert!(biology.can_execute_specialist("test_specialist"));
    }

    #[test]
    fn test_expression_rate_clamping() {
        let mut biology = SystemBiology::new();
        biology.set_expression_rate(1.5);
        assert_eq!(biology.expression_rate, 1.0);

        biology.set_expression_rate(-0.5);
        assert_eq!(biology.expression_rate, 0.0);
    }

    #[test]
    fn test_throttle_state_transitions() {
        let mut biology = SystemBiology::new();
        assert_eq!(biology.throttle_state, ThrottleState::Normal);

        biology.set_expression_rate(0.65);
        assert_eq!(biology.throttle_state, ThrottleState::Metabolic);

        biology.set_expression_rate(0.5);
        assert_eq!(biology.throttle_state, ThrottleState::Dormant);
    }
}
