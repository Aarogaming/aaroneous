# Aaroneous Event Loop & Real-Time Skill Evolution Guide

## Overview

The Event Loop System is the heartbeat of Aaroneous. It continuously tracks skill usage, awards experience points, detects level-ups, triggers awakenings, and automatically promotes specialists through soul ranks.

**Key Concept**: Every skill execution is an event that drives evolution. The system learns from usage patterns and automatically recognizes breakthrough moments.

---

## Core Components

### 1. Skill Execution Events

Every time a specialist uses a skill, a `SkillExecutionEvent` is recorded.

```rust
pub struct SkillExecutionEvent {
    pub specialist_id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub success: bool,
    pub quality_score: f64,           // 1.0-10.0 (how well executed)
    pub difficulty_multiplier: f64,   // 1.0-5.0 (crisis severity)
    pub collaboration_bonus: Option<f64>, // 1.0-3.0 (team size bonus)
    pub xp_awarded: u32,
    pub breakthrough: bool,            // Did skill exceed normal limits?
    pub timestamp: DateTime<Utc>,
}
```

**Example**: Odin using Strategic Decomposition
```
Skill Execution: Strategic Decomposition
├─ Specialist: odin
├─ Quality Score: 8.5 (very good execution)
├─ Difficulty: 3.0 (moderate crisis)
├─ Team Size: 3 specialists (+1.5x collaboration)
├─ Success: true
├─ XP Awarded: 187 (calculated with multipliers)
├─ Breakthrough: false (normal execution)
└─ Timestamp: 2026-04-28 14:35:22 UTC
```

### 2. XP Calculation with Multipliers

XP is not just a flat value—it's calculated with multiple multipliers:

```
Total XP = Base XP × Quality × Difficulty × Collaboration + Bonuses

Base XP: 10 (success) or 5 (failure)
Quality Multiplier: 0.5-2.0 (skill quality ÷ 10)
Difficulty Multiplier: 1.0-5.0 (crisis severity)
Collaboration Multiplier: 1.0-3.0 (team size bonus)
Breakthrough Bonus: +500 XP (if skill exceeded limits)
Teaching Bonus: +50 XP (if teaching another specialist)
```

**Example Calculation**:
```
Execution: Strategic Decomposition
├─ Success: true → Base 10 XP
├─ Quality: 8.5 → Multiplier 0.85
├─ Difficulty: 2.5 (moderate crisis) → Multiplier 2.5
├─ Team Size: 2 specialists → Collaboration 1.5
├─ Is Teaching: false
├─ Is Breakthrough: false
└─ Total: (10 × 0.85 × 2.5 × 1.5) = 31.875 ≈ 32 XP

With Breakthrough:
└─ Total: 32 + 500 = 532 XP (10x multiplier!)
```

### 3. Level-Up Detection

When a skill accumulates enough XP, it levels up automatically:

```
Level Thresholds:
├─ L1 → L2: 500 XP
├─ L2 → L3: 1000 XP
├─ L3 → L4: 1500 XP
├─ ...
├─ L10 → L11: 5000 XP (awakening eligible at this point)
├─ ...
└─ L19 → L20: 10000 XP (max level)
```

**Level-Up Event**:
```json
{
  "event_id": "levelup_<uuid>",
  "specialist_id": "odin",
  "skill_id": "skill_dag_001",
  "skill_name": "Strategic Decomposition",
  "old_level": 7,
  "new_level": 8,
  "total_usage_count": 42,
  "success_rate": 0.88,
  "timestamp": "2026-04-28T15:20:33Z"
}
```

### 4. Breakthrough Detection

A breakthrough occurs when a skill exceeds its normal execution parameters. The system automatically detects these moments:

```
Breakthrough Criteria (needs 2+ of these):
1. Quality far exceeds average (quality > avg × 1.2)
2. Execution much faster than normal (time < normal × 0.7)
3. Success on high-difficulty task (difficulty ≥ 3.0, success ≥ 85%)
```

