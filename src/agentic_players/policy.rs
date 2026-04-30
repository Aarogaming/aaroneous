/// Policy Builder - Builds learned player policy from observations
///
/// Accumulates observations and decision rules to build a comprehensive policy

use super::types::*;

/// Builds and updates the player policy
pub struct PolicyBuilder;

impl PolicyBuilder {
    /// Create a new policy from observations
    pub fn build_policy(observations: &[Observation]) -> PlayerPolicy {
        let mut policy = PlayerPolicy::new();
        policy.observations_total = observations.len();

        if observations.is_empty() {
            return policy;
        }

        // Extract playstyle from action patterns
        policy.playstyle = Self::infer_playstyle(observations);

        // Build decision rules
        policy.decision_rules = Self::extract_decision_rules(observations);

        // Calculate overall confidence
        policy.overall_confidence = Self::calculate_confidence(observations);

        policy
    }

    /// Infer playstyle from observation patterns
    fn infer_playstyle(observations: &[Observation]) -> PlayStyle {
        let mut dodge_count = 0;
        let mut attack_count = 0;
        let mut _move_count = 0;
        let mut item_count = 0;

        for obs in observations {
            match &obs.player_action {
                PlayerAction::Dodge { .. } => dodge_count += 1,
                PlayerAction::Attack { .. } => attack_count += 1,
                PlayerAction::Move { .. } => _move_count += 1,
                PlayerAction::Pickup { .. } | PlayerAction::Use { .. } => item_count += 1,
                _ => {}
            }
        }

        let total = observations.len() as f32;
        let dodge_ratio = dodge_count as f32 / total;
        let attack_ratio = attack_count as f32 / total;

        if dodge_ratio > 0.4 {
            PlayStyle::Defensive
        } else if attack_ratio > 0.4 {
            PlayStyle::Aggressive
        } else if item_count as f32 / total > 0.3 {
            PlayStyle::Efficient
        } else {
            PlayStyle::Balanced
        }
    }

    /// Extract decision rules from observations
    fn extract_decision_rules(_observations: &[Observation]) -> Vec<DecisionRule> {
        // Placeholder: will extract real rules from observations in Phase 6C.2
        vec![]
    }

    /// Calculate overall confidence in the policy
    fn calculate_confidence(observations: &[Observation]) -> f32 {
        // More observations = higher confidence
        let obs_count = observations.len() as f32;
        (obs_count / (obs_count + 100.0)).min(0.95) // Cap at 0.95
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_obs(action: PlayerAction) -> Observation {
        Observation {
            pre_state: GameState {
                health_pct: 100.0,
                stamina_pct: 100.0,
                location: "Test".to_string(),
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
            player_action: action,
            post_state: GameState {
                health_pct: 100.0,
                stamina_pct: 100.0,
                location: "Test".to_string(),
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
    fn test_build_empty_policy() {
        let policy = PolicyBuilder::build_policy(&[]);
        assert_eq!(policy.observations_total, 0);
        assert_eq!(policy.overall_confidence, 0.0);
    }

    #[test]
    fn test_infer_aggressive_playstyle() {
        let obs = vec![
            create_test_obs(PlayerAction::Attack {
                target: TargetType::Enemy,
                ability: None,
            }),
            create_test_obs(PlayerAction::Attack {
                target: TargetType::Enemy,
                ability: None,
            }),
            create_test_obs(PlayerAction::Move {
                direction: Direction::Forward,
                sprint: true,
            }),
        ];

        let policy = PolicyBuilder::build_policy(&obs);
        assert_eq!(policy.playstyle, PlayStyle::Aggressive);
    }

    #[test]
    fn test_infer_defensive_playstyle() {
        let obs = vec![
            create_test_obs(PlayerAction::Dodge {
                direction: Direction::Left,
            }),
            create_test_obs(PlayerAction::Dodge {
                direction: Direction::Right,
            }),
            create_test_obs(PlayerAction::Block),
        ];

        let policy = PolicyBuilder::build_policy(&obs);
        assert_eq!(policy.playstyle, PlayStyle::Defensive);
    }

    #[test]
    fn test_confidence_increases_with_observations() {
        let single_obs = vec![create_test_obs(PlayerAction::Idle)];
        let policy1 = PolicyBuilder::build_policy(&single_obs);

        let many_obs: Vec<_> = (0..100)
            .map(|_| create_test_obs(PlayerAction::Idle))
            .collect();
        let policy2 = PolicyBuilder::build_policy(&many_obs);

        assert!(policy2.overall_confidence > policy1.overall_confidence);
    }

    #[test]
    fn test_observation_count_recorded() {
        let obs = vec![
            create_test_obs(PlayerAction::Idle),
            create_test_obs(PlayerAction::Idle),
            create_test_obs(PlayerAction::Idle),
        ];

        let policy = PolicyBuilder::build_policy(&obs);
        assert_eq!(policy.observations_total, 3);
    }
}
