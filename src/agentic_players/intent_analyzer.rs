/// Intent Analyzer - Infers player intent from observations using LLM
///
/// Analyzes observations to understand the "why" behind player actions

use super::types::*;
use std::sync::Arc;

/// Analyzes game actions to infer player intent
#[derive(Clone)]
pub struct IntentAnalyzer {
    /// LLM client for intent inference
    /// Will be connected to actual LLM in Phase 6C.2
    _placeholder: Arc<std::sync::Mutex<()>>,
}

impl IntentAnalyzer {
    /// Create new intent analyzer
    pub fn new() -> Self {
        Self {
            _placeholder: Arc::new(std::sync::Mutex::new(())),
        }
    }

    /// Analyze an observation to infer intent
    pub fn analyze_observation(
        &self,
        observation: &Observation,
    ) -> InferredIntent {
        // Placeholder: Simple heuristic-based intent inference
        // In Phase 6C.2, this will use LLM
        
        match &observation.player_action {
            PlayerAction::Dodge { .. } => {
                InferredIntent {
                    goal: "Evade incoming threat".to_string(),
                    confidence: 0.8,
                    alternatives: vec![
                        ("Move to better positioning".to_string(), 0.15),
                        ("Chase enemy".to_string(), 0.05),
                    ],
                    sample_count: 1,
                    intent_category: IntentCategory::Defensive,
                }
            }
            PlayerAction::Attack { .. } => {
                InferredIntent {
                    goal: "Defeat enemy".to_string(),
                    confidence: 0.9,
                    alternatives: vec![],
                    sample_count: 1,
                    intent_category: IntentCategory::Offensive,
                }
            }
            PlayerAction::Pickup { item } => {
                InferredIntent {
                    goal: format!("Obtain {}", item),
                    confidence: 0.85,
                    alternatives: vec![
                        ("Examine item".to_string(), 0.10),
                        ("Move item out of the way".to_string(), 0.05),
                    ],
                    sample_count: 1,
                    intent_category: IntentCategory::ResourceGathering,
                }
            }
            _ => {
                InferredIntent {
                    goal: "Unknown intent".to_string(),
                    confidence: 0.5,
                    alternatives: vec![],
                    sample_count: 1,
                    intent_category: IntentCategory::Unknown,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_observation(action: PlayerAction) -> Observation {
        Observation {
            pre_state: GameState {
                health_pct: 80.0,
                stamina_pct: 100.0,
                location: "Dungeon".to_string(),
                nearby_enemies: vec![],
                available_items: vec![],
                current_objective: None,
                inventory_summary: InventorySummary {
                    weapon_equipped: Some("Sword".to_string()),
                    armor_equipped: Some("Armor".to_string()),
                    quick_slots: vec![],
                    total_items: 5,
                },
            },
            player_action: action,
            post_state: GameState {
                health_pct: 80.0,
                stamina_pct: 100.0,
                location: "Dungeon".to_string(),
                nearby_enemies: vec![],
                available_items: vec![],
                current_objective: None,
                inventory_summary: InventorySummary {
                    weapon_equipped: Some("Sword".to_string()),
                    armor_equipped: Some("Armor".to_string()),
                    quick_slots: vec![],
                    total_items: 5,
                },
            },
            duration_ms: 500,
            stated_intent: None,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_intent_analyzer_creation() {
        let analyzer = IntentAnalyzer::new();
        // Just verify it can be created
        let _ = analyzer.clone();
    }

    #[test]
    fn test_infer_dodge_intent() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Dodge {
            direction: Direction::Left,
        });

        let intent = analyzer.analyze_observation(&obs);
        assert_eq!(intent.intent_category, IntentCategory::Defensive);
        assert!(intent.confidence > 0.7);
    }

    #[test]
    fn test_infer_attack_intent() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Attack {
            target: TargetType::Enemy,
            ability: None,
        });

        let intent = analyzer.analyze_observation(&obs);
        assert_eq!(intent.intent_category, IntentCategory::Offensive);
        assert!(intent.confidence > 0.8);
    }

    #[test]
    fn test_infer_pickup_intent() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Pickup {
            item: "HealthPotion".to_string(),
        });

        let intent = analyzer.analyze_observation(&obs);
        assert_eq!(intent.intent_category, IntentCategory::ResourceGathering);
    }
}