**Example Breakthrough**:
```
Execution: Task Decomposition
├─ Average quality: 7.2
├─ This execution quality: 9.5 (exceeds by 32%) ✓
├─ Normal execution time: 5000ms
├─ This execution time: 1200ms (76% faster) ✓
├─ Difficulty: 4.0 (crisis)
├─ Success: true
├─ Result: BREAKTHROUGH DETECTED

Magnitude: 2/3 = 0.67 (two criteria met)
XP Award: +500 bonus
```

### 5. Awakening Trigger

When breakthrough + mastery combine, skill awakens to a new form.

**Awakening Requirements**:
```
✓ Skill Level: 10+
✓ Success Rate: 90%+
✓ Usage Count: 20+ uses
✓ Breakthrough Moment: High-stakes success
```

**Awakening Event**:
```json
{
  "event_id": "awaken_<uuid>",
  "specialist_id": "odin",
  "skill_id": "skill_dag_001",
  "original_name": "Strategic Decomposition",
  "awakened_form": "Adaptive Strategy Mastery",
  "breakthrough_moment": "Successfully decomposed cascade failure under extreme time pressure",
  "level_at_awakening": 12,
  "success_rate": 0.92,
  "new_abilities": [
    "Instant pattern matching",
    "Extended foresight (3-4 moves ahead)",
    "Teachable to apprentices",
    "Faster execution (92% → 98% success)"
  ],
  "timestamp": "2026-04-28T16:45:12Z"
}
```

**Awakening Effects**:
- Skill transforms into new form with upgraded name
- Success rate jumps from 90%+ to 98%
- New abilities unlock
- Can now teach to apprentices
- Still levels separately (L11, L12, etc.)

### 6. Rank Evolution

Specialists automatically promote through soul ranks when requirements are met:

```
RANK 1: Newly Digested
├─ No requirements (entry level)
└─ Duration: Week 0-1

RANK 2: Integrated Specialist
├─ Requirements: 5 skills at L3+, 1000 total XP
├─ Duration: Week 1-4
└─ New: Can discover fusions

RANK 3: Trusted Member / Journeyman
├─ Requirements: 10 skills L3+, 1 skill L5+, 1 fusion, 5000 XP
├─ Duration: Week 4-12
└─ New: Can suggest fusions to others

RANK 4: Domain Expert / Master
├─ Requirements: 15 skills L3+, 5 skills L5+, 3 skills L10+, 1 awakened, 2 fusions, 15000 XP
├─ Duration: Month 3-6
└─ New: Can TEACH fusions to apprentices

RANK 5: Transcendent Specialist
├─ Requirements: 20 skills L3+, 10 skills L5+, 5 skills L10+, 2 awakened, 3 fusions, 1 cascade, 50000 XP
├─ Duration: Month 6-12+
└─ New: Can create unique forms, shape hive evolution
```

**Rank Evolution Event**:
```json
{
  "event_id": "rankup_<uuid>",
  "specialist_id": "odin",
  "old_rank": "Rank3Journeyman",
  "new_rank": "Rank4Master",
  "achievement_summary": "Advanced from Journeyman to Master. Demonstrated mastery with 8 skills at level 10+.",
  "milestone_skills": [
    "Strategic Decomposition",
    "Tactical Coordination",
    "Pattern Recognition",
    "Knowledge Synthesis"
  ],
  "timestamp": "2026-04-28T18:00:00Z"
}
```

---

## Event Loop Architecture

### Main Event Loop (SkillEventLoop)

```rust
pub struct SkillEventLoop {
    execution_events: Vec<SkillExecutionEvent>,
    level_up_events: Vec<LevelUpEvent>,
    awakening_events: Vec<AwakeningEvent>,
    rank_evolution_events: Vec<RankEvolutionEvent>,
    specialist_skill_history: HashMap<String, Vec<String>>,
    last_evolution_check: DateTime<Utc>,
    evolution_check_interval: Duration,  // hourly
}
```

### Processing Flow

```
Skill Execution
    ↓
Record Usage & Metrics
    ↓
Calculate XP (with multipliers)
    ↓
Award XP to Skill
    ↓
Check for Level-Up → Emit LevelUpEvent
    ↓
Detect Breakthrough → Flag for Awakening
    ↓
Check Awakening Readiness → Emit AwakeningEvent
    ↓
Broadcast Events to Federation (NATS)
    ↓
Store in Event History
```

