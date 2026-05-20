/// Emulation Agent - Executes learned policy with humanization
///
/// Takes a learned PlayerPolicy and generates actions that emulate the player,
/// including human-like delays, jitter, and decision patterns

use super::types::*;

/// Executes a learned policy to emulate player behavior
#[derive(Clone)]
pub struct EmulationAgent {
    /// The policy to emulate
    policy: PlayerPolicy,
    
    /// Statistics about this emulation
    stats: EmulationStats,
}

impl EmulationAgent {
    /// Create new emulation agent from policy
    pub fn new(policy: PlayerPolicy) -> Self {
        Self {
            policy,
            stats: EmulationStats {
                actions_taken: 0,
                successful_actions: 0,
                success_rate: 0.0,
                objectives_completed: 0,
                times_interrupted: 0,
                policy_deviations: 0,
                duration: std::time::Duration::from_secs(0),
            },
        }
    }

    /// Get the next action for a condition
    pub fn get_next_action(&self, condition: &GameCondition) -> Option<PlayerAction> {
        self.policy.recommend_action(condition)
    }

    /// Apply humanization to an action (delays, jitter)
    pub fn humanize_action(&self, action: &PlayerAction) -> (PlayerAction, u64) {
        let delay = self.calculate_reaction_delay();
        (action.clone(), delay)
    }

    /// Calculate reaction delay based on humanization patterns
    fn calculate_reaction_delay(&self) -> u64 {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        // Simple deterministic "randomness" for testing
        let hasher = RandomState::new();
        let mut s = hasher.build_hasher();
        chrono::Utc::now().hash(&mut s);
        let rand_val = (s.finish() % 100) as i32;

        let variance = self.policy.humanization.reaction_variance_ms as i32;
        let base = self.policy.humanization.reaction_delay_ms as i32;
        
        let delay = base + (rand_val % variance) - (variance / 2);
        (delay.max(0) as u64).min(1000) // Clamp to reasonable range
    }

    /// Get current statistics
    pub fn get_stats(&self) -> &EmulationStats {
        &self.stats
    }

    /// Get the underlying policy
    pub fn get_policy(&self) -> &PlayerPolicy {
        &self.policy
    }

    /// Update playstyle for this session
    pub fn set_playstyle(&mut self, playstyle: PlayStyle) {
        self.policy.playstyle = playstyle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_policy() -> PlayerPolicy {
        PlayerPolicy::new()
    }

    #[test]
    fn test_emulation_agent_creation() {
        let policy = create_test_policy();
        let agent = EmulationAgent::new(policy);

        assert_eq!(agent.get_stats().actions_taken, 0);
    }

    #[test]
    fn test_humanization_delay_is_reasonable() {
        let policy = create_test_policy();
        let agent = EmulationAgent::new(policy);

        let delay = agent.calculate_reaction_delay();
        assert!(delay < 1000); // Should be under 1 second
        assert!(delay > 0);    // Should be non-zero
    }

    #[test]
    fn test_action_humanization() {
        let policy = create_test_policy();
        let agent = EmulationAgent::new(policy);

        let action = PlayerAction::Idle;
        let (humanized_action, delay) = agent.humanize_action(&action);

        assert_eq!(humanized_action, action);
        assert!(delay > 0);
    }

    #[test]
    fn test_playstyle_modification() {
        let policy = create_test_policy();
        let mut agent = EmulationAgent::new(policy);

        assert_eq!(agent.get_policy().playstyle, PlayStyle::Balanced);

        agent.set_playstyle(PlayStyle::Aggressive);
        assert_eq!(agent.get_policy().playstyle, PlayStyle::Aggressive);
    }

    #[test]
    fn test_get_next_action_empty_policy() {
        let policy = create_test_policy();
        let agent = EmulationAgent::new(policy);

        let condition = GameCondition {
            health_below_pct: Some(50),
            enemies_nearby_count: None,
            location_matches: None,
            item_nearby: None,
            custom: None,
        };

        // Should return None since policy has no rules
        assert!(agent.get_next_action(&condition).is_none());
    }
}
