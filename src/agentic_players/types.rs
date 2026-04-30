/// Core types for agentic player system

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::time::Duration;
use std::collections::HashMap;

/// Unique identifier for an agent
pub type AgentId = String;

/// A single observation of player behavior
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    /// Game state before the action
    pub pre_state: GameState,
    
    /// Action taken by player
    pub player_action: PlayerAction,
    
    /// Game state after the action
    pub post_state: GameState,
    
    /// Time spent on this action (milliseconds)
    pub duration_ms: u64,
    
    /// Optional: player's stated intent (via voice/chat)
    pub stated_intent: Option<String>,
    
    /// When was this recorded?
    pub timestamp: DateTime<Utc>,
}

/// Player's game state at a moment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameState {
    /// Player health percentage (0-100)
    pub health_pct: f32,
    
    /// Player mana/stamina percentage
    pub stamina_pct: f32,
    
    /// Current location in game
    pub location: String,
    
    /// Nearby enemies (count and types)
    pub nearby_enemies: Vec<EnemyInfo>,
    
    /// Items on ground or in range
    pub available_items: Vec<ItemInfo>,
    
    /// Current objective or quest
    pub current_objective: Option<String>,
    
    /// Inventory summary
    pub inventory_summary: InventorySummary,
}

/// Information about nearby enemy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyInfo {
    pub enemy_type: String,
    pub health_pct: f32,
    pub distance: f32,
    pub threat_level: ThreatLevel,
}

/// Threat level of an enemy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ThreatLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Information about available item
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemInfo {
    pub item_type: String,
    pub rarity: Rarity,
    pub distance: f32,
}

/// Item rarity level
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

/// Player's inventory at a moment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventorySummary {
    pub weapon_equipped: Option<String>,
    pub armor_equipped: Option<String>,
    pub quick_slots: Vec<Option<String>>,
    pub total_items: usize,
}

/// Action taken by player
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PlayerAction {
    /// Movement
    Move { direction: Direction, sprint: bool },
    
    /// Combat
    Attack { target: TargetType, ability: Option<String> },
    Dodge { direction: Direction },
    Block,
    
    /// Item interaction
    Pickup { item: String },
    Use { item: String },
    
    /// Communication
    TalkTo { npc: String },
    
    /// UI
    OpenMenu,
    CloseMenu,
    
    /// Special
    Custom { name: String },
    Idle,
}

/// Direction of movement
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Direction {
    Forward,
    Backward,
    Left,
    Right,
}

/// Type of target for actions
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetType {
    Enemy,
    NPC,
    Environment,
}

/// Inferred intent behind an action
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InferredIntent {
    /// Primary goal ("Dodge incoming projectile")
    pub goal: String,
    
    /// Confidence 0.0-1.0
    pub confidence: f32,
    
    /// Alternative explanations with confidence
    pub alternatives: Vec<(String, f32)>,
    
    /// How many observations led to this inference
    pub sample_count: usize,
    
    /// Category of intent
    pub intent_category: IntentCategory,
}

/// Category of inferred intent
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum IntentCategory {
    Defensive,
    Offensive,
    ResourceGathering,
    Navigation,
    Social,
    Exploration,
    Unknown,
}

/// A decision rule the player tends to follow
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecisionRule {
    /// Condition that triggers this rule
    pub condition: GameCondition,
    
    /// Action the player tends to take
    pub recommended_action: PlayerAction,
    
    /// Success rate of this strategy
    pub success_rate: f32,
    
    /// Times observed
    pub observations: usize,
    
    /// Confidence in this rule
    pub confidence: f32,
}

/// Condition that may trigger a decision
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GameCondition {
    /// Health threshold
    pub health_below_pct: Option<u32>,
    
    /// Enemy count threshold
    pub enemies_nearby_count: Option<usize>,
    
    /// Location
    pub location_matches: Option<String>,
    
    /// Item nearby
    pub item_nearby: Option<String>,
    
    /// Custom condition string
    pub custom: Option<String>,
}

/// Player's playstyle
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PlayStyle {
    Aggressive,      // Face danger head-on
    Defensive,       // Avoid danger, prioritize safety
    Balanced,        // Middle ground
    Efficient,       // Optimize for speed/resource economy
    Exploratory,     // Investigate unknowns
    Social,          // Prioritize NPC interaction
}

impl PlayStyle {
    pub fn aggressiveness_score(&self) -> f32 {
        match self {
            PlayStyle::Aggressive => 1.0,
            PlayStyle::Balanced => 0.5,
            PlayStyle::Defensive => 0.0,
            PlayStyle::Efficient => 0.3,
            PlayStyle::Exploratory => 0.5,
            PlayStyle::Social => 0.2,
        }
    }
}