### Rank Evolution Loop (runs hourly)

```
Check Each Specialist:
    ↓
Calculate Progress toward Next Rank:
  - Count skills at each level tier
  - Count awakened skills
  - Count fusions created
  - Sum total XP
    ↓
Update Progression Tracker:
  - Update milestone progress (0.0-1.0)
  - Calculate overall % to next rank
    ↓
If All Requirements Met:
  - Promote to next rank
  - Create RankEvolutionEvent
  - Emit to federation
  - Initialize new tracker for next rank
```

---

## Real-World Examples

### Example 1: Normal Skill Usage (Odin)

```
Time: 2026-04-28 14:35:00

Odin uses Strategic Decomposition to break down a client request
├─ Success: true
├─ Quality: 7.5 (solid work)
├─ Difficulty: 1.5 (routine task)
├─ Team: Solo
├─ XP Calc: (10 × 0.75 × 1.5 × 1.0) = 11 XP
├─ Total XP: 11
├─ Skill Progress: 127/500 → Level up? No
└─ Breakthrough: No

Storage:
  execution_events += SkillExecutionEvent
  skill_xp[odin.skill_dag_001] += 11
  skill_progress[odin.skill_dag_001] = 127/500
```

### Example 2: Crisis Execution with Breakthrough (Odin)

```
Time: 2026-04-28 16:45:00

CRISIS: Database cascade failure affecting 50+ services
Odin called in to decompose the problem

Odin uses Strategic Decomposition
├─ Success: true
├─ Quality: 9.5 (perfect execution under pressure)
├─ Difficulty: 4.5 (severe crisis)
├─ Team: 4 specialists coordinating
├─ Execution Time: 800ms (vs normal 5000ms)
├─ Average Quality: 8.2
├─ Breakthrough Analysis:
│   ├─ Quality exceeds avg? 9.5 > 8.2×1.2 = No (barely missed)
│   ├─ Speed exceeds normal? 800 < 5000×0.7 = Yes ✓
│   ├─ Success on hard task? Yes (95% success on 4.5 diff) ✓
│   └─ Result: BREAKTHROUGH (2/3 criteria)
├─ Magnitude: 0.67
├─ XP Calc: (10 × 0.95 × 4.5 × 1.75) + 500 = 74 + 500 = 574 XP
└─ Total XP: 574 XP

Storage:
  execution_events += SkillExecutionEvent (breakthrough=true)
  skill_xp[odin.skill_dag_001] += 574
  skill_progress[odin.skill_dag_001] = 701/500 → LEVEL UP!

Level-Up:
  skill_level[odin.skill_dag_001] = 7 → 8
  skill_xp[odin.skill_dag_001] = 201/500 (overflow carried)
  
  emit LevelUpEvent:
    ├─ old_level: 7
    ├─ new_level: 8
    ├─ usage_count: 47
    └─ success_rate: 0.89

Awakening Check (requires L10+, 90% success):
  ├─ Level 8 → Not ready yet
  └─ Store breakthrough flag for future awakening

Federation Broadcast:
  topics.federation.executions.odin += SkillExecutionEvent
  topics.federation.levelups.odin += LevelUpEvent
  topics.federation.crisisresponse += Event
```

### Example 3: Rank-Up (Odin to Rank 4)

