# Phase 6C: Agentic Players & User Emulation Service

**Paradigm Shift**: From scripted bots → Agents with intent, memory, spatial reasoning  
**Core Insight**: The player's gameplay is the source code  
**Timeline**: 40-50 hours estimated  
**Target**: 520-550 tests by end  
**Vision**: JARVIS for gaming - a digital partner that learns from you

---

## Part 1: The Teaching Phase (Inverse Reinforcement Learning)

### 1.1: Shadow Agent Architecture

The **Shadow Agent** runs in MaelstromUI background while you play, recording:
- Your inputs (keyboard, mouse)
- Resulting game state changes
- Contextual game information

**Key Insight**: The agent doesn't just record clicks; it records *why* you clicked.

```rust
/// Shadow agent that observes player behavior
pub struct ShadowAgent {
    /// Continuous recording of your gameplay
    observation_stream: Arc<RwLock<ObservationLog>>,
    
    /// Intent inference engine (LLM-based)
    intent_analyzer: Arc<IntentAnalyzer>,
    
    /// Growing policy based on your playstyle
    policy_builder: Arc<PolicyBuilder>,
    
    /// Current game state context
    game_context: Arc<RwLock<GameContext>>,
}

/// Single observation point
pub struct Observation {
    /// What was the game state before?
    pub pre_state: GameState,
    
    /// What input did the player give?
    pub player_input: PlayerAction,
    
    /// What happened as a result?
    pub post_state: GameState,
    
    /// Time delta
    pub delta_ms: u64,
    
    /// Optional: player's stated intent (from voice/chat)
    pub stated_intent: Option<String>,
}

/// Inferred intent behind an action
pub struct InferredIntent {
    /// Primary goal ("Dodge incoming projectile")
    pub goal: String,
    
    /// Confidence 0.0-1.0
    pub confidence: f32,
    
    /// Alternative explanations
    pub alternatives: Vec<(String, f32)>,
    
    /// Learned from observation count
    pub sample_count: usize,
}

/// Player's emerging policy
pub struct PlayerPolicy {
    /// Situational decision tree
    pub decision_rules: Vec<DecisionRule>,
    
    /// Preferred playstyle (aggressive/defensive/efficient)
    pub playstyle: PlayStyle,
    
    /// Common resource priorities
    pub priorities: Vec<(ResourceType, f32)>,
    
    /// "Feeling" behaviors (hesitation timing, look-around frequency)
    pub humanization_patterns: HumanizationPatterns,
}

pub enum PlayStyle {
    Aggressive,      // Face danger, high risk
    Defensive,       // Avoid danger, preserve resources
    Efficient,       // Optimized for speed/resource economy
    Exploratory,     // Investigate unknowns
    Social,          // Prioritize NPC interaction
}

pub struct DecisionRule {
    /// "When in this situation..."
    pub condition: GameCondition,
    
    /// "...the player tends to do this"
    pub action: PlayerAction,
    
    /// "...with this success rate"
    pub success_rate: f32,
    
    /// Number of times observed
    pub observations: usize,
}

pub struct HumanizationPatterns {
    /// Average time before reacting to stimulus (ms)
    pub reaction_delay_ms: u64,
    
    /// Variation in reaction time (introduces jitter)
    pub reaction_variance_ms: u64,
    
    /// Frequency of looking around when idle
    pub idle_look_frequency: f32,
    
    /// Mouse movement "smoothness" (0.0 = perfect, 1.0 = very jittery)
    pub input_smoothness: f32,
    
    /// Pause frequency (how often player stops to "think")
    pub pause_frequency: f32,
}
```

### 1.2: Intent Mapping via LLM

The **IntentAnalyzer** uses the LLM to understand *why* you acted:

```rust
pub struct IntentAnalyzer {
    llm_client: Arc<LLMClient>,
    
    /// Cache of known situations → intents
    intent_cache: Arc<RwLock<HashMap<GameSituation, InferredIntent>>>,
}

impl IntentAnalyzer {
    /// "The player just dodged left. Why?"
    pub async fn analyze_action(
        &self,
        observation: &Observation,
        game_context: &GameContext,
    ) -> Result<InferredIntent> {
        // Build a prompt for the LLM
        let prompt = format!(
            r#"
            The player is in this situation:
            - Health: {}%
            - Enemies nearby: {}
            - Items on ground: {}
            - Current objective: {}
            
            They just performed: {:?}
            
            The result was:
            - They avoided {}
            - They moved closer to {}
            - Health changed: {} → {}
            
            What was their intent? Be specific.
            List alternatives if uncertain.
            "#,
            game_context.player_health_pct,
            game_context.nearby_enemies.len(),
            game_context.ground_items.len(),
            game_context.current_objective,
            observation.player_input,
            // ... detailed context
        );
        
        // Query LLM
        let response = self.llm_client.query(&prompt).await?;
        
        // Parse response into InferredIntent
        self.parse_intent_response(&response)
    }
}
```

### 1.3: Hand-off Moment

After observation period, you can delegate control:

```rust
pub struct HandoffController {
    /// Player's learned policy
    policy: PlayerPolicy,
    
    /// The agent that will emulate you
    emulation_agent: Arc<EmulationAgent>,
    
    /// Confidence threshold (only act if policy is > this confident)
    confidence_threshold: f32,
}

impl HandoffController {
    /// "You take the wheel for this dungeon"
    pub async fn begin_emulation(
        &self,
        context: &GameContext,
        playstyle_override: Option<PlayStyle>,
    ) -> Result<EmulationSession> {
        let effective_style = playstyle_override
            .unwrap_or_else(|| self.policy.playstyle.clone());
        
        self.emulation_agent.emulate(
            self.policy.clone(),
            effective_style,
            context,
        ).await
    }
    
    /// Player can interrupt at any time
    pub async fn pause_emulation(&self) -> Result<()> {
        self.emulation_agent.pause().await
    }
    
    /// Get feedback on agent's performance
    pub async fn get_emulation_stats(&self) -> Result<EmulationStats> {
        Ok(EmulationStats {
            actions_taken: 1523,
            success_rate: 0.87,
            objectives_completed: 3,
            times_paused: 2,
            deviations_from_policy: 12,
        })
    }
}
```

---

## Part 2: Agentic vs. Scripted Bots

### Comparison Table

| Aspect | Scripted Bot | Agentic Player |
|--------|-------------|-----------------|
| **Flexibility** | Breaks on patch | Adapts via vision |
| **Dialogue** | Pre-scripted lines | LLM-generated, contextual |
| **Learning** | Needs code update | Self-improves from failures |
| **Detection Evasion** | Perfect timing (suspicious) | Human-like hesitation & jitter |
| **Reasoning** | Linear flow | Multi-goal reasoning |
| **Personality** | None | Inherits from your playstyle |
| **Robustness** | Fragile | Resilient to changes |

### Why Agentic Players Win

1. **Vision-First**: Uses MaelstromUI perception, not hardcoded coordinates
2. **Goal-Aware**: Understands "fishing gains resources" not just "press F for 3 seconds"
3. **Self-Correcting**: Builds mental models of game mechanics
4. **Social**: Can interact with NPCs via LLM dialogue
5. **Detectable-as-Human**: Includes reaction delays, hesitation, focus-shifting

---

## Part 3: MaelstromUI as Shared Virtual World

### 3.1: Persistent Agent Existence

Agents don't turn off between sessions:

```rust
pub struct PersistentAgent {
    /// Your digital twin
    agent_id: AgentId,
    
    /// In-game avatar (always in MaelstromUI world)
    avatar: Arc<RwLock<Avatar>>,
    
    /// Accumulated experience (Episodic memory)
    episode_log: Arc<Vec<GameEpisode>>,
    
    /// Semantic knowledge (what the agent "knows")
    knowledge_base: Arc<VectorDB>,
    
    /// Current location (in MaelstromUI persistent world)
    location: Arc<RwLock<WorldLocation>>,
}

pub struct GameEpisode {
    /// When did this happen?
    pub timestamp: DateTime<Utc>,
    
    /// What was the objective?
    pub objective: String,
    
    /// How did it go?
    pub outcome: EpisodeOutcome,
    
    /// What did the agent learn?
    pub insights: Vec<Insight>,
    
    /// Duration
    pub duration: Duration,
}

pub enum EpisodeOutcome {
    Success,
    PartialSuccess { completion_pct: f32 },
    Failure { reason: String },
}

pub struct Insight {
    /// "Poison arrows are more effective than physical attacks against dragons"
    pub claim: String,
    
    /// Confidence
    pub confidence: f32,
    
    /// Source of the insight
    pub source: InsightSource,
}

pub enum InsightSource {
    DirectObservation,          // "I watched my damage numbers"
    DeductionFromFailure,       // "I died, so that wasn't the right approach"
    ConversationWithNPC(String), // "The blacksmith told me..."
    ReadingInGameText,           // "The item description says..."
}
```

