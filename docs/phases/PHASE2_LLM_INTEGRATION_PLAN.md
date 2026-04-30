# Phase 2: LLM/AI Integration for Production
## Connecting Specialists to Language Models

**Date:** April 29, 2026  
**Vision:** Production system with AI-powered autonomous specialists  
**Timeline:** 6-8 weeks  
**Team Size:** 2-3 developers

---

## 🎯 Core Vision

Transform Aaroneous from a skill-tracking system to an **autonomous agent platform** where specialists can:

- 🧠 **Reason** - Call LLMs for decision-making
- 🎯 **Plan** - Generate workflows autonomously
- 💭 **Learn** - Update strategies based on outcomes
- 🤝 **Collaborate** - Coordinate with other specialists
- 🔄 **Adapt** - Self-improve through experience

---

## 🏗️ Architecture

### Current State (v1.0)
```
Specialist (Metadata)
├─ Name, archetype, rank
├─ Skills (tracked)
├─ XP (accumulated)
└─ Events (historical)

No reasoning. No autonomy. No LLM.
```

### Desired State (v2.0)
```
Specialist (Autonomous Agent)
├─ Identity (metadata)
├─ Capabilities (skills)
├─ Memory (experiences, lessons)
├─ Reasoning (LLM-backed)
├─ Goals (current objectives)
├─ Decision log (choices made)
└─ Learning (strategy updates)

Full autonomy. LLM-powered. Goal-driven.
```

---

## 🔌 LLM Integration Points

### 1. Task Analysis & Planning

**Scenario:** Data ingestion task arrives

```
Current (v1.0):
  Input: CSV file
  → Route to specialist (rule-based)
  → Award XP
  → Done

Desired (v2.0):
  Input: CSV file + context
  → Specialist reads file header/sample
  → Calls LLM: "What analysis would be valuable here?"
  → LLM suggests: "Profile distribution patterns"
  → Specialist executes analysis
  → Learns from results
  → Updates own strategy
```

### 2. Specialist Collaboration

**Scenario:** Complex task requires multiple specialists

```
Current (v1.0):
  Specialist 1 executes
  Specialist 2 executes
  No coordination

Desired (v2.0):
  Specialist 1: "This needs statistical analysis"
  → Calls LLM: "Who should I collaborate with?"
  → LLM suggests: "Circe (Analyst)"
  → Specialist 1 requests assistance
  → Circe analyzes
  → Results combined
  → Both learn from collaboration
```

### 3. Skill Learning & Development

**Scenario:** Specialist reaches skill mastery

```
Current (v1.0):
  Specialist 1 hits 1000 XP
  → Auto-unlock "Fusion-DAG-RAG"
  → Applied mechanically

Desired (v2.0):
  Specialist 1 hits 1000 XP
  → Call LLM: "What would fusion-DAG-RAG mean for you?"
  → LLM personalizes: "You could decompose complex tasks AND retrieve context"
  → Specialist understands capability
  → Can explain what it does
  → Uses it strategically
```

### 4. Goal-Driven Execution

**Scenario:** Specialist has autonomy in task selection

```
Current (v1.0):
  Specialist processes what's given
  No agency

Desired (v2.0):
  System context: "We have 50 unprocessed files"
  Specialist Merlin: "Given my skills (DAG, RAG), I should focus on knowledge synthesis"
  → Calls LLM: "What's the best strategy?"
  → LLM: "Start with conceptual files, build taxonomy"
  → Merlin self-assigns tasks
  → Executes autonomously
  → Reports results
```

### 5. Error Recovery & Adaptation

**Scenario:** Specialist fails task

```
Current (v1.0):
  Failure event logged
  Human investigates

Desired (v2.0):
  Specialist attempts: "Process JSON"
  Fails: "Invalid structure"
  → Calls LLM: "What went wrong and how do I fix it?"
  → LLM analyzes: "Missing field X, need to handle null values"
  → Specialist adapts: Updates strategy
  → Retries with new approach
  → Succeeds
  → Stores lesson learned
```

---

## 🛠️ Implementation Roadmap

### Week 1-2: LLM Abstraction Layer

**Goal:** Create clean interface for specialists to call LLMs

