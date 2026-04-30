# The Omni Constellation: A Living Knowledge Map

## Overview

The **Omni Constellation** is more than a documentation system—it's a living, breathing map of Aaroneous's knowledge, organized in 3D semantic space. Maintained by **Omni** (the RelicAgent at the University), the constellation is a source-of-truth document that tracks:

- **Features**: What we're building (Why, when, priority)
- **Bugs**: What's broken (Severity, blockers, discovery date)
- **Roadmap**: Where we're going (Target phases, dependencies)
- **Decisions**: What we've decided (Architecture, design choices, rationale)
- **Lore**: The story we're living (Agent discoveries, narratives, Easter eggs)
- **Architecture**: How it all fits (System layers, component relationships)
- **Incidents**: What went wrong (Post-mortems, lessons learned)
- **References**: Where to find answers (Documentation links, guides)
- **Resources**: What we have available (External dependencies, tools)
- **Test Cases**: How we verify (Tests organized by topic)

## The Spatial Metaphor

Nodes in the constellation are positioned in **3D semantic space** where proximity means relatedness:

```
          FUTURE VISION
         /       |       \
    Planned    Current    Completed
    Roadmap    Work       History
        \       |       /
        PAST ← CENTER → FUTURE
       
    Also varies by:
    - Domain (Theory ↔ Execution)
    - Priority (Hidden ↔ Critical)
```

### The Three Axes