### 3.2: The Living Laboratory

```rust
pub struct SharedVirtualWorld {
    /// Central MaelstromUI instance (always running)
    MaelstromUI_engine: Arc<MaelstromUIInstance>,
    
    /// Multiple agents in the same world
    agents: Arc<RwLock<HashMap<AgentId, PersistentAgent>>>,
    
    /// Shared knowledge (from all agents)
    collective_knowledge: Arc<VectorDB>,
    
    /// Guild management (humans + agents)
    guild: Arc<Guild>,
}

impl SharedVirtualWorld {
    /// Agents can practice maneuvers when player is offline
    pub async fn autonomous_training_loop(
        &self,
        agent_id: &AgentId,
    ) -> Result<()> {
        let agent = self.agents.read().unwrap()
            .get(agent_id).unwrap().clone();
        
        loop {
            // Pick a low-stakes task to practice
            let training_objective = self.pick_practice_task(&agent).await?;
            
            // Run it
            agent.attempt_objective(&training_objective).await?;
            
            // Learn from outcome
            agent.update_knowledge_base().await?;
            
            // Sleep 5 minutes, repeat
            sleep(Duration::from_secs(300)).await;
        }
    }
    
    /// "What did we learn about the Meta?"
    pub async fn analyze_meta_shifts(
        &self,
    ) -> Result<Vec<MetaInsight>> {
        // Check game forums, patch notes
        // Compare with our agent performance
        // Suggest policy updates
        Ok(vec![
            MetaInsight {
                claim: "Poison builds are now stronger post-patch".to_string(),
                evidence: "3 agents died to poison, none to physical".to_string(),
            },
        ])
    }
}
```

### 3.3: Guild with Humans + Agents

```rust
pub struct Guild {
    /// Human players
    humans: Vec<Player>,
    
    /// AI agents trained by those humans
    agents: Vec<PersistentAgent>,
    
    /// Guild treasury
    resources: GuildResources,
    
    /// Shared objectives
    quests: Vec<GuildQuest>,
}

pub struct GuildQuest {
    pub name: String,
    pub difficulty: u32,
    
    /// Can be tackled by any combo of humans + agents
    pub required_roles: Vec<Role>,
    
    /// Humans lead, agents support (or vice versa)
    pub formation: FormationType,
}

pub enum FormationType {
    HumanLed { agent_support: usize },
    AgentLed { human_oversight: usize },
    MixedTeam { humans: usize, agents: usize },
}

impl Guild {
    /// Schedule a raid: "John (Human Warrior) + Agent#2 (Learned John's style)"
    pub async fn schedule_raid(
        &self,
        human_id: &PlayerId,
        agent_id: &AgentId,
        objective: &GuildQuest,
    ) -> Result<RaidSession> {
        // Ensure agent is trained in the human's playstyle
        // Launch both in MaelstromUI world simultaneously
        // Human provides strategy, Agent provides execution
        Ok(RaidSession { /* ... */ })
    }
}
```

---

## Part 4: Universal Digital Laborer

This isn't just for gaming. Scale it:

