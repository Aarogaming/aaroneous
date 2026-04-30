# Aaroneous v2.0 - Complete API Reference

## Table of Contents

1. [Core Runtime API](#core-runtime-api)
2. [Task Management API](#task-management-api)
3. [Autonomous Systems API](#autonomous-systems-api)
4. [Memory System API](#memory-system-api)
5. [Error Handling API](#error-handling-api)
6. [Collaboration API](#collaboration-api)
7. [Goal Autonomy API](#goal-autonomy-api)
8. [Observability API](#observability-api)

---

## Core Runtime API

### HiveRuntime

Main orchestrator for the autonomous hive.

#### Initialization

```rust
use aaroneous::hive_runtime::HiveRuntime;
use aaroneous::specialist::Specialist;

// Create runtime
let runtime = HiveRuntime::new("./hive.db").await?;

// Add specialists
let merlin = Specialist::new(
    "merlin",
    "Merlin",
    "Data expert",
    vec!["SQL", "Python", "Data Analysis"],
);
runtime.add_specialist(merlin).await?;

// Start event loop
runtime.start().await?;
```

#### Task Submission

```rust
use aaroneous::task::{Task, TaskPriority};

let task = Task {
    id: "task-001".to_string(),
    name: "Analyze Sales Data".to_string(),
    description: "Process Q4 sales records".to_string(),
    data_sample: Some("Sample 100 records...".to_string()),
    priority: TaskPriority::High,
    deadline_secs: Some(300),
    required_skills: vec!["SQL", "Data Analysis"],
    tags: vec!["analysis", "sales"],
};

let task_id = runtime.submit_task(task).await?;
println!("Task submitted: {}", task_id);
```

#### Status & Monitoring

```rust
// Get task status
if let Some(status) = runtime.get_task_status(&task_id).await? {
    println!("Status: {:?}", status);
    // Output: Status: Executing
}

// Get specialist status
let specialists = runtime.get_specialists().await?;
for spec in specialists {
    println!("{}: {} XP", spec.name, spec.xp);
}

// System health
let health = runtime.health_check().await?;
println!("System healthy: {}", health);

// Statistics
let stats = runtime.get_statistics().await?;
println!("Uptime: {} seconds", stats.uptime_seconds);
println!("Tasks: {}/{} completed", stats.completed_tasks, stats.total_tasks);
```

#### Lifecycle Management

```rust
// Graceful shutdown
runtime.shutdown().await?;

// Pause processing
runtime.pause().await?;

// Resume processing
runtime.resume().await?;

// Force cleanup
runtime.cleanup().await?;
```

---

## Task Management API

### Task Structure

```rust
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub data_sample: Option<String>,
    pub priority: TaskPriority,
    pub deadline_secs: Option<u32>,
    pub required_skills: Vec<String>,
    pub tags: Vec<String>,
}

pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}
```

### Task Submission

```rust
// Basic task
let task = Task {
    id: uuid::Uuid::new_v4().to_string(),
    name: "Process Logs".to_string(),
    description: "Parse and categorize logs".to_string(),
    data_sample: None,
    priority: TaskPriority::Normal,
    deadline_secs: Some(600),
    required_skills: vec!["Log Analysis", "Regex"],
    tags: vec!["logs", "parsing"],
};

runtime.submit_task(task).await?;
```

### Task Status

```rust
pub enum TaskCoordinationStatus {
    Submitted,
    Analyzing,
    AnalysisComplete,
    Matching,
    MatchingComplete,
    Planning,
    PlanningComplete,
    Executing,
    Completed,
    Failed,
}
```

### Task Analysis

```rust
use aaroneous::task_analysis::TaskAnalysisEngine;

let analysis_engine = TaskAnalysisEngine::new(llm_client);

let analysis = analysis_engine.analyze_task(&task).await?;

println!("Complexity: {:?}", analysis.estimated_complexity);
println!("XP Reward: {}", analysis.estimated_xp_reward);
println!("Recommended Approach: {}", analysis.recommended_approach);
println!("Potential Challenges: {:?}", analysis.potential_challenges);
```

### Result: TaskAnalysis

```rust
pub struct TaskAnalysis {
    pub task_id: String,
    pub estimated_complexity: TaskComplexity,
    pub estimated_duration_minutes: u32,
    pub estimated_xp_reward: u32,
    pub recommended_approach: String,
    pub potential_challenges: Vec<String>,
    pub required_capabilities: Vec<String>,
}

pub enum TaskComplexity {
    Simple,
    Moderate,
    Complex,
    VeryComplex,
}
```

---

## Autonomous Systems API

### 1. Capability Matching Engine

```rust
use aaroneous::capability_matching_v2::CapabilityMatchingEngine;

let matching_engine = CapabilityMatchingEngine::new();

let matches = matching_engine
    .find_best_matches(&task, specialists, 3)
    .await?;

for match_result in matches {
    println!("{}: {:.2}% match", 
        match_result.specialist_id, 
        match_result.match_score * 100.0);
    
    if let Some(gap) = match_result.skill_gap {
        println!("  Gap: {}", gap);
    }
}
```

### Result: SpecialistCapabilityMatch

```rust
pub struct SpecialistCapabilityMatch {
    pub specialist_id: String,
    pub match_score: f64,           // 0.0 - 1.0
    pub skill_matches: Vec<SkillMatch>,
    pub experience_score: f64,
    pub availability_score: f64,
    pub learning_potential: f64,
    pub skill_gap: Option<String>,
    pub recommendation: String,
}

pub struct SkillMatch {
    pub skill: String,
    pub required_level: u32,
    pub specialist_level: u32,
    pub match_type: MatchType,
}

pub enum MatchType {
    Exact,          // Exact skill level match
    Partial,        // Skill exists, lower level
    Learning,       // Can learn quickly
    NotAvailable,   // Specialist lacks skill
}
```

### 2. Autonomous Planning Engine

```rust
use aaroneous::autonomous_planning::AutonomousPlanningEngine;

let planning_engine = AutonomousPlanningEngine::new(llm_client);

let plan = planning_engine
    .generate_plan(
        &task,
        &analysis,
        &specialist,
        &matching_result,
    )
    .await?;

println!("Plan ID: {}", plan.plan_id);
println!("Steps: {}", plan.steps.len());
println!("Duration: ~{} minutes", plan.estimated_duration_minutes);
println!("Success Probability: {:.1}%", plan.success_probability * 100.0);

for step in &plan.steps {
    println!("  Step {}: {}", step.sequence, step.action);
}

for contingency in &plan.contingencies {
    println!("  If {}: {}", contingency.trigger, contingency.action);
}
```

### Result: AutonomousPlan

```rust
pub struct AutonomousPlan {
    pub plan_id: String,
    pub task_id: String,
    pub primary_specialist: String,
    pub steps: Vec<ExecutionStep>,
    pub estimated_duration_minutes: u32,
    pub success_probability: f64,
    pub contingencies: Vec<Contingency>,
}

pub struct ExecutionStep {
    pub sequence: u32,
    pub action: String,
    pub expected_outcome: String,
    pub estimated_minutes: u32,
    pub validation_criteria: Vec<String>,
}

pub struct Contingency {
    pub trigger: String,
    pub action: String,
    pub fallback_approach: Option<String>,
}
```

### 3. Execution Tracking

```rust
let tracker = plan.create_tracker();

// Record step completion
tracker.record_step_completion(1, "Successfully loaded data").await?;

// Update progress
let progress = tracker.get_progress().await?;
println!("Progress: {}/{} steps", progress.completed_steps, progress.total_steps);

// Get current status
let status = tracker.get_status().await?;
match status {
    ExecutionStatus::InProgress => println!("Running..."),
    ExecutionStatus::Completed => println!("Done!"),
    ExecutionStatus::Failed => println!("Failed"),
}
```

---

## Memory System API

### SpecialistMemory

```rust
use aaroneous::specialist_memory::{SpecialistMemory, MemorySource};

// Create memory system
let memory = SpecialistMemory::new("./hive.db").await?;

// Record a lesson
let memory_entry = MemoryEntry {
    id: uuid::Uuid::new_v4().to_string(),
    specialist_id: "merlin".to_string(),
    memory_type: MemoryType::Lesson,
    content: "Parallel processing 10x faster than sequential".to_string(),
    source: MemorySource::Experience,
    confidence: 0.95,
    tags: vec!["performance", "optimization"],
    created_at: Utc::now(),
    last_accessed: Utc::now(),
};

memory.record_memory(memory_entry).await?;
```

### Memory Types

```rust
pub enum MemoryType {
    Lesson,      // Knowledge learned
    Strategy,    // Effective approach
    Decision,    // Choice made & rationale
    Reflection,  // Self-analysis
    Goal,        // Objective being pursued
}

pub enum MemorySource {
    Experience,      // Learned by doing
    LLMReasoning,   // Insight from LLM
    PeerLearning,   // Learned from others
    Configuration,  // Explicitly provided
    ErrorRecovery,  // Learned from failure
}
```

### Memory Operations

```rust
// Search memories
let lessons = memory
    .search_memories("async", Some("performance"))
    .await?;

println!("Found {} relevant lessons", lessons.len());

// Get active goals
let goals = memory.get_active_goals().await?;
for goal in goals {
    println!("{}: {}%", goal.name, goal.progress_percentage);
}

// Get best strategy
if let Some(strategy) = memory.get_best_strategy("database_optimization").await? {
    println!("Strategy: {}", strategy.content);
    println!("Success Rate: {:.1}%", strategy.success_rate * 100.0);
}

// Record decision
memory.record_decision(
    &task_id,
    "Used async processing",
    "Provides better throughput",
).await?;

// Get memory health
let health = memory.calculate_health().await?;
println!("Memory Health: {:.1}%", health * 100.0);

// Memory statistics
let stats = memory.get_memory_statistics().await?;
println!("Total memories: {}", stats.total_entries);
println!("Average confidence: {:.2}", stats.average_confidence);
```

### Result: MemoryEntry

```rust
pub struct MemoryEntry {
    pub id: String,
    pub specialist_id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub source: MemorySource,
    pub confidence: f64,        // 0.0 - 1.0
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}
```

---

## Error Handling API

### ErrorRecoveryEngine

```rust
use aaroneous::error_recovery::{ErrorRecoveryEngine, ErrorType, ExecutionError};

let recovery_engine = ErrorRecoveryEngine::new(memory.clone());

let error = ExecutionError {
    id: uuid::Uuid::new_v4().to_string(),
    task_id: task_id.clone(),
    error_type: ErrorType::TimeoutExceeded,
    message: "Processing took >300 seconds".to_string(),
    contributing_factors: vec![
        "Large dataset (10GB)".to_string(),
        "Sequential processing".to_string(),
    ],
    timestamp: Utc::now(),
};

let recovery = recovery_engine.analyze_and_recover(&error).await?;

println!("Root Cause: {}", recovery.root_cause);
println!("Recovery Strategy: {:?}", recovery.recovery_actions);

for action in &recovery.recovery_actions {
    println!("  - {}: {}", action.action_type, action.description);
}
```

### Error Types

```rust
pub enum ErrorType {
    TimeoutExceeded,           // Operation exceeded deadline
    ResourceExhaustion,        // Memory/CPU/disk full
    InvalidInput,              // Bad data format
    ExternalServiceFailed,     // API/DB unavailable
    SkillGapFound,             // Specialist lacks skill
    DataFormatMismatch,        // Input/output incompatibility
    ConcurrencyConflict,       // Race condition/deadlock
    UnexpectedFailure,         // Unknown error
}
```

### Recovery Result

```rust
pub struct RecoveryStrategy {
    pub error_id: String,
    pub root_cause: String,
    pub contributing_factors: Vec<String>,
    pub recovery_actions: Vec<RecoveryAction>,
    pub estimated_retry_delay_secs: u32,
}

pub struct RecoveryAction {
    pub action_type: String,
    pub description: String,
    pub priority: u8,
}
```

### Retry Logic

```rust
// Automatic exponential backoff
let max_attempts = 4;
for attempt in 0..max_attempts {
    match execute_task().await {
        Ok(result) => return Ok(result),
        Err(e) => {
            let delay_secs = 2_u64.pow(attempt as u32);
            println!("Attempt {} failed, retrying in {} seconds", attempt + 1, delay_secs);
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
        }
    }
}
```

---

## Collaboration API

### SpecialistCollaborationEngine

```rust
use aaroneous::specialist_collaboration::{
    SpecialistCollaborationEngine, HelpRequest, Urgency, AssistanceType,
};

let collab_engine = SpecialistCollaborationEngine::new(memory.clone());

// Request help
let help_request = HelpRequest {
    request_id: uuid::Uuid::new_v4().to_string(),
    requester_id: "merlin".to_string(),
    task_id: task_id.clone(),
    skill_needed: "Rust Async Programming".to_string(),
    challenge_description: "Ownership rules preventing parallel iteration".to_string(),
    urgency: Urgency::High,
    timestamp: Utc::now(),
};

let request_id = collab_engine.request_help(help_request).await?;
println!("Help request sent: {}", request_id);

// Find best helper
let helpers = collab_engine
    .find_best_helpers(&task, "Rust Async Programming", 3)
    .await?;

for helper in helpers {
    println!("{}: {:.1}% match", helper.specialist_id, helper.score * 100.0);
}

// Respond to help request
let response = HelpResponse {
    response_id: uuid::Uuid::new_v4().to_string(),
    request_id: request_id.clone(),
    helper_id: "odin".to_string(),
    assistance_type: AssistanceType::Mentoring,
    guidance: "Use Arc<Mutex<>> for shared mutable state".to_string(),
    timestamp: Utc::now(),
};

collab_engine.respond_to_help(response).await?;
println!("Help response recorded");

// Get collaboration metrics
let metrics = collab_engine.get_collaboration_metrics("merlin").await?;
println!("Help requests sent: {}", metrics.help_requests_sent);
println!("Help responses received: {}", metrics.help_requests_received);
println!("Collaboration success rate: {:.1}%", metrics.collaboration_success_rate * 100.0);
```

### Help Request Structure

```rust
pub struct HelpRequest {
    pub request_id: String,
    pub requester_id: String,
    pub task_id: String,
    pub skill_needed: String,
    pub challenge_description: String,
    pub urgency: Urgency,
    pub timestamp: DateTime<Utc>,
}

pub enum Urgency {
    Low,        // Background, flexible
    Medium,     // Normal workflow
    High,       // Blocks progress
    Critical,   // Immediate action
}

pub enum AssistanceType {
    DirectHelp,        // Take over task
    Consultation,      // Advice & guidance
    Mentoring,         // Teaching the skill
    ResourceSharing,   // Provide tools/data
    Delegation,        // Full handoff
}
```

### Collaboration Metrics

```rust
pub struct CollaborationMetrics {
    pub help_requests_sent: u32,
    pub help_requests_received: u32,
    pub help_requests_accepted: u32,
    pub collaboration_success_rate: f64,
    pub peers: Vec<String>,
    pub taught_specialists: Vec<String>,
    pub learned_from_specialists: Vec<String>,
}
```

---

## Goal Autonomy API

### GoalDrivenAutonomyEngine

```rust
use aaroneous::goal_driven_autonomy::{
    GoalDrivenAutonomyEngine, AutonomousGoal, AutonomousGoalStatus, GoalCategory,
};

let goal_engine = GoalDrivenAutonomyEngine::new(memory.clone());

// Create a goal
let goal = AutonomousGoal {
    goal_id: uuid::Uuid::new_v4().to_string(),
    specialist_id: "merlin".to_string(),
    category: GoalCategory::SkillDevelopment,
    title: "Master Async Rust".to_string(),
    description: "Become expert in tokio ecosystem".to_string(),
    target_value: 100.0,
    current_progress: 0.0,
    status: AutonomousGoalStatus::Planning,
    milestones: vec![
        Milestone {
            id: "m1".to_string(),
            name: "Complete tokio tutorial".to_string(),
            target_value: 25.0,
            current_value: 0.0,
            completed: false,
        },
        Milestone {
            id: "m2".to_string(),
            name: "Build concurrent app".to_string(),
            target_value: 50.0,
            current_value: 0.0,
            completed: false,
        },
    ],
    created_at: Utc::now(),
    last_updated: Utc::now(),
};

goal_engine.create_goal(goal.clone()).await?;

// Activate goal
goal_engine.activate_goal(&goal.goal_id).await?;

// Update progress
goal_engine.update_progress(&goal.goal_id, 25.0).await?;

// Get goals by status
let active_goals = goal_engine.get_goals_by_status(AutonomousGoalStatus::Active).await?;
println!("Active goals: {}", active_goals.len());

for goal in active_goals {
    println!("{}: {}%", goal.title, (goal.current_progress as u32));
    for milestone in &goal.milestones {
        if !milestone.completed {
            println!("  ✓ {}: {:.0}%", 
                milestone.name, 
                (milestone.current_value / milestone.target_value * 100.0));
        }
    }
}

// Complete milestone
goal_engine.complete_milestone(&goal.goal_id, "m1").await?;

// Calculate autonomy index
let autonomy = goal_engine.calculate_autonomy_index("merlin").await?;
println!("Autonomy Index: {:.1}%", autonomy * 100.0);
```

### Goal Status Machine

```rust
pub enum AutonomousGoalStatus {
    Planning,        // Design phase
    Active,          // In progress (user activated)
    InProgress,      // Making progress (20%+)
    OnTrack,         // High progress (80%+)
    AtRisk,          // Low progress (<20%)
    Completed,       // 100% done
    Failed,          // Abandoned
    Cancelled,       // User cancelled
    Paused,          // On hold
}
```

### Goal Structure

```rust
pub struct AutonomousGoal {
    pub goal_id: String,
    pub specialist_id: String,
    pub category: GoalCategory,
    pub title: String,
    pub description: String,
    pub target_value: f64,
    pub current_progress: f64,
    pub status: AutonomousGoalStatus,
    pub milestones: Vec<Milestone>,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

pub enum GoalCategory {
    SkillDevelopment,
    XPThreshold,
    Collaboration,
    Specialization,
    MentorshipGiving,
    MentorshipReceiving,
    TaskCompletion,
    Innovation,
}

pub struct Milestone {
    pub id: String,
    pub name: String,
    pub target_value: f64,
    pub current_value: f64,
    pub completed: bool,
}
```

---

## Observability API

### Tracing & Logging

```rust
use tracing::{info, debug, warn, error};

// System automatically logs all major events
info!("Task submitted: {}", task_id);
debug!("Analyzing complexity...");
warn!("Specialist skill gap detected, requesting help");
error!("Task failed after 3 retries: {}", error_msg);
```

### Statistics & Metrics

```rust
pub struct HiveStatistics {
    pub uptime_seconds: u64,
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub failed_tasks: u32,
    pub success_rate: f64,
    pub avg_completion_time_secs: u32,
    pub total_specialists: u32,
    pub total_xp_distributed: u64,
    pub memory_entries: u32,
    pub active_goals: u32,
    pub collaboration_index: f64,
}

let stats = runtime.get_statistics().await?;
println!("Success Rate: {:.1}%", stats.success_rate * 100.0);
println!("Avg Time: {} seconds", stats.avg_completion_time_secs);
println!("Team XP: {}", stats.total_xp_distributed);
```

### Health Checks

```rust
// System-wide health
let healthy = runtime.health_check().await?;

// Specialist health
for specialist in runtime.get_specialists().await? {
    let is_healthy = specialist.available && specialist.xp > 0;
    let status = if is_healthy { "✓" } else { "✗" };
    println!("{} {} - {:.0}% XP, {} skills", 
        status, specialist.name, 
        specialist.xp as f64 / 10000.0 * 100.0,
        specialist.skills.len());
}

// Memory health
let memory_health = memory.calculate_health().await?;
println!("Memory Health: {:.1}%", memory_health * 100.0);
```

---

## Complete Usage Example

```rust
use aaroneous::hive_runtime::HiveRuntime;
use aaroneous::task::{Task, TaskPriority};
use aaroneous::specialist::Specialist;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize
    let runtime = HiveRuntime::new("./hive.db").await?;
    
    // Add specialists
    let merlin = Specialist::new(
        "merlin",
        "Merlin",
        "Data master",
        vec!["SQL", "Python", "Statistics"],
    );
    runtime.add_specialist(merlin).await?;
    
    let odin = Specialist::new(
        "odin",
        "Odin",
        "Systems expert",
        vec!["Rust", "Systems Design", "Performance"],
    );
    runtime.add_specialist(odin).await?;
    
    // Start hive
    runtime.start().await?;
    
    // Submit task
    let task = Task {
        id: "analysis-001".to_string(),
        name: "Analyze User Metrics".to_string(),
        description: "Process and categorize user behavior".to_string(),
        data_sample: Some("1000 user records".to_string()),
        priority: TaskPriority::High,
        deadline_secs: Some(600),
        required_skills: vec!["SQL", "Data Analysis"],
        tags: vec!["analytics", "users"],
    };
    
    let task_id = runtime.submit_task(task).await?;
    
    // Monitor progress
    loop {
        if let Some(status) = runtime.get_task_status(&task_id).await? {
            println!("Task Status: {:?}", status);
            
            if status == TaskCoordinationStatus::Completed {
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    // Get final stats
    let stats = runtime.get_statistics().await?;
    println!("Tasks Completed: {}", stats.completed_tasks);
    println!("Success Rate: {:.1}%", stats.success_rate * 100.0);
    
    // Shutdown
    runtime.shutdown().await?;
    
    Ok(())
}
```

---

## Error Handling

All APIs return `Result<T, anyhow::Error>` for consistent error handling:

```rust
match runtime.submit_task(task).await {
    Ok(task_id) => println!("Task submitted: {}", task_id),
    Err(e) => eprintln!("Failed to submit task: {}", e),
}
```

---

## Thread Safety

All APIs are fully async and thread-safe:

- `HiveRuntime`: `Arc<Mutex<>>` internally
- `SpecialistMemory`: Thread-safe database access
- All engines: Concurrent-safe design

---

## Performance Notes

- **Task Submission**: <1ms
- **Capability Matching**: 10-50ms (depends on specialist count)
- **LLM Analysis**: 1-5s (depends on model size)
- **Plan Generation**: 2-10s (with contingencies)
- **Error Recovery**: 500ms (strategy selection)
- **Memory Operations**: 10-50ms (SQL queries)

---

## Version 2.0 Features Covered

✅ Full async/await support  
✅ Local LLM integration (GGUF)  
✅ Specialist memory with persistence  
✅ Task analysis & capability matching  
✅ Autonomous planning with contingencies  
✅ Error recovery with retry logic  
✅ Specialist collaboration & mentoring  
✅ Goal-driven autonomy with milestones  
✅ Comprehensive observability  
✅ Thread-safe, production-ready APIs

---

**Aaroneous v2.0 - Complete API Reference**