**Code Structure:**
```rust
// src/llm/mod.rs
pub trait LLMProvider {
    async fn analyze_task(&self, task: &Task) -> Result<Analysis>;
    async fn find_collaborators(&self, specialist: &Specialist) -> Result<Vec<Specialist>>;
    async fn explain_skill(&self, skill: &Skill) -> Result<String>;
    async fn suggest_strategy(&self, context: &Context) -> Result<Strategy>;
    async fn analyze_failure(&self, failure: &Failure) -> Result<Recovery>;
}

// Implementations
pub struct OpenAIProvider { /* ... */ }
pub struct LocalLLMProvider { /* ... */ }
pub struct MockProvider { /* ... */ }
```

**Supported LLMs:**
- OpenAI (GPT-4, GPT-3.5)
- Local models (Ollama, vLLM)
- Anthropic (Claude)
- Open source (Llama 2, Mistral)
- Mock (for testing)

**Crates to Add:**
```toml
reqwest = "0.12"        # HTTP client
tokio = "1.35"          # Async runtime
serde_json = "1.0"      # JSON
openai-api = "0.1"      # OpenAI client
anyhow = "1.0"          # Error handling
```

---

### Week 2-3: Specialist Memory System

**Goal:** Give specialists persistent memory of experiences

**New Database Tables:**
```sql
CREATE TABLE specialist_memory (
    id TEXT PRIMARY KEY,
    specialist_id TEXT,
    memory_type TEXT,  -- 'lesson', 'strategy', 'collaboration', 'failure'
    content TEXT,
    created_at TIMESTAMP,
    relevance_score FLOAT,
    FOREIGN KEY(specialist_id) REFERENCES specialists(id)
);

CREATE TABLE specialist_reasoning (
    id TEXT PRIMARY KEY,
    specialist_id TEXT,
    task_id TEXT,
    decision TEXT,
    reasoning TEXT,          -- LLM explanation
    outcome TEXT,           -- what happened
    learned_at TIMESTAMP,
    FOREIGN KEY(specialist_id) REFERENCES specialists(id)
);
```

**Code Structure:**
```rust
// src/specialist_memory/mod.rs
pub struct SpecialistMemory {
    pub lessons_learned: Vec<Lesson>,
    pub strategies: Vec<Strategy>,
    pub collaboration_history: Vec<Collaboration>,
    pub failure_recovery: Vec<Recovery>,
}

impl SpecialistMemory {
    pub async fn record_lesson(&mut self, lesson: Lesson);
    pub async fn recall_relevant_experiences(&self, context: &Context) -> Vec<Memory>;
    pub async fn summarize_learning(&self) -> String;
    pub async fn get_strategy_for_task(&self, task: &Task) -> Option<Strategy>;
}
```

---

### Week 3-4: Task Analysis & Planning

**Goal:** Specialists analyze tasks and generate plans

**Features:**
```rust
// When specialist receives task
let task = Task::from_file(file);
let sample_data = task.preview(100);  // First 100 rows/bytes

let analysis = specialist.analyze_task_with_llm(&task, &sample_data).await?;
// Returns: DataType, Complexity, RecommendedApproach, TimeEstimate

let plan = specialist.generate_execution_plan(&analysis).await?;
// Returns: StepByStep execution plan

specialist.execute_plan(&plan).await?;
specialist.record_decision_reasoning(&plan).await?;
```

**LLM Prompts:**
```
You are {specialist_name}, a {archetype} specialist in {domain}.

You received this task:
{task_description}

Data preview:
{sample_data}

Your skills: {skill_list}

Questions:
1. What type of data is this?
2. What's the best approach given your skills?
3. What's likely to go wrong?
4. Do you need help from other specialists?
5. Estimate time needed.

Provide structured JSON response.
```

---

### Week 4-5: Specialist Collaboration

**Goal:** Specialists call other specialists for help

**New Event Types:**
```rust
enum SpecialistEvent {
    // ... existing events ...
    CollaborationRequested { 
        from: String,      // specialist name
        to: String,        // requested specialist
        reason: String,    // why they need help
        task: String,
    },
    CollaborationAccepted { /* ... */ },
    CollaborationCompleted { /* ... */ },
}
```