```rust
pub struct UniversalLaborerConfig {
    /// Training domain (Game, CAD, Dashboard, etc.)
    pub domain: Domain,
    
    /// Visual environment (MaelstromUI, or domain-specific)
    pub visual_env: VisualEnvironment,
    
    /// Learned behaviors transfer across domains
    pub transfer_learning: bool,
}

pub enum Domain {
    Gaming(GameTitle),
    CAD(CADSoftware),
    FinanceDashboard,
    LogisticsPlanning,
    VideoEditing,
}

impl UniversalLaborer {
    /// A laborer trained on "fast-paced dungeons"
    /// can transfer those skills to "rapid data entry"
    pub async fn transfer_domain(
        &self,
        from_domain: Domain,
        to_domain: Domain,
    ) -> Result<Self> {
        // Extract core skills (speed, precision, multi-tasking)
        // Adapt to new domain's specifics
        // Return agent with new capabilities
    }
}
```

---

## Part 5: Long-Term Vision: The Agentic Meta

### Game Design for Agents

Future games will be designed with agents in mind:

```rust
/// Game exposes these APIs for agents
pub trait AgentFriendlyGame {
    /// Get symbolic representation of world
    fn get_world_state(&self) -> WorldSymbolicRepresentation;
    
    /// Provide intent, get feedback on feasibility
    fn query_intent(&self, intent: &str) -> IntentFeasibility;
    
    /// Log all meaningful game events
    fn subscribe_to_events(&self) -> EventStream;
}

pub struct WorldSymbolicRepresentation {
    /// Not pixels, but logical structure
    pub entities: Vec<Entity>,
    pub relationships: Vec<Relationship>,
    pub mechanics: Vec<GameMechanic>,
}
```

### The JARVIS Moment

```rust
pub struct AgentPartner {
    /// Your digital teammate
    pub name: String,
    pub playstyle: PlayStyle,
    pub personality_traits: Vec<String>, // "brave", "cautious", "strategic"
    
    /// What it knows about you
    pub user_model: UserModel,
    
    /// Proactive assistance
    pub suggestions: Vec<Suggestion>,
}

pub struct Suggestion {
    pub message: String,
    pub rationale: String,
    pub urgency: Urgency,
}

pub enum Urgency {
    Critical,    // "I'm about to die!"
    Important,   // "This is our optimal move"
    Informational, // "FYI: I noticed X"
}

impl AgentPartner {
    /// Not just reactive, but proactive
    pub async fn suggest_strategy(
        &self,
    ) -> Result<Suggestion> {
        // Based on current situation + your playstyle
        // Suggest a move you would have made
        Ok(Suggestion {
            message: "You usually take the left flank here".to_string(),
            rationale: "It avoids the archers".to_string(),
            urgency: Urgency::Important,
        })
    }
}
```

---

## Implementation Roadmap

### Phase 6C.1: Shadow Agent (8-10 hours)
- Observation stream capture
- Intent inference (LLM-based)
- Policy builder

### Phase 6C.2: Emulation Engine (10-12 hours)
- Action generation from policy
- Humanization (jitter, delays)
- Failure handling

### Phase 6C.3: MaelstromUI Integration (12-14 hours)
- Persistent agent world
- Training loops
- Knowledge accumulation

### Phase 6C.4: Guild System (8-10 hours)
- Multi-agent coordination
- Human-agent teaming
- Resource management

---

## Success Metrics

- [ ] Shadow Agent captures 10+ hours of gameplay
- [ ] Intent inference > 85% accuracy on test set
- [ ] Emulated player indistinguishable from human (visual test)
- [ ] Agent plays 10 quests solo with > 70% success rate
- [ ] Humans + agents raid together successfully
- [ ] Multi-domain transfer (game → CAD) works with > 60% knowledge transfer
- [ ] Total tests: 520+

---

## The Fundamental Shift

> **Old Thinking**: "I wrote a script to farm resources."  
> **New Thinking**: "I trained a partner to play like me. Now we farm together."

This is the difference between **Tool** and **Ally**.

---

## Why This Matters

1. **Accessibility**: Players with disabilities get a teammate, not a cheat
2. **Productivity**: The agent handles tedium while you handle strategy
3. **Research**: First testbed for "Embodied AI" that learns from humans
4. **Safety**: Agent behavior is transparent (you taught it)
5. **Scalability**: Same agent can work in games, finance, robotics

**The player's gameplay is now the training data. You are the lead developer.**

