# Aaroneous Phase 2 v2.0 - Operational Guide

## Overview

Aaroneous v2.0 is a fully autonomous specialist hive with local LLM reasoning, memory-driven learning, and collaborative problem-solving. This guide covers operational aspects.

**Version**: 2.0.0  
**Status**: Production-Ready  
**Tests**: 230/230 passing  
**Build Time**: ~15 seconds  
**Test Runtime**: 1.17 seconds

---

## System Architecture

### Core Components

1. **HiveRuntime** - Main orchestrator with event loop
2. **AutonomousCoordinator** - Task pipeline management
3. **TaskAnalysisEngine** - LLM-powered task reasoning
4. **CapabilityMatchingEngine** - Specialist-to-task scoring
5. **AutonomousPlanningEngine** - Execution plan generation
6. **ErrorRecoveryEngine** - Failure analysis & recovery
7. **SpecialistCollaborationEngine** - Peer-to-peer help
8. **GoalDrivenAutonomyEngine** - Self-directed goal pursuit
9. **SpecialistMemory** - Experience persistence
10. **MemoryReflectionEngine** - Learning from outcomes

### Data Flow

```
Task Submit
    ↓
Analysis (LLM reasoning)
    ↓
Matching (Capability scoring)
    ↓
Planning (Execution steps + contingencies)
    ↓
Execution (Track progress)
    ↓
Error Detection
    ↓
Recovery (Adapt strategy)
    ↓
Collaboration (Request help if needed)
    ↓
Goal Update (Progress tracking)
    ↓
Memory Reflection (Extract lessons)
    ↓
Persistence (Save for future)
```

---

## Configuration

### Environment Variables

```bash
# LLM Configuration
config.toml [llm] section=GGUF           # GGUF or Mock
AARONEOUS_LLM_TEMPERATURE=0.7         # 0.0-1.0
AARONEOUS_LLM_MAX_TOKENS=2048         # Max response tokens
AARONEOUS_LLM_TIMEOUT_SECS=30         # Request timeout
AARONEOUS_MODELS_PATH=/path/to/models # Model search path

# Runtime Configuration
config.toml [database] section=./hive.db           # SQLite database
AARONEOUS_INBOX_FOLDER=./inbox        # Task input folder
AARONEOUS_OUTPUT_FOLDER=./output      # Results output folder
AARONEOUS_UPDATE_INTERVAL_MS=100      # Event loop interval
AARONEOUS_MAX_CONCURRENT_TASKS=4      # Parallel task limit
AARONEOUS_ENABLE_PERSISTENCE=true     # Save to database
AARONEOUS_ENABLE_INGESTION=true       # File monitoring
AARONEOUS_ENABLE_DASHBOARD=true       # TUI dashboard

# Model Discovery
AARONEOUS_AUTO_DISCOVERY=true         # Enable auto-discovery
AARONEOUS_FALLBACK_PROVIDER=Mock      # Fallback if GGUF unavailable
```

### Model Discovery Paths (Auto-Detected)

1. `~/.lm-studio/models`
2. `~/AppData/Local/LM Studio/models`
3. `~/.ollama/models`
4. `./models`
5. `../models`
6. `/opt/local-ai/models`
7. `C:/LM Studio/models`
8. `D:/models`

---

## Task Submission

### Task Structure

```rust
Task {
    id: "task-unique-id",
    name: "Analyze Customer Data",
    description: "Process and classify sentiment",
    data_sample: Some("sample data"),
    priority: TaskPriority::High,
    deadline_secs: Some(300),
    required_skills: vec!["data_analysis", "nlp"],
    tags: vec!["analysis", "customer"],
}
```

### Priority Levels

- **Low** (1) - Background tasks, flexible deadline
- **Normal** (2) - Standard workload
- **High** (3) - Important, needs attention
- **Critical** (4) - Immediate action required

### Task Submission Flow