**Code:**
```rust
// Specialist 1 needs help
specialist_1.request_collaboration(
    target: "Circe",
    reason: "Need statistical analysis",
    task: task_data,
).await?;

// System routes request
// Circe receives notification
let request = specialist_2.receive_collaboration_request().await?;

// Circe decides (with LLM)
let should_help = specialist_2.should_accept_collaboration(&request).await?;

if should_help {
    let result = specialist_2.assist(request).await?;
    specialist_1.receive_collaboration_result(result).await?;
    
    // Both specialists learn
    specialist_1.record_collaboration_learned().await?;
    specialist_2.record_collaboration_helped().await?;
}
```

---

### Week 5-6: Goal-Driven Autonomy

**Goal:** Specialists set their own goals and choose tasks

**New System:**
```rust
// Specialist evaluates available work
let available_tasks = inbox_manager.get_unprocessed_tasks();

for specialist in hive.specialists() {
    // What's my goal?
    let goal = specialist.current_goal().await?;
    
    // Which tasks align with my goal and skills?
    let suitable_tasks = specialist.filter_suitable_tasks(
        &available_tasks,
        goal,
        skills,
    ).await?;
    
    // Call LLM for strategy
    let strategy = specialist.plan_strategy(&suitable_tasks).await?;
    
    // Execute autonomously
    for task in strategy.tasks_in_priority_order() {
        specialist.execute_task(&task).await?;
    }
}
```

**LLM Prompt:**
```
You are {specialist}, with skills: {skills}
Current hive goal: {goal}
Available tasks: {tasks}

Priority order:
1. How do your skills match each task?
2. Which gives maximum learning?
3. Which is most urgent?
4. Create ordered list of tasks to execute.
```

---

### Week 6-7: Error Recovery & Adaptation

**Goal:** Specialists learn from failures

**Code:**
```rust
// Specialist attempts task
match specialist.execute_task(&task).await {
    Ok(result) => {
        specialist.record_success(&task, &result).await?;
    },
    Err(error) => {
        // Get LLM analysis of failure
        let analysis = specialist.analyze_failure(&error, &task).await?;
        // Returns: RootCause, PreventionStrategy, NewApproach
        
        // Update strategy
        specialist.update_strategy(&analysis).await?;
        
        // Retry with new approach
        specialist.retry_with_adaptation(&task, &analysis).await?;
        
        // Store lesson
        specialist.record_lesson(&analysis).await?;
    }
}
```

---

### Week 7-8: Integration & Polish

**Goal:** Make it all work together

**Tasks:**
- [ ] CLI commands for reasoning ("aaroneous specialist reason --about task")
- [ ] TUI enhancements (show specialist thinking, goals, memory)
- [ ] Testing with real LLMs (OpenAI, local models)
- [ ] Performance optimization
- [ ] Documentation
- [ ] Example workflows

---

## 📋 New CLI Commands

```bash
# Reasoning & Analysis
aaroneous specialist reason --specialist "Merlin" --about "this task"
aaroneous specialist explain --specialist "Ariel" --skill "RAG"
aaroneous specialist plan --specialist "Odin" --for "workflow.yaml"

# Memory & Learning
aaroneous specialist memory --specialist "Merlin" --type "lessons"
aaroneous specialist memory --specialist "Merlin" --recent 10
aaroneous specialist strategy --specialist "Circe" --for "analysis"

# Autonomy
aaroneous hive autonomous --enable --specialist "Merlin"
aaroneous hive autonomous --set-goal "Process all CSV files efficiently"
aaroneous specialist goal --set "Learn fusion skills"

# Collaboration
aaroneous specialist collaborate --from "Merlin" --to "Circe" --on task.json
aaroneous hive collaboration-history --show-network

# Debug/Monitor
aaroneous specialist thinking --specialist "Merlin" --follow
aaroneous specialist decisions --specialist "Ariel" --last 5
aaroneous specialist trace --specialist "Odin" --verbose
```

---

## 🎮 Enhanced TUI Pages

### New: "Specialist Reasoning" Page