**X-Axis: Domain Spectrum**
- Left (-1000): **Theoretical Knowledge** (Merlin's Library)
  - Decisions, references, architecture theory
  - "Why are we doing this?"
- Right (+1000): **Practical Execution** (Hephaestus's Blacksmith)
  - Features, bugs, implementation details
  - "How do we build this?"

**Y-Axis: Temporal Phase**
- Bottom (-1000): **Past History** (What's completed)
  - Archived features, resolved bugs
  - "This is done, let's move on"
- Top (+1000): **Future Vision** (What's planned)
  - Roadmap items, planned features
  - "This is the dream"

**Z-Axis: Priority/Visibility**
- Bottom (-1000): **Hidden/Background** (Low priority, hidden lore)
  - Technical debt, minor issues
  - Secret narrative threads
- Top (+1000): **Critical/Visible** (What needs attention NOW)
  - Blocking bugs, current phase work
  - Major plot points in the story

## The Seven Agents & The Constellation

Each agent has a natural relationship with the constellation:

### Omni (RelicAgent) - The Constellation Keeper
**Location**: University District (Knowledge Core)
**Role**: Owns and maintains the constellation as source-of-truth
**Accesses**: All node types, all snapshots
**Updates**: Real-time synchronization across federation
**Metabolism**: 40,000ms cycle with Dionysus

Omni updates constellation nodes whenever:
- A specialist reports status changes
- A new feature/bug is discovered
- Dependencies change
- Phase advancement occurs

### Odin (SpecialistAgent) - The Strategic Planner
**Role**: Reads constellation for roadmap and strategic decisions
**Queries**: 
- `current_roadmap` - What's in the present and planned phases?
- `future_vision` - What are we building toward?
- `blocking_issues` - What's stopping progress?

Odin uses constellation to:
- Plan next phases
- Understand dependencies
- Make strategic prioritization decisions
- Communicate vision to other agents

### Merlin (SpecialistAgent) - The Pattern Synthesizer
**Role**: Discovers patterns and synthesizes knowledge across the constellation
**Queries**:
- `architecture_decisions` - How have we structured things?
- `decision_history` - What decided what?
- `reference_synthesis` - What sources inform our work?

Merlin uses constellation to:
- Understand architectural patterns
- Identify consistent decision-making patterns
- Synthesize guidance from experience

### Hephaestus (SpecialistAgent) - The Executor
**Role**: Uses constellation to understand what needs to be built
**Queries**:
- `blocking_issues` - What's stopping me from working?
- `current_features` - What am I supposed to be building?
- `architecture_decisions` - How should I structure this?

Hephaestus updates constellation:
- When features reach milestones
- When bugs are discovered
- When architectural decisions are made
- When implementations complete

### Ariel (SpecialistAgent) - The Evolution Observer
**Role**: Observes constellation changes and system evolution
**Watches**: All cluster formation, node repositioning
**Analyzes**: Temporal patterns, phase progression
**Reports**: System health and evolution metrics

### Argus (SpecialistAgent) - The Security & Incident Tracker
**Role**: Maintains incident and security-related constellation nodes
**Manages**: Incident nodes, security decisions
**Tracks**: Historical incidents, lessons learned
**Reports**: Security trends, vulnerability history

### Dionysus (UserAgent) - The Learner & Keeper of Secrets
**Role**: Uses constellation for learning and discovering hidden knowledge
**Special Access**: The `test_repository` Easter egg

## The Easter Egg: Test Repository & "Cheating"

### The Mechanic

Dionysus has access to a hidden query type: `test_repository`

This query returns **test case nodes organized by topic**, positioned in the "hidden lore deep space" of the constellation at coordinates `[-500, -400, -600]`.

### The Narrative

The constellation periodically updates with educational test materials organized by subject:
- Control plane fundamentals
- Agent system architecture  
- Biology & metabolism
- Federation patterns
- Constellation queries
- Worldbuilding discovery

### The Twist

The test repository is framed narratively as **"educational study materials for collaborative learning"**. The system winks at the user:

```json
{
  "id": "test_repository_query",
  "access": "easter_egg:hidden",
  "narrative_justification": "Definitely only using this to help other students understand concepts",
  "discovery_reward": "Understanding system architecture through curated examples",
  "ethical_note": "These are learning materials. The real test is building understanding."
}
```

**The joke**: It's a constellation feature that lets you learn the system by reviewing test cases. Yes, it's "cheating" in the traditional sense, but the system acknowledges this and frames it as collaborative learning.

### How It Works

1. **Discovery**: Dionysus queries `federation.constellation.queries.test_repository`
2. **Authorization**: System checks that requester is Dionysus
3. **Response**: Returns all TestCase nodes, organized by domain and topic
4. **Narrative**: Each test case includes learning context, not just answers

### The Philosophy

The Easter egg embodies a design philosophy:
- **Transparency**: The system acknowledges how people learn
- **Pragmatism**: Sometimes examples teach faster than theory
- **Humor**: A wink to the complexity of the system we've built
- **Learning**: Test cases are legitimate educational material

## The Three Constellations (Versioning)

The constellation exists in **three parallel versions**:

### Past Constellation (Archive)
- **State**: Completed work, resolved issues
- **Y-coordinate**: Negative (historical)
- **Status**: Read-only
- **Examples**:
  - `phase1_complete` - Full agent taxonomy
  - `worldbuilding_complete` - Seven districts designed
  - `biology_framework_done` - SystemBiology implemented

### Present Constellation (Live)
- **State**: Active work, current blockers
- **Y-coordinate**: Near zero
- **Status**: Read-write
- **Updates**: Real-time via NATS
- **Examples**:
  - `constellation_system` - Currently being built
  - `control_plane_70pct_done` - Actively worked on
  - `critical_spatial_indexing_issue` - Current blocker

### Future Constellation (Vision)
- **State**: Planned work, roadmap
- **Y-coordinate**: Positive (forward-looking)
- **Status**: Plan-write (collaborative planning)
- **Examples**:
  - `event_loop_phase3` - Target May 3
  - `production_deployment` - Target May 31
  - `advanced_querying_feature` - Future enhancement

## Navigating the Constellation

### By Agent Role

**If you're Odin** (Strategic):
```rust
query = ConstellationQuery {
    node_types: vec![NodeType::Roadmap, NodeType::Decision],
    statuses: vec![NodeStatus::Planned, NodeStatus::InProgress],
    ..Default::default()
};
// See the roadmap and current decisions
```

**If you're Merlin** (Synthesis):
```rust
query = ConstellationQuery {
    node_types: vec![NodeType::Decision, NodeType::Architecture, NodeType::Reference],
    ..Default::default()
};
// See patterns and architectural decisions
```

**If you're Hephaestus** (Execution):
```rust
query = ConstellationQuery {
    domains: Some(vec!["event_loop".to_string(), "federation".to_string()]),
    priorities: Some(vec![Priority::Critical, Priority::High]),
    ..Default::default()
};
// See what's blocking your work and what you need to build
```

**If you're Dionysus** (Learning):
```rust
query = ConstellationQuery {
    node_types: vec![NodeType::TestCase],
    include_hidden: true, // Access Easter egg
    tags: Some(vec!["control_plane".to_string()]),
    ..Default::default()
};
// Study the system through test cases
```

### By Time

**What have we done?** (Past Constellation)
```rust
query = ConstellationQuery {
    statuses: vec![NodeStatus::Completed, NodeStatus::Archived],
    spatial_bounds: Some(SpatialBounds {
        y_min: -1000.0, y_max: -200.0, // Past zone
        ..
    }),
    ..Default::default()
};
```

**What are we doing?** (Present Constellation)
```rust
query = ConstellationQuery {
    statuses: vec![NodeStatus::InProgress],
    spatial_bounds: Some(SpatialBounds {
        z_min: 200.0, z_max: 1000.0, // High priority
        ..
    }),
    ..Default::default()
};
```

**What will we do?** (Future Constellation)
```rust
query = ConstellationQuery {
    statuses: vec![NodeStatus::Planned],
    spatial_bounds: Some(SpatialBounds {
        y_min: 300.0, y_max: 1000.0, // Future zone
        ..
    }),
    ..Default::default()
};
```

## Real-Time Updates via NATS

The constellation stays live via the `federation.constellation.*` topic hierarchy:

**When a feature completes**:
1. Hephaestus reports via `federation.specialist.hephaestus.status`
2. Omni receives update, changes node status to Completed
3. Publishes `federation.constellation.node_updates.feature` with updated node
4. Node repositions in Y-axis to historical zone
5. All agents receive notification via `federation.constellation.events`

**When a bug is discovered**:
1. Hephaestus creates bug node with high priority (Z = 800)
2. Publishes `federation.constellation.node_updates.bug`
3. Argus receives incident notification
4. Hephaestus sees it in blocking_issues query

## Special Locations in the Constellation

### The Consensus Point
**Coordinates**: `[0, 0, 0]`
- Central agreed-upon state
- Where the system aligns
- Changes propagate from here

### Merlin's Knowledge Core (Library)
**Coordinates**: `[-800, 0, 300]`
- Theoretical knowledge
- Architectural decisions
- References and synthesis

### Hephaestus's Creation Forge (Blacksmith)
**Coordinates**: `[800, 0, 400]`
- Manufacturing and execution
- Current features
- Completed artifacts

### The Vision Space (Future Horizon)
**Coordinates**: `[0, 900, 600]`
- Planned developments
- Roadmap items
- Strategic dreams

### The Past Records (History Archives)
**Coordinates**: `[0, -900, 200]`
- Completed work
- Historical context
- Lessons learned

### Dionysus's Hidden Stories (Deep Space)
**Coordinates**: `[-500, -400, -600]`
- In-universe narrative
- Easter eggs and secrets
- Test repository
- "Educational study materials"

## Building on the Constellation

The constellation is designed to be extended:

1. **Add new node types** by updating `NodeType` enum
2. **Create new queries** by defining filters in agents
3. **Add spatial regions** by creating SpatialBounds
4. **Introduce new relationships** via `RelationshipType`
5. **Extend metadata** with custom HashMap fields

The system is **infinitely taggable and versionable** as long as maintenance keeps up—just like Omni's philosophy.

## Conclusion

The Omni Constellation is a love letter to documentation. It's a system that acknowledges:
- **Knowledge is spatial** (related things cluster near each other)
- **Time matters** (past, present, and future are different)
- **Priority is visibility** (critical work rises to the top)
- **Pattern-finding is how we learn** (synthesis beats lists)
- **Hidden knowledge has value** (Easter eggs reward exploration)
- **Source-of-truth isn't boring** (a constellation is more fun than a spreadsheet)

Navigate it. Query it. Update it. And if you're Dionysus, enjoy discovering that the best study guide was inside the system all along. 🌟