```bash
# Via HiveRuntime.submit_task()
let task_id = runtime.submit_task(task).await?;

# Task enters pipeline:
# 1. Submitted → waiting in queue
# 2. Analyzing → LLM analyzes approach
# 3. Analysis Complete → matched to specialists
# 4. Matching → scoring specialists
# 5. Matching Complete → top specialist selected
# 6. Planning → creating execution steps
# 7. Planning Complete → ready to execute
# 8. Executing → steps in progress
# 9. Completed → task finished
# Or: Failed → error recovery triggered
```

---

## Specialist Matching

### Scoring Factors

| Factor | Weight | Calculation |
|--------|--------|-------------|
| Skill Match | 40% | Exact + partial matches |
| Experience | 30% | XP vs. requirement level |
| Availability | 20% | Current task load |
| Learning Potential | 10% | Missing skills vs. learn rate |

### Match Score Range

- **0.0 - 0.3**: Poor fit, consider escalation
- **0.3 - 0.6**: Adequate, may need help
- **0.6 - 0.8**: Good fit, should succeed
- **0.8 - 1.0**: Excellent fit, high success probability

### Example Matching

```
Task: Analyze financial data (needs SQL + Statistics)

Specialist Scores:
1. Merlin (Data Expert)    - 0.92 (SQL 95%, Stats 92%)
2. Ariel (UI Designer)     - 0.45 (SQL 20%, Stats 10%)
3. Odin (Systems Expert)   - 0.65 (SQL 40%, Stats 70%)

Decision: Assign to Merlin, offer Odin as consultant
```

---

## Execution Planning

### Plan Structure

```rust
AutonomousPlan {
    plan_id: "plan-123",
    task_id: "task-456",
    primary_specialist: "Merlin",
    steps: vec![
        ExecutionStep { sequence: 1, action: "Load data", ... },
        ExecutionStep { sequence: 2, action: "Parse CSV", ... },
        ExecutionStep { sequence: 3, action: "Validate schema", ... },
        // ... 5-7 more steps
    ],
    estimated_duration_minutes: 45,
    success_probability: 0.87,
    contingencies: vec![
        Contingency { trigger: "Timeout", action: "Chunk data" },
        Contingency { trigger: "MemoryFull", action: "Stream processing" },
    ],
}
```

### Step Execution

Each step includes:
- **Sequence**: Order (1, 2, 3...)
- **Action**: What to do
- **Expected Outcome**: What success looks like
- **Estimated Time**: Duration in minutes
- **Validation**: Checks before proceeding

### Contingencies

Automatically generated for:
- **Timeout Exceeded** → Increase timeout, chunk processing
- **Resource Exhaustion** → Allocate more memory, free cache
- **Skill Gap Found** → Request help, acquire skill
- **Data Format Mismatch** → Apply transformation, fallback format
- **External Service Failed** → Retry with backoff, use cache

---

## Error Recovery

### Error Detection

System automatically detects 8 error types:

1. **ResourceExhaustion** - Memory/CPU/disk full
2. **TimeoutExceeded** - Operation took too long
3. **InvalidInput** - Bad data format
4. **ExternalServiceFailed** - API/DB unavailable
5. **SkillGapFound** - Specialist lacks required skill
6. **DataFormatMismatch** - Input/output incompatibility
7. **ConcurrencyConflict** - Race condition/deadlock
8. **UnexpectedFailure** - Unknown error

### Recovery Strategy

```
Error Detected
    ↓
Root Cause Analysis (LLM reasoning)
    ↓
Contributing Factors Extracted
    ↓
Recovery Strategy Generated (3-5 actions)
    ↓
Retry Logic Applied
    ├─ Attempt 1: Immediate retry
    ├─ Attempt 2: After 2 seconds
    ├─ Attempt 3: After 4 seconds
    └─ Attempt 4: After 8 seconds
    ↓
Escalation (if all retries fail)
    ├─ Collaboration request
    ├─ Human alert
    └─ Task marked failed
    ↓
Memory Recording (lesson saved)
```

### Retry Backoff

Exponential backoff: `delay = 2^attempt seconds`

- Attempt 0: 1 second
- Attempt 1: 2 seconds
- Attempt 2: 4 seconds
- Attempt 3: 8 seconds
- Max attempts: 3 (configurable)

---