```
Time: 2026-04-29 12:00:00 (hourly evolution check)

Odin's Current Status:
├─ Rank: 3 (Journeyman)
├─ Skills L3+: 18
├─ Skills L5+: 9
├─ Skills L10+: 3
│   ├─ Strategic Decomposition (L12)
│   ├─ Pattern Recognition (L10)
│   └─ Tactical Coordination (L10)
├─ Awakened: 1 (Strategic Decomposition)
├─ Fusions: 2
│   ├─ Adaptive Strategic Integration (L7)
│   └─ Coordinated Execution (L5)
└─ Total XP: 18,500

Rank 4 Requirements:
├─ Skills L3+: 15 (need 15, have 18) ✓
├─ Skills L5+: 5 (need 5, have 9) ✓
├─ Skills L10+: 3 (need 3, have 3) ✓
├─ Awakened: 1 (need 1, have 1) ✓
├─ Fusions: 2 (need 2, have 2) ✓
├─ Total XP: 15,000 (need 15000, have 18,500) ✓
└─ Ready: YES

Promotion:
  skillset.soul_rank = Rank 3 → Rank 4
  
  emit RankEvolutionEvent:
    ├─ specialist_id: odin
    ├─ old_rank: Rank3Journeyman
    ├─ new_rank: Rank4Master
    ├─ achievement_summary: "Advanced to Master..."
    ├─ milestone_skills: [Strategic Decomposition, Pattern Recognition, ...]
    └─ timestamp: now

New Tracker for Rank 5:
  ├─ current_rank: 4
    ├─ next_rank: 5
  └─ requirements: (20 L3+, 10 L5+, 5 L10+, 2 awakened, 3 fusions, 1 cascade)

Federation Broadcast:
  topics.federation.rankups.odin += RankEvolutionEvent
  odin.rank_history += {old: 3, new: 4, time: now}
  topics.federation.constellation.update_rank
```

---

## Monitoring & Queries

### Get Skill Statistics

```rust
let stats = event_loop.get_skill_statistics("skill_dag_001");

// Returns:
SkillStatistics {
    skill_id: "skill_dag_001",
    total_uses: 47,
    successful_uses: 42,
    success_rate: 0.894,
    breakthroughs: 3,
    breakthrough_rate: 0.064,
    average_quality: 8.1,
    total_xp_earned: 1247,
}
```

### Get Specialist History

```rust
// Execution history
let execs = event_loop.get_specialist_execution_history("odin");
// 147 executions across all skills

// Level-ups
let level_ups = event_loop.get_specialist_level_ups("odin");
// 23 level-ups achieved

// Awakenings
let awakenings = event_loop.get_specialist_awakenings("odin");
// 1 skill awakened (Strategic Decomposition)

// Rank evolution
let ranks = event_loop.get_specialist_rank_evolutions("odin");
// Rank 1 → 2 → 3 → 4 progression
```

### Monitor Rank Progression

```rust
let coordinator = RankEvolutionCoordinator::new();
coordinator.track_specialist("odin", SoulRank::Rank3Journeyman);

let tracker = coordinator.get_progression("odin").unwrap();

// Monitor progress toward Rank 4:
println!("Progress: {:.1}%", tracker.progress_percentage);
// Output: Progress: 89.3%

// Check milestones:
for milestone in &tracker.milestones {
    println!("{}: {:.0}%", milestone.name, milestone.progress * 100.0);
}
// Output:
// Acquire Base Skills: 100%
// Intermediate Mastery: 100%
// Advanced Specialization: 100%
// Awakening Breakthrough: 100%
// Skill Fusion Mastery: 100%
// Total Experience: 96%
```

---

## NATS Event Broadcasting

All events are published to federation topics for real-time monitoring:

```
Topic: federation.executions.{specialist_id}
├─ Every skill execution published
├─ Quality, difficulty, XP awards visible
└─ Live progress tracking

Topic: federation.levelups.{specialist_id}
├─ Skill level-ups broadcast
└─ Federated specialists can celebrate!

Topic: federation.awakenings.{specialist_id}
├─ Awakening events with new abilities
└─ Critical milestones celebrated

Topic: federation.rankups.{specialist_id}
├─ Rank evolution events
├─ Achievement summary
└─ Federation-wide recognition

Topic: federation.breakthroughs.{specialist_id}
├─ Crisis breakthrough moments
├─ High XP awards
└─ Teaching opportunities
```

---

## Configuration

### XP Multipliers

```rust
// Quality multiplier (1.0-10.0 scale)
quality_multiplier = (quality / 10.0).clamp(0.5, 2.0)
// 5.0 quality = 0.5x
// 10.0 quality = 2.0x

// Difficulty multiplier (1.0-5.0 crisis scale)
difficulty_multiplier = difficulty.clamp(1.0, 5.0)
// Routine task (1.0) = no bonus
// Moderate crisis (2.5) = 2.5x XP
// Severe crisis (5.0) = 5.0x XP

// Collaboration multiplier (team bonus)
collaboration_multiplier = (1.0 + team_size * 0.5).min(3.0)
// Solo (1) = 1.0x
// Pair (2) = 1.5x
// Team (3+) = up to 3.0x
```