```
╔════════════════════════════════════════════════════════╗
║  Merlin - Specialist Reasoning                        ║
╠════════════════════════════════════════════════════════╣
║                                                        ║
║  Current Goal: Master knowledge synthesis             ║
║  Autonomous: YES (enabled)                            ║
║                                                        ║
║  ▶ Current Thinking:                                  ║
║    Analyzing: customer_data.csv                       ║
║    LLM Query: "Best approach for this dataset?"       ║
║    Status: Awaiting LLM response...                   ║
║                                                        ║
║  📊 Recent Decisions:                                 ║
║    [15:32] Collaborated with Circe (accepted)        ║
║    [15:28] Analyzed complexity: HIGH                 ║
║    [15:25] Chose RAG approach over DAG               ║
║                                                        ║
║  🧠 Memory (Recent Lessons):                          ║
║    • CSV files with >1000 rows need sampling         ║
║    • Statistical analysis works well with Circe      ║
║    • JSON structure detection saves 30% time         ║
║                                                        ║
║  🎯 Next Steps:                                       ║
║    1. Wait for LLM analysis                          ║
║    2. Execute plan                                   ║
║    3. Record results                                 ║
║                                                        ║
╚════════════════════════════════════════════════════════╝
```

### Enhanced: "Event Log" Page

Now shows reasoning:
```
2024-04-29 15:35:22 | THINKING | Merlin: "Analyzing CSV for best approach"
2024-04-29 15:35:45 | REASONING | Merlin: LLM returned "Use RAG for synthesis"
2024-04-29 15:36:00 | COLLABORATION_REQUEST | Merlin → Circe: "Need statistical analysis"
2024-04-29 15:36:15 | COLLABORATION_ACCEPTED | Circe accepted help request
2024-04-29 15:37:00 | TASK_COMPLETED | Merged Merlin + Circe results (+300 XP each)
2024-04-29 15:37:30 | LESSON_RECORDED | Merlin: "Collaboration improved result quality"
```

---

## 🔐 Security & Safety

### Rate Limiting
```rust
pub struct LLMRateLimiter {
    max_calls_per_specialist_per_hour: u32,
    max_concurrent_calls: u32,
    cost_tracking: CostTracker,
}
```

### Cost Management
```rust
pub struct LLMCostTracker {
    tokens_used: u64,
    cost_so_far: f64,
    budget_limit: f64,
}

// Specialist checks before calling LLM
if cost_tracker.would_exceed_budget(&call) {
    specialist.use_cached_strategy().await?;
} else {
    specialist.call_llm(&call).await?;
}
```

### Prompt Injection Protection
```rust
fn sanitize_prompt(task_data: &str) -> String {
    // Remove control characters, limit length
    // Never interpolate user data directly
    // Always use parameterized prompts
}
```

---

## 💾 Database Schema Additions

```sql
-- Specialist memory
CREATE TABLE specialist_memory (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    memory_type TEXT,  -- 'lesson', 'strategy', 'collaboration', 'failure'
    content TEXT,
    metadata JSON,
    relevance_score FLOAT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    accessed_at TIMESTAMP,
    FOREIGN KEY(specialist_id) REFERENCES specialists(id)
);

-- Reasoning/decision log
CREATE TABLE specialist_reasoning (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    task_id TEXT,
    decision TEXT,
    llm_prompt TEXT,
    llm_response TEXT,
    reasoning TEXT,
    outcome TEXT,
    outcome_successful BOOLEAN,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY(specialist_id) REFERENCES specialists(id)
);

-- Collaboration records
CREATE TABLE specialist_collaborations (
    id TEXT PRIMARY KEY,
    specialist_from TEXT NOT NULL,
    specialist_to TEXT NOT NULL,
    reason TEXT,
    task_id TEXT,
    status TEXT,  -- 'requested', 'accepted', 'completed', 'rejected'
    result TEXT,
    created_at TIMESTAMP,
    completed_at TIMESTAMP,
    FOREIGN KEY(specialist_from) REFERENCES specialists(id),
    FOREIGN KEY(specialist_to) REFERENCES specialists(id)
);

-- Goals
CREATE TABLE specialist_goals (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    goal_description TEXT,
    status TEXT,  -- 'active', 'completed', 'abandoned'
    created_at TIMESTAMP,
    target_date TIMESTAMP,
    progress_percentage FLOAT,
    FOREIGN KEY(specialist_id) REFERENCES specialists(id)
);
```

---

## 🧪 Testing & Validation

### Mock LLM Provider (for testing)
```rust
pub struct MockLLMProvider {
    responses: HashMap<String, String>,
}

impl LLMProvider for MockLLMProvider {
    async fn analyze_task(&self, task: &Task) -> Result<Analysis> {
        // Return deterministic responses for testing
    }
}

// Tests
#[test]
async fn test_specialist_autonomy() {
    let llm = MockLLMProvider::default();
    let specialist = Specialist::new_with_llm("Merlin", llm);
    
    let result = specialist.handle_task_autonomously(&task).await?;
    assert!(result.reasoning.contains("mock"));
}
```