/// Humanization patterns (make agent play like a human)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HumanizationPatterns {
    /// Average reaction time in milliseconds
    pub reaction_delay_ms: u64,
    
    /// Variance in reaction time
    pub reaction_variance_ms: u64,
    
    /// How often player looks around when idle (0.0-1.0)
    pub idle_look_frequency: f32,
    
    /// Input smoothness (0.0 = perfect, 1.0 = very jittery)
    pub input_jitter: f32,
    
    /// How often player pauses to "think"
    pub pause_frequency: f32,
    
    /// Average pause duration
    pub pause_duration_ms: u64,
}

impl Default for HumanizationPatterns {
    fn default() -> Self {
        Self {
            reaction_delay_ms: 150,
            reaction_variance_ms: 50,
            idle_look_frequency: 0.3,
            input_jitter: 0.2,
            pause_frequency: 0.1,
            pause_duration_ms: 1500,
        }
    }
}

/// Episode of gameplay
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameEpisode {
    /// Episode identifier
    pub episode_id: String,
    
    /// When did this happen?
    pub timestamp: DateTime<Utc>,
    
    /// What was the objective?
    pub objective: String,
    
    /// How did it turn out?
    pub outcome: EpisodeOutcome,
    
    /// Duration of episode
    pub duration: Duration,
    
    /// Insights gained
    pub insights: Vec<Insight>,
    
    /// Observations made
    pub observation_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    Success,
    PartialSuccess { completion_pct: f32 },
    Failure { reason: String },
    Interrupted,
}

/// Knowledge gained from an episode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Insight {
    /// The claim ("Poison arrows are effective against dragons")
    pub claim: String,
    
    /// Confidence (0.0-1.0)
    pub confidence: f32,
    
    /// How was this learned?
    pub source: InsightSource,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum InsightSource {
    DirectObservation,
    DeductionFromFailure,
    ConversationWithNPC(String),
    ReadingInGameText,
    PatternMatching,
}

/// Overall player policy
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerPolicy {
    /// Decision rules for various situations
    pub decision_rules: Vec<DecisionRule>,
    
    /// Player's overall playstyle
    pub playstyle: PlayStyle,
    
    /// Resource priorities (what matters most)
    pub resource_priorities: HashMap<String, f32>,
    
    /// How to humanize outputs
    pub humanization: HumanizationPatterns,
    
    /// Confidence in this policy (0.0-1.0)
    pub overall_confidence: f32,
    
    /// How many observations were used to build this
    pub observations_total: usize,
}

impl PlayerPolicy {
    pub fn new() -> Self {
        Self {
            decision_rules: Vec::new(),
            playstyle: PlayStyle::Balanced,
            resource_priorities: HashMap::new(),
            humanization: HumanizationPatterns::default(),
            overall_confidence: 0.0,
            observations_total: 0,
        }
    }

    /// Find the best action for a given condition
    pub fn recommend_action(&self, condition: &GameCondition) -> Option<PlayerAction> {
        self.decision_rules
            .iter()
            .filter(|rule| rule.condition == *condition)
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .map(|rule| rule.recommended_action.clone())
    }
}

/// Statistics about an emulation session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmulationStats {
    /// Number of actions taken
    pub actions_taken: usize,
    
    /// Success rate (0.0-1.0)
    pub success_rate: f32,
    
    /// Objectives completed
    pub objectives_completed: usize,
    
    /// Times player paused/interrupted
    pub times_interrupted: usize,
    
    /// Deviations from learned policy
    pub policy_deviations: usize,
    
    /// Duration
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playstyle_aggressiveness() {
        assert_eq!(PlayStyle::Aggressive.aggressiveness_score(), 1.0);
        assert_eq!(PlayStyle::Defensive.aggressiveness_score(), 0.0);
        assert_eq!(PlayStyle::Balanced.aggressiveness_score(), 0.5);
    }

    #[test]
    fn test_player_action_equality() {
        let action1 = PlayerAction::Idle;
        let action2 = PlayerAction::Idle;
        assert_eq!(action1, action2);
    }

    #[test]
    fn test_player_policy_creation() {
        let policy = PlayerPolicy::new();
        assert_eq!(policy.decision_rules.len(), 0);
        assert_eq!(policy.playstyle, PlayStyle::Balanced);
        assert_eq!(policy.overall_confidence, 0.0);
    }

    #[test]
    fn test_humanization_defaults() {
        let h = HumanizationPatterns::default();
        assert!(h.reaction_delay_ms > 0);
        assert!(h.idle_look_frequency > 0.0);
    }

    #[test]
    fn test_game_condition_equality() {
        let cond1 = GameCondition {
            health_below_pct: Some(50),
            enemies_nearby_count: None,
            location_matches: None,
            item_nearby: None,
            custom: None,
        };
        let cond2 = GameCondition {
            health_below_pct: Some(50),
            enemies_nearby_count: None,
            location_matches: None,
            item_nearby: None,
            custom: None,
        };
        assert_eq!(cond1, cond2);
    }
}