## Specialist Collaboration

### Help Request System

```rust
HelpRequest {
    request_id: "help-req-123",
    requester_id: "specialist-1",
    task_id: "task-456",
    skill_needed: "Rust Async",
    challenge_description: "Ownership rules unclear",
    urgency: Urgency::High,
    timestamp: now(),
}
```

### Urgency Levels

- **Low** - Can wait, background task
- **Medium** - Within normal workflow
- **High** - Priority, blocks other work
- **Critical** - Immediate attention required

### Response Types

| Type | Use Case |
|------|----------|
| DirectHelp | Specialist takes over the task |
| Consultation | Advice & guidance on approach |
| Mentoring | Teaching the skill for growth |
| ResourceSharing | Provide tools/data/code |
| Delegation | Full task handoff |

### Collaboration Metrics

```rust
CollaborationMetrics {
    help_requests_sent: 5,
    help_requests_received: 3,
    help_requests_accepted: 2,
    collaboration_success_rate: 0.67,
    peers: vec!["spec-2", "spec-3"],
    taught_specialists: vec!["spec-4"],
    learned_from_specialists: vec!["spec-5"],
}
```

---

## Goal-Driven Autonomy

### Goal Categories

1. **SkillDevelopment** - Learn new capability
2. **XPThreshold** - Reach experience level
3. **Collaboration** - Work with peers
4. **Specialization** - Master domain expertise
5. **MentorshipGiving** - Teach others
6. **MentorshipReceiving** - Learn from others
7. **TaskCompletion** - Finish challenging tasks
8. **Innovation** - Create novel solutions

### Goal Status Transitions

```
Planning
    ↓
Active (manually activated)
    ↓
AtRisk (progress < 20%)
    ├→ InProgress (progress reaches 20%)
    └→ InProgress (direct if progress >= 20%)
    ↓
InProgress (20-80%)
    ├→ OnTrack (progress >= 80%)
    └→ AtRisk (if drops < 20%)
    ↓
OnTrack (80-99%)
    ↓
Completed (progress = 100%)
    or
Failed (manually marked)
Cancelled (manually marked)
Paused (on-hold)
```

### Goal Milestones

Each goal can have sub-milestones:

```rust
Milestone {
    id: "m-1",
    name: "Complete basic course",
    target_value: 100.0,
    current_value: 75.0,
    progress_percentage: 75.0,
    completed: false,
}
```

---

## Memory System

### Memory Types

1. **Lesson** - Knowledge learned from experience
2. **Strategy** - Effective approach for problem class
3. **Decision** - Record of choice made
4. **Reflection** - Self-analysis of performance
5. **Goal** - Objective being pursued

### Memory Sources

- **Experience** - Learned by doing
- **LLMReasoning** - Insights from LLM
- **PeerLearning** - Learned from collaborators
- **Configuration** - Explicitly provided
- **ErrorRecovery** - Learned from failure

### Memory Operations

```rust
// Record memory
memory.record_memory(entry);

// Search by tag
let memories = memory.search_memories("async");

// Get active goals
let goals = memory.get_active_goals();

// Get best strategy for problem
let strategy = memory.get_best_strategy("async");

// Record decision
memory.record_decision(task_id, choice, reasoning);

// Calculate memory health
let health = memory.calculate_health(); // 0.0-1.0
```

### Memory Persistence

All memory is saved to SQLite:

- **memory_entries** table (5,000+ entries)
- **decision_records** table (1,000+ records)
- **strategies** table (200+ strategies)
- **goals** table (100+ active goals)

---

## Monitoring & Observability

### Metrics Collected

| Metric | Unit | Description |
|--------|------|-------------|
| tasks_submitted | count | Total tasks submitted |
| tasks_completed | count | Successfully completed |
| tasks_failed | count | Failed, not recovered |
| avg_completion_time | seconds | Average task duration |
| success_rate | percentage | Completed / submitted |
| specialist_xp | points | Accumulated experience |
| memory_entries | count | Total memories |
| collaboration_index | 0.0-1.0 | Team collaboration score |

### Logging

All operations logged with tracing:

