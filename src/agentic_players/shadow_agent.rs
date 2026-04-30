/// Shadow Agent - Observes and records player behavior
///
/// The Shadow Agent runs in the background while the player games, recording:
/// - All inputs (keyboard, mouse)
/// - Resulting game state changes
/// - Inferred intent behind actions

use super::types::*;
use std::sync::Arc;
use std::collections::VecDeque;

/// Captures continuous player behavior
#[derive(Clone)]
pub struct ShadowAgent {
    /// Rolling buffer of observations
    observation_log: Arc<parking_lot::RwLock<VecDeque<Observation>>>,
    
    /// Maximum observations to keep in memory
    max_observations: usize,
}

impl ShadowAgent {
    /// Create new shadow agent
    pub fn new(max_observations: usize) -> Self {
        Self {
            observation_log: Arc::new(parking_lot::RwLock::new(VecDeque::new())),
            max_observations,
        }
    }

    /// Record an observation
    pub fn record_observation(&self, observation: Observation) {
        let mut log = self.observation_log.write();
        
        log.push_back(observation);
        
        // Keep only recent observations
        while log.len() > self.max_observations {
            log.pop_front();
        }
    }

    /// Get all recent observations
    pub fn get_observations(&self) -> Vec<Observation> {
        self.observation_log.read().iter().cloned().collect()
    }

    /// Get observation count
    pub fn observation_count(&self) -> usize {
        self.observation_log.read().len()
    }

    /// Clear all observations
    pub fn clear(&self) {
        self.observation_log.write().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_observation() -> Observation {
        Observation {
            pre_state: GameState {
                health_pct: 100.0,
                stamina_pct: 100.0,
                location: "Forest".to_string(),
                nearby_enemies: vec![],
                available_items: vec![],
                current_objective: None,
                inventory_summary: InventorySummary {
                    weapon_equipped: None,
                    armor_equipped: None,
                    quick_slots: vec![],
                    total_items: 0,
                },
            },
            player_action: PlayerAction::Move {
                direction: Direction::Forward,
                sprint: false,
            },
            post_state: GameState {
                health_pct: 100.0,
                stamina_pct: 95.0,
                location: "Forest".to_string(),
                nearby_enemies: vec![],
                available_items: vec![],
                current_objective: None,
                inventory_summary: InventorySummary {
                    weapon_equipped: None,
                    armor_equipped: None,
                    quick_slots: vec![],
                    total_items: 0,
                },
            },
            duration_ms: 1000,
            stated_intent: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_shadow_agent_creation() {
        let agent = ShadowAgent::new(1000);
        assert_eq!(agent.observation_count(), 0);
    }

    #[test]
    fn test_record_observation() {
        let agent = ShadowAgent::new(1000);
        let obs = create_test_observation();

        agent.record_observation(obs);
        assert_eq!(agent.observation_count(), 1);
    }

    #[test]
    fn test_multiple_observations() {
        let agent = ShadowAgent::new(100);

        for _ in 0..50 {
            agent.record_observation(create_test_observation());
        }

        assert_eq!(agent.observation_count(), 50);
    }

    #[test]
    fn test_observation_buffer_limit() {
        let agent = ShadowAgent::new(10);

        for _ in 0..20 {
            agent.record_observation(create_test_observation());
        }

        // Should only keep 10 most recent
        assert_eq!(agent.observation_count(), 10);
    }

    #[test]
    fn test_get_observations() {
        let agent = ShadowAgent::new(100);
        let obs = create_test_observation();

        agent.record_observation(obs.clone());
        agent.record_observation(obs.clone());

        let retrieved = agent.get_observations();
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_clear_observations() {
        let agent = ShadowAgent::new(100);

        for _ in 0..10 {
            agent.record_observation(create_test_observation());
        }

        assert!(agent.observation_count() > 0);
        agent.clear();
        assert_eq!(agent.observation_count(), 0);
    }
}