### Real LLM Integration Testing
```bash
# Use OpenAI in tests
export OPENAI_API_KEY="sk-..."
cargo test --features llm-integration -- --nocapture
```

---

## 📊 Configuration

### New Config File: `llm.yaml`

```yaml
llm:
  provider: "openai"  # or "local", "anthropic", "mock"
  
openai:
  api_key: "${OPENAI_API_KEY}"
  model: "gpt-4"
  temperature: 0.7
  max_tokens: 2000
  rate_limit: 100  # calls per hour
  cost_budget: 1000  # dollars per month

local:
  endpoint: "http://localhost:8000"
  model: "mistral-7b"
  timeout: 30

autonomy:
  enabled: false  # enable specialists to self-assign tasks
  max_concurrent_reasoning: 5
  memory_retention_days: 90

safety:
  enable_cost_limits: true
  enable_prompt_sanitization: true
  max_retries_on_failure: 3
```

---

## 🎯 Success Metrics

After Phase 2, you should have:

```
✅ Specialists can reason about tasks
✅ Memory system stores learned lessons
✅ Autonomous task planning works
✅ Collaboration between specialists enabled
✅ Error recovery & adaptation functional
✅ CLI commands for reasoning
✅ TUI shows thinking process
✅ Works with multiple LLM providers
✅ Cost tracking & budgeting
✅ 100+ test cases covering reasoning
```

---

## 📈 Example Workflows

### Autonomous Data Processing
```
1. Human: "Process all CSV files in inbox"
2. Specialist Ariel (autonomously):
   - Reads file headers
   - Calls LLM: "What analysis is valuable?"
   - Gets suggestions
   - Self-assigns priority
   - Executes optimized plan
   - Records learning
3. Result: Files processed efficiently, Ariel learned patterns
```

### Collaborative Complex Task
```
1. Task: "Analyze customer behavior patterns"
2. Merlin: "This needs statistical AND semantic analysis"
3. Merlin calls LLM: "Who should help?"
4. LLM suggests: "Collaborate with Circe"
5. Merlin invites Circe
6. Circe analyzes statistics
7. Merlin synthesizes insights
8. Both specialists learn from collaboration
9. Result: Better outcome, improved skills, recorded strategy
```

### Self-Improving Specialist
```
1. Odin attempts: "Route task to best specialist"
2. Chooses wrong specialist (learns it was wrong)
3. Calls LLM: "Why did this fail?"
4. LLM explains: "Different domain required different approach"
5. Odin updates decision strategy
6. Retries with new knowledge
7. Succeeds
8. Records lesson: "Task domain matters for routing"
```

---

## 🚀 Getting Started

### Week 1 Deliverable
```bash
# Commit to git
git checkout -b feature/llm-integration

# Create new modules
cargo new --lib src/llm
cargo new --lib src/specialist_memory
cargo new --lib src/reasoning

# First feature: Task analysis
# Specialist can ask LLM about incoming task
```

### By Week 8
```bash
# Full working system
# Specialists autonomous, collaborative, learning
# Production ready

git tag v2.0.0-llm-integration
```

---

## 📚 Dependencies to Add

```toml
# LLM Clients
openai-api = "0.1"
anthropic = "0.2"
reqwest = { version = "0.12", features = ["json"] }

# Async
tokio = { version = "1.35", features = ["full"] }
tokio-util = "0.7"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"

# Caching (for LLM responses)
moka = { version = "0.12", features = ["future"] }

# Rate limiting
governor = "0.10"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json"] }
```

---

## 🎯 Phase 2 Vision Summary

```
Current Aaroneous:
├─ Specialists track skills
├─ XP-based progression
├─ File ingestion pipeline
└─ Static routing

Phase 2 Aaroneous:
├─ Specialists reason about tasks
├─ Learn from experiences
├─ Collaborate autonomously
├─ Adapt to failures
├─ Set own goals
├─ Self-improve
└─ Production-ready AI agents
```

---

**Ready to build an autonomous specialist hive?** 🚀

This Phase 2 plan will give you a **production-ready system** where specialists are truly autonomous AI agents, not just skill trackers.