```rust
info!("Task submitted: {}", task_id);
debug!("Analyzing task...");
warn!("Specialist gap found, requesting help");
error!("Task failed: {}", error);
```

### Health Checks

```bash
# Check system health
runtime.health_check().await // bool

# Get statistics
let stats = runtime.get_statistics().await;
println!("Uptime: {} seconds", stats.uptime_seconds);
println!("Total specialists: {}", stats.total_specialists);
```

---

## Performance Tuning

### Concurrency Settings

```rust
HiveRuntimeConfig {
    max_concurrent_tasks: 4,      // Increase for parallel workload
    update_interval_ms: 100,      // Lower for faster response
    // ...
}
```

### Memory Optimization

- Memory health check: `memory.calculate_health()`
- Delete old memories: Keep recent 5,000 entries
- Archive completed goals: Move to history table

### LLM Optimization

```rust
LLMConfig {
    enable_caching: true,         // Cache responses
    cache_ttl_secs: 3600,        // 1 hour TTL
    max_tokens: 2048,            // Reasonable limit
    temperature: 0.7,            // Balanced creativity
}
```

---

## Troubleshooting

### Issue: Tasks stuck in "Analyzing" state

**Cause**: LLM model unavailable  
**Solution**:
1. Check config.toml [llm] section environment
2. Ensure model file accessible
3. Check LM Studio / Ollama running
4. Review logs for LLM errors

### Issue: Low specialist match scores

**Cause**: Specialists lack required skills  
**Solution**:
1. Request collaboration (automatic)
2. Assign learning goal to specialist
3. Use mentorship to transfer skills
4. Consider task decomposition

### Issue: Memory growing unbounded

**Cause**: Too many memory entries  
**Solution**:
```rust
// Clean up old entries
memory.cleanup_old_entries(keep_recent: 5000);

// Archive completed goals
goals.archive_completed();

// Review memory health
let health = memory.calculate_health();
```

### Issue: Tasks timing out frequently

**Cause**: Slow processing or unrealistic deadline  
**Solution**:
1. Increase deadline_secs in task
2. Enable contingency chunking
3. Review task complexity
4. Check system resources

---

## Best Practices

### Task Design

✅ **DO:**
- Break large tasks into smaller steps
- Provide data samples for analysis
- Set realistic deadlines
- Use priority levels appropriately

❌ **DON'T:**
- Submit identical duplicate tasks
- Set 0-second deadlines
- Overload with 100+ concurrent tasks
- Mix unrelated requirements

### Specialist Management

✅ **DO:**
- Review specialist XP progress
- Assign learning goals
- Enable collaboration
- Check memory health

❌ **DON'T:**
- Ignore specialist skill gaps
- Isolate specialists (no collaboration)
- Let memory grow unbounded
- Skip error recovery setup

### Goal Setting

✅ **DO:**
- Set specific, measurable goals
- Break into milestones
- Monitor progress regularly
- Celebrate completions

❌ **DON'T:**
- Set vague goals
- Create impossible targets
- Ignore blocked goals
- Mix unrelated sub-goals

---

## CLI Commands

```bash
# Start hive
aaroneous start

# Submit task
aaroneous task submit --name "Analyze Data" --priority high

# Check specialist status
aaroneous specialist status

# View memory stats
aaroneous memory stats

# Run dashboard
aaroneous dashboard

# Check system health
aaroneous status health
```

---

## Version 2.0 Features

✅ LLM-powered task analysis  
✅ Autonomous specialist matching  
✅ Intelligent execution planning  
✅ Error recovery & learning  
✅ Specialist collaboration  
✅ Goal-driven autonomy  
✅ Memory-driven decision making  
✅ Concurrent task processing  
✅ 230 passing tests  
✅ Production-ready

---

## Support & Troubleshooting

For issues:
1. Check logs: `tail -f ~/.aaroneous/hive.log`
2. Run health check: `aaroneous status health`
3. Review memory: `aaroneous memory stats`
4. Check specialist XP: `aaroneous specialist list --full`

---

**Aaroneous v2.0 - Autonomous Intelligence in Action**

