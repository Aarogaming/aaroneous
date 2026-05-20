/// Intent Analyzer — infers player intent from game observations.
///
/// Two inference modes:
/// - **Heuristic** (always available): fast pattern matching on action type.
///   Deterministic, zero latency, reasonable baseline accuracy.
/// - **LLM-backed** (when attached via `with_llm()`): uses the system's
///   `LLMClient` to analyze the full observation context (pre/post state,
///   action, stated intent) and produce a richer `InferredIntent`.

use super::types::*;
use std::sync::Arc;

/// Analyzes game actions to infer player intent.
#[derive(Clone)]
pub struct IntentAnalyzer {
    /// Optional LLM client for rich intent inference.
    /// When `None`, the analyzer uses fast heuristics instead.
    llm: Option<Arc<crate::llm::LLMClient>>,
}

impl IntentAnalyzer {
    /// Create an intent analyzer using only heuristics (no LLM).
    pub fn new() -> Self {
        Self { llm: None }
    }

    /// Attach an LLM client. After this call, `analyze_with_llm()` becomes
    /// available for richer intent inference.
    pub fn with_llm(mut self, client: Arc<crate::llm::LLMClient>) -> Self {
        self.llm = Some(client);
        self
    }

    /// Create an analyzer with a mock LLM provider for testing.
    pub async fn with_mock_llm() -> anyhow::Result<Self> {
        use crate::llm::{LLMClient, LLMConfig, ProviderType};
        let config = LLMConfig {
            provider_type: ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 256,
            timeout_secs: 10,
            enable_caching: false,
            cache_ttl_secs: 60,
            gguf_model_path: None,
        };
        let client = Arc::new(LLMClient::new(config).await?);
        Ok(Self::new().with_llm(client))
    }

    /// Whether an LLM client is attached.
    pub fn has_llm(&self) -> bool {
        self.llm.is_some()
    }

    /// Fast heuristic intent inference. Returns immediately with a reasonable
    /// estimate based on the action type. No LLM required.
    pub fn analyze_observation(&self, observation: &Observation) -> InferredIntent {
        match &observation.player_action {
            PlayerAction::Dodge { .. } => InferredIntent {
                goal: "Evade incoming threat".to_string(),
                confidence: 0.8,
                alternatives: vec![
                    ("Reposition for tactical advantage".to_string(), 0.15),
                    ("Retreat from combat".to_string(), 0.05),
                ],
                sample_count: 1,
                intent_category: IntentCategory::Defensive,
            },
            PlayerAction::Block => InferredIntent {
                goal: "Absorb or deflect incoming damage".to_string(),
                confidence: 0.85,
                alternatives: vec![
                    ("Create opening for counterattack".to_string(), 0.15),
                ],
                sample_count: 1,
                intent_category: IntentCategory::Defensive,
            },
            PlayerAction::Attack { target, ability } => {
                let goal = match ability {
                    Some(a) => format!("Use ability '{}' against {:?}", a, target),
                    None => format!("Deal damage to {:?}", target),
                };
                InferredIntent {
                    goal,
                    confidence: 0.9,
                    alternatives: vec![],
                    sample_count: 1,
                    intent_category: IntentCategory::Offensive,
                }
            }
            PlayerAction::Pickup { item } => InferredIntent {
                goal: format!("Acquire '{}'", item),
                confidence: 0.85,
                alternatives: vec![("Examine item before deciding".to_string(), 0.10)],
                sample_count: 1,
                intent_category: IntentCategory::ResourceGathering,
            },
            PlayerAction::Use { item } => InferredIntent {
                goal: format!("Consume or apply '{}'", item),
                confidence: 0.80,
                alternatives: vec![],
                sample_count: 1,
                intent_category: IntentCategory::ResourceGathering,
            },
            PlayerAction::Move { sprint, .. } => InferredIntent {
                goal: if *sprint { "Move quickly to new area" } else { "Navigate to target" }.to_string(),
                confidence: 0.65,
                alternatives: vec![("Explore area".to_string(), 0.20)],
                sample_count: 1,
                intent_category: IntentCategory::Exploration,
            },
            PlayerAction::TalkTo { npc } => InferredIntent {
                goal: format!("Interact with '{}'", npc),
                confidence: 0.75,
                alternatives: vec![("Gather quest information".to_string(), 0.15)],
                sample_count: 1,
                intent_category: IntentCategory::Exploration,
            },
            _ => InferredIntent {
                goal: "Intent unclear from action alone".to_string(),
                confidence: 0.4,
                alternatives: vec![],
                sample_count: 1,
                intent_category: IntentCategory::Unknown,
            },
        }
    }