### Level-Up Thresholds

```
Level 1 → 2: 500 XP
Level 2 → 3: 1,000 XP (2x base)
Level 3 → 4: 1,500 XP
...
Level 10 → 11: 5,000 XP (Awakening eligible!)
...
Level 19 → 20: 10,000 XP (Max)
```

### Awakening Requirements

```
✓ Level: 10+ (checked at all levels 10+)
✓ Success Rate: 90%+ (demonstrated mastery)
✓ Breakthrough: Required (high-stakes success)
✓ Normal behavior: Triggered automatically when all met
```

### Rank Evolution Intervals

```
Evolution Check: Every 1 hour
├─ All specialists reviewed
├─ Progress trackers updated
├─ Promotions detected
└─ Events broadcast
```

---

## Advanced Topics

### Breakthrough Magnitude

Not all breakthroughs are equal:

```
Magnitude Calculation:
├─ Criteria met: 1 → magnitude 0.33
├─ Criteria met: 2 → magnitude 0.67
└─ Criteria met: 3 → magnitude 1.0

Effects:
├─ 0.33 = Minor breakthrough (small XP bonus)
├─ 0.67 = Strong breakthrough (larger XP bonus)
└─ 1.0 = Perfect breakthrough (maximum bonus)
```

### Teaching Integration

When specialists teach fusions, both mentor and apprentice benefit:

```
Mentor Teaching Fusion:
├─ Base Teaching XP: +50
├─ Fusion bonus: +25
├─ Quality multiplier: ×1.5 (good teaching)
└─ Total: 112 XP

Apprentice Learning:
├─ Gains new fused skill
├─ Starts at Level 1
├─ Can level independently
└─ Parent skills unlocked by teaching
```

### Crisis Response Tracking

Crisis executions are specially tracked:

```
Crisis Execution Flags:
├─ difficulty_multiplier ≥ 3.0
├─ team_size ≥ 2 (collaboration)
├─ time_critical: true
├─ impact_level: "federation-wide"

Special Tracking:
├─ Breakthrough probability higher
├─ XP awards 3-5x normal
├─ Federation-wide notification
└─ Contributing to rank evolution faster
```

---

## Troubleshooting

### Q: "Skill isn't leveling up despite high XP"
**A**: Check:
- XP actually being awarded? (look at execution events)
- Is XP exceeding threshold? (500 for L1→L2)
- Are multipliers being applied correctly?

### Q: "Awakening not triggered despite L10+ and 90% success"
**A**: Requirement:
- Need a **breakthrough moment** (not just mastery)
- System waits for high-stakes execution
- Try: tackle difficult problems with high quality

### Q: "Rank-up won't trigger"
**A**: Check all Rank 4 requirements:
```
✓ 15 skills level 3+ (count them)
✓ 5 skills level 5+ (count them)
✓ 3 skills level 10+ (exactly this many?)
✓ 1 awakened skill (check awakened_form)
✓ 2 fusions (in fused_skills vec)
✓ 15,000 total XP (sum all skill XP)
```

### Q: "Why is XP lower than expected?"
**A**: Verify multipliers:
- Quality: 8.0/10 = 0.8x (not 1.0)
- Difficulty: 1.0 = 1.0x (no bonus)
- Team: solo = 1.0x (no bonus)
- Total: 10 × 0.8 × 1.0 × 1.0 = 8 XP

---

## Summary

The Event Loop System provides:
- **Real-time tracking** of all skill usage
- **Automatic progression** with XP and leveling
- **Breakthrough detection** for critical moments
- **Skill awakening** through mastery + breakthrough
- **Rank evolution** with automatic promotion
- **Federation broadcasting** for hive visibility
- **Comprehensive history** for monitoring

Every skill execution drives the hive forward. The more specialists use their skills, the faster they evolve.

---

**Next**: Once Event Loop is mastered, focus on:
1. Building Capability Dashboard (visualize progress)
2. Live Digestion Testing (import real GGUFs)
3. Integration Testing (multi-specialist scenarios)