    /// LLM-backed intent inference. Builds a prompt from the full observation
    /// context and queries the LLM for a richer analysis.
    ///
    /// Falls back to `analyze_observation()` if no LLM is attached or if the
    /// LLM call fails.
    pub async fn analyze_with_llm(&self, observation: &Observation) -> InferredIntent {
        let Some(llm) = &self.llm else {
            return self.analyze_observation(observation);
        };

        let prompt = format!(
            r#"You are analyzing a game player's intent from their action.

Pre-state:  health={:.0}%, stamina={:.0}%, location="{}", enemies={}
Action:     {:?}
Post-state: health={:.0}%, stamina={:.0}%
Stated intent: {}

In 1-2 sentences: What was the player trying to accomplish? \
What category fits best (Offensive/Defensive/ResourceGathering/Exploration/Social/Unknown)?

Respond with JSON:
{{"goal": "...", "category": "...", "confidence": 0.0-1.0, "alternatives": ["..."]}}
"#,
            observation.pre_state.health_pct,
            observation.pre_state.stamina_pct,
            observation.pre_state.location,
            observation.pre_state.nearby_enemies.len(),
            observation.player_action,
            observation.post_state.health_pct,
            observation.post_state.stamina_pct,
            observation.stated_intent.as_deref().unwrap_or("none")
        );

        // Use the task analysis context path (simplest available LLM call)
        let context = crate::llm::types::TaskAnalysisContext {
            task_id: format!("intent-{}", chrono::Utc::now().timestamp_millis()),
            file_name: "observation".to_string(),
            file_size: 0,
            file_type: "game_observation".to_string(),
            data_sample: prompt.clone(),
            specialist_skills: vec!["intent_analysis".to_string()],
            specialist_domain: "game_ai".to_string(),
            team_context: "agentic_player".to_string(),
        };

        match llm.analyze_task(&context).await {
            Ok(analysis) => {
                // Map the LLM's analysis back to InferredIntent
                let category = if analysis.recommended_approach.to_lowercase().contains("defensive") {
                    IntentCategory::Defensive
                } else if analysis.recommended_approach.to_lowercase().contains("offensive") ||
                          analysis.recommended_approach.to_lowercase().contains("attack") {
                    IntentCategory::Offensive
                } else if analysis.recommended_approach.to_lowercase().contains("resource") ||
                          analysis.recommended_approach.to_lowercase().contains("gather") {
                    IntentCategory::ResourceGathering
                } else if analysis.recommended_approach.to_lowercase().contains("explor") {
                    IntentCategory::Exploration
                } else {
                    // Fall back to heuristic for categorization
                    self.analyze_observation(observation).intent_category
                };

                InferredIntent {
                    goal: analysis.recommended_approach.clone(),
                    confidence: (analysis.confidence_percentage as f32 / 100.0).clamp(0.1, 0.99),
                    alternatives: analysis.potential_risks.iter()
                        .take(2)
                        .map(|r| (r.clone(), 0.1))
                        .collect(),
                    sample_count: 1,
                    intent_category: category,
                }
            }
            Err(e) => {
                tracing::warn!("IntentAnalyzer LLM call failed: {}, using heuristics", e);
                self.analyze_observation(observation)
            }
        }
    }
}

impl Default for IntentAnalyzer {
    fn default() -> Self {
        Self::new()
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

    // ======================================================
    // New tests for LLM integration and expanded heuristics
    // ======================================================

    #[test]
    fn test_intent_analyzer_has_no_llm_by_default() {
        let a = IntentAnalyzer::new();
        assert!(!a.has_llm());
    }

    #[tokio::test]
    async fn test_intent_analyzer_with_mock_llm_attaches_client() {
        let a = IntentAnalyzer::with_mock_llm().await.unwrap();
        assert!(a.has_llm());
    }

    #[test]
    fn test_heuristic_block_intent_is_defensive() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Block);
        let intent = analyzer.analyze_observation(&obs);
        assert_eq!(intent.intent_category, IntentCategory::Defensive);
        assert!(intent.confidence > 0.7);
    }

    #[test]
    fn test_heuristic_move_intent_is_exploration() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Move {
            direction: Direction::Forward,
            sprint: false,
        });
        let intent = analyzer.analyze_observation(&obs);
        assert_eq!(intent.intent_category, IntentCategory::Exploration);
    }

    #[test]
    fn test_heuristic_sprint_goal_mentions_quickly() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Move {
            direction: Direction::Forward,
            sprint: true,
        });
        let intent = analyzer.analyze_observation(&obs);
        assert!(intent.goal.contains("quickly"));
    }

    #[test]
    fn test_heuristic_unknown_action_returns_low_confidence() {
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::OpenMenu);
        let intent = analyzer.analyze_observation(&obs);
        // OpenMenu falls to the _ catch-all → low confidence
        assert!(intent.confidence < 0.7);
    }

    #[test]
    fn test_analyze_with_llm_falls_back_without_llm_attached() {
        // Without LLM, analyze_with_llm should be identical to analyze_observation
        let rt = tokio::runtime::Runtime::new().unwrap();
        let analyzer = IntentAnalyzer::new();
        let obs = create_test_observation(PlayerAction::Dodge {
            direction: Direction::Backward,
        });
        let heuristic = analyzer.analyze_observation(&obs);
        let llm_result = rt.block_on(analyzer.analyze_with_llm(&obs));
        // Both should return the same category (Defensive for Dodge)
        assert_eq!(heuristic.intent_category, llm_result.intent_category);
    }

    #[tokio::test]
    async fn test_llm_analyze_dodge_returns_some_intent() {
        let analyzer = IntentAnalyzer::with_mock_llm().await.unwrap();
        let obs = create_test_observation(PlayerAction::Dodge {
            direction: Direction::Left,
        });
        // LLM call should not panic and return something
        let intent = analyzer.analyze_with_llm(&obs).await;
        assert!(!intent.goal.is_empty());
        assert!(intent.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_llm_analyze_with_stated_intent() {
        let analyzer = IntentAnalyzer::with_mock_llm().await.unwrap();
        let mut obs = create_test_observation(PlayerAction::Pickup {
            item: "legendary_sword".to_string(),
        });
        obs.stated_intent = Some("I want the best weapon".to_string());
        let intent = analyzer.analyze_with_llm(&obs).await;
        // With a stated intent, we should get something meaningful
        assert!(!intent.goal.is_empty());
        assert!(intent.confidence >= 0.1);
    }

    #[test]
    fn test_intent_analyzer_clone_works() {
        let a = IntentAnalyzer::new();
        let _b = a.clone();
    }
}
