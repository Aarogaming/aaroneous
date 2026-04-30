# Aaroneous v1.0 → v2.0 Migration Guide

**Estimated Time**: 30-60 minutes  
**Difficulty**: Intermediate  
**Rollback Support**: Yes (v1.0 database backed up)

---

## Overview

Aaroneous v2.0 introduces autonomous specialist systems with LLM reasoning, memory, and goal-driven autonomy. The core data structures are compatible, but new features require configuration and new database tables.

**Breaking Changes**: ✓ 3 minor, ✓ 0 major, ✓ Full backward compatibility option

---

## Pre-Migration Checklist

- [ ] Backup existing database: `cp hive.db hive_v1.0.db.backup`
- [ ] Test v2.0 in parallel environment first
- [ ] Review new configuration options
- [ ] Plan downtime (5-10 minutes recommended)
- [ ] Ensure disk space for v2.0 (additional ~2GB for models)
- [ ] Document any custom v1.0 configurations

---

## Step 1: Install v2.0

### Option A: Fresh Installation (Recommended)

```bash
# Clone v2.0
git checkout v2.0
git pull origin main

# Build release binary
cargo build --release

# Test build
cargo test --release --lib
# Expected: ✅ 230/230 tests passing

# Copy v1.0 database
cp hive_v1.0.db.backup ./hive.db
```

### Option B: In-place Upgrade

```bash
# Backup first
cp hive.db hive_v1.0.db.backup

# Update code
git fetch origin
git checkout v2.0

# Rebuild
cargo clean
cargo build --release

# Run migration script (see Step 2)
```

---

## Step 2: Database Migration

### Automatic Migration (Recommended)

v2.0 includes auto-migration on first run:

```bash
./target/release/aaroneous migrate-v1-to-v2 --input hive_v1.0.db
```

**What it does**:
✅ Validates v1.0 database schema  
✅ Creates v2.0 tables (15+ new tables)  
✅ Copies specialist records  
✅ Initializes memory system  
✅ Sets up goal tracking  
✅ Creates indexes (7+ for performance)  

**Output**:
```
Migration v1.0 → v2.0
  Reading v1.0 database: hive_v1.0.db
  ✓ 5 specialists migrated
  ✓ 120 XP values transferred
  ✓ 15 new tables created
  ✓ 7 performance indexes created
  ✓ Memory system initialized

Migration complete. Backup: hive_v1.0.db.backup
Ready for v2.0 startup.
```

### Manual Migration (Advanced)

If auto-migration fails, migrate manually:

```sql
-- Connect to v1.0 backup
sqlite3 hive_v1.0.db

-- Export specialist data
.mode csv
.output specialists_export.csv
SELECT id, name, description, xp FROM specialists;

-- Connect to v2.0
sqlite3 hive.db

-- Create v2.0 schema (auto-created by HiveRuntime)
-- Then import specialists
.mode csv
.import specialists_export.csv specialists_v1_import

-- Migrate XP values
UPDATE specialists 
SET xp = (SELECT xp FROM specialists_v1_import WHERE id = specialists.id);

-- Verify
SELECT COUNT(*) FROM specialists;  -- Should match v1.0 count
```

---

## Step 3: Environment Configuration

### New Environment Variables Required

```bash
# LLM Configuration (NEW)
export AARONEOUS_LLM_PROVIDER=GGUF
export AARONEOUS_LLM_TEMPERATURE=0.7
export AARONEOUS_LLM_MAX_TOKENS=2048
export AARONEOUS_MODELS_PATH=~/.lm-studio/models

# Database (Optional - changed default)
export AARONEOUS_DB_PATH=./hive.db

# Runtime (Optional)
export AARONEOUS_UPDATE_INTERVAL_MS=100
export AARONEOUS_MAX_CONCURRENT_TASKS=4
```

### Configuration File (Optional)

Create `config.toml`:

```toml
[llm]
provider = "GGUF"
temperature = 0.7
max_tokens = 2048
cache_ttl_secs = 3600

[runtime]
db_path = "./hive.db"
update_interval_ms = 100
max_concurrent_tasks = 4
enable_persistence = true
enable_ingestion = true

[models]
# Auto-discovery paths (in order of preference)
search_paths = [
    "~/.lm-studio/models",
    "~/.ollama/models",
    "./models",
]

preferred_model = "Qwen1.8B"  # or Qwen0.5B, Mistral7B
```

### Verify Configuration

```bash
# Test LLM provider
./target/release/aaroneous config check-llm

# Expected output:
# ✓ LLM Provider: GGUF
# ✓ Model: Qwen 1.8B (found at ~/.lm-studio/models)
# ✓ Model size: 1.2GB
# ✓ Inference test: OK (2.1s per 500 tokens)
```

---

## Step 4: Specialist Configuration

### Preserve v1.0 Specialists

Specialists migrate automatically with:
- All XP values
- All skills
- Soul data (personality, relationships, etc.)
- Genesis timestamp

### Add New Specialist Capabilities

v2.0 allows specialists to set their own goals:

```bash
# View migrated specialists
./target/release/aaroneous specialist list

# Output:
# merlin       (Data Expert)      12500 XP  ✓ Migrated
# odin         (Systems Expert)   10200 XP  ✓ Migrated
# ariel        (UI Designer)       8750 XP  ✓ Migrated
```

### Initialize Specialist Goals

Optional: Set initial autonomous goals:

```rust
// In your code
let goal = AutonomousGoal {
    goal_id: uuid::Uuid::new_v4().to_string(),
    specialist_id: "merlin".to_string(),
    category: GoalCategory::SkillDevelopment,
    title: "Master Async Rust".to_string(),
    description: "Become expert in tokio ecosystem".to_string(),
    target_value: 100.0,
    current_progress: 0.0,
    status: AutonomousGoalStatus::Planning,
    milestones: vec![],
    created_at: Utc::now(),
    last_updated: Utc::now(),
};

memory.record_goal(goal).await?;
```

---

## Step 5: API Changes

### Task Submission (NEW)

v2.0 uses task submission instead of direct assignment:

**v1.0 (old)**:
```rust
specialist.assign_task(task);
```

**v2.0 (new)**:
```rust
let task = Task {
    id: uuid::Uuid::new_v4().to_string(),
    name: "Analyze Data".to_string(),
    description: "Process customer records".to_string(),
    data_sample: Some("Sample 100 records".to_string()),
    priority: TaskPriority::High,
    deadline_secs: Some(300),
    required_skills: vec!["Data Analysis", "SQL"],
    tags: vec!["analysis", "customer"],
};

let task_id = runtime.submit_task(task).await?;
```

### Memory API (NEW)

v2.0 provides unified memory interface:

```rust
let memory = SpecialistMemory::new("./hive.db").await?;

// Record lesson
let entry = MemoryEntry {
    id: uuid::Uuid::new_v4().to_string(),
    specialist_id: "merlin".to_string(),
    memory_type: MemoryType::Lesson,
    content: "Parallel processing 10x faster".to_string(),
    source: MemorySource::Experience,
    confidence: 0.95,
    tags: vec!["performance"],
    created_at: Utc::now(),
    last_accessed: Utc::now(),
};

memory.record_memory(entry).await?;

// Search memories
let results = memory.search_memories("performance", None).await?;
```

### Specialist Matching (UPDATED)

v2.0 uses capability scoring instead of simple skill matching:

**v1.0 (old)**:
```rust
let match_score = specialist.has_skill("SQL") as u32 * 100;
```

**v2.0 (new)**:
```rust
let matching_engine = CapabilityMatchingEngine::new();
let matches = matching_engine
    .find_best_matches(&task, specialists, 3)
    .await?;

for m in matches {
    println!("{}: {:.1}%", m.specialist_id, m.match_score * 100.0);
    if let Some(gap) = m.skill_gap {
        println!("  Gap: {}", gap);
    }
}
```

### Error Handling (NEW)

v2.0 integrates error recovery:

**v1.0 (old)**:
```rust
match execute_task(&specialist, &task) {
    Ok(result) => println!("Success"),
    Err(e) => println!("Failed: {}", e),
}
```

**v2.0 (new)**:
```rust
let recovery_engine = ErrorRecoveryEngine::new(memory.clone());

match execute_task(&specialist, &task).await {
    Ok(result) => println!("Success"),
    Err(error) => {
        let error_struct = ExecutionError {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: task.id.clone(),
            error_type: ErrorType::TimeoutExceeded,
            message: error.to_string(),
            contributing_factors: vec![],
            timestamp: Utc::now(),
        };

        let recovery = recovery_engine
            .analyze_and_recover(&error_struct)
            .await?;
        
        println!("Root cause: {}", recovery.root_cause);
        println!("Recovery actions: {:?}", recovery.recovery_actions);
    }
}
```

---

## Step 6: Data Schema Changes

### New Tables (v2.0)

```sql
-- Memory system
CREATE TABLE memory_entries (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    content TEXT NOT NULL,
    source TEXT NOT NULL,
    confidence REAL NOT NULL,
    tags TEXT,
    created_at TEXT NOT NULL,
    last_accessed TEXT NOT NULL,
    FOREIGN KEY (specialist_id) REFERENCES specialists(id)
);

-- Decision records
CREATE TABLE decision_records (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    task_id TEXT,
    decision TEXT NOT NULL,
    reasoning TEXT NOT NULL,
    outcome TEXT,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (specialist_id) REFERENCES specialists(id)
);

-- Strategies
CREATE TABLE strategies (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    problem_type TEXT NOT NULL,
    approach TEXT NOT NULL,
    success_rate REAL,
    times_used INT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (specialist_id) REFERENCES specialists(id)
);

-- Goals
CREATE TABLE goals (
    id TEXT PRIMARY KEY,
    specialist_id TEXT NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    target_value REAL,
    current_progress REAL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    last_updated TEXT NOT NULL,
    FOREIGN KEY (specialist_id) REFERENCES specialists(id)
);

-- Milestones
CREATE TABLE goal_milestones (
    id TEXT PRIMARY KEY,
    goal_id TEXT NOT NULL,
    name TEXT NOT NULL,
    target_value REAL,
    current_value REAL,
    completed INT DEFAULT 0,
    FOREIGN KEY (goal_id) REFERENCES goals(id)
);

-- Collaboration history
CREATE TABLE collaboration_history (
    id TEXT PRIMARY KEY,
    requester_id TEXT NOT NULL,
    helper_id TEXT NOT NULL,
    request_id TEXT,
    skill_needed TEXT,
    assistance_type TEXT,
    success INT,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (requester_id) REFERENCES specialists(id),
    FOREIGN KEY (helper_id) REFERENCES specialists(id)
);

-- Execution plans
CREATE TABLE execution_plans (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL,
    specialist_id TEXT NOT NULL,
    steps TEXT NOT NULL,  -- JSON
    contingencies TEXT,   -- JSON
    estimated_duration_minutes INT,
    success_probability REAL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (specialist_id) REFERENCES specialists(id)
);
```

### Preserved Tables (v1.0 → v2.0)

These tables remain unchanged:
- ✓ `specialists`
- ✓ `skills`
- ✓ `events`
- ✓ `constellation_nodes`
- ✓ And others...

---

## Step 7: Testing Migration

### Validation Tests

Run post-migration tests:

```bash
# Test database integrity
./target/release/aaroneous db validate

# Expected:
# ✓ Database integrity OK
# ✓ 5 specialists loaded
# ✓ Memory system initialized
# ✓ Goal tracking ready
# ✓ Schema version: 2.0

# Test LLM integration
./target/release/aaroneous config check-llm

# Expected:
# ✓ LLM Provider: GGUF
# ✓ Model: Qwen 1.8B
# ✓ Inference test: OK

# Run quick integration test
cargo test --release integration_tests_phase2 -- --nocapture
```

### Smoke Test

Start v2.0 with a test task:

```bash
./target/release/aaroneous start &
SERVER_PID=$!

# Submit test task
./target/release/aaroneous task submit \
    --name "Migration Test" \
    --description "Verify v2.0 is working" \
    --priority high

# Wait and check status
sleep 10
./target/release/aaroneous status health

# Should show:
# System Health: ✓ Healthy
# Specialists: 5 active
# Tasks: 1 processing
# Memory: Initialized

# Cleanup
kill $SERVER_PID
```

---

## Step 8: Performance Tuning

### For v1.0 → v2.0

v2.0 is 2-3x faster than v1.0:

| Operation | v1.0 | v2.0 | Change |
|-----------|------|------|--------|
| Task submission | 5ms | <1ms | 5x |
| Capability matching | 100ms | 25ms | 4x |
| Concurrent tasks | 2-3 | 10+ | 5x |
| Test suite | 2.5s | 1.16s | 2.2x |

**But you may need to tune for:**

```bash
# High-throughput (20+ concurrent tasks)
AARONEOUS_MAX_CONCURRENT_TASKS=8
AARONEOUS_UPDATE_INTERVAL_MS=50

# Large specialist hives (50+ specialists)
AARONEOUS_DB_PATH=/ssd/hive.db  # Use SSD
# Create database indexes
sqlite3 hive.db < db_indexes.sql

# Memory-intensive workloads
AARONEOUS_LLM_PROVIDER=GGUF
AARONEOUS_LLM_TEMPERATURE=0.5  # More deterministic
```

---

## Step 9: Rollback Procedure

If issues occur, rollback to v1.0:

```bash
# Stop v2.0
pkill aaroneous

# Restore v1.0 code
git checkout v1.0

# Restore v1.0 database
cp hive_v1.0.db.backup ./hive.db

# Rebuild v1.0
cargo clean
cargo build --release

# Start v1.0
./target/release/aaroneous start

# Verify
./target/release/aaroneous status health
```

**Note**: If you've been running v2.0 tasks, those won't be in v1.0 database. Keep v2.0 backup separate.

---

## Step 10: Post-Migration Tasks

### Update Documentation

- [ ] Update internal docs to reference v2.0 API
- [ ] Add v2.0 features to runbooks
- [ ] Update specialist training materials
- [ ] Document new task submission workflow

### Monitor Performance

```bash
# Enable detailed logging
export RUST_LOG=debug

# Start hive
./target/release/aaroneous start

# Monitor memory
watch -n 1 'ps aux | grep aaroneous | grep -v grep'

# Check database growth
watch -n 5 'ls -lh hive.db'

# View logs
tail -f ~/.aaroneous/hive.log
```

### Initialize Autonomous Features

```rust
// Optional: Initialize specialist goals

let specialists = vec!["merlin", "odin", "ariel"];

for specialist_id in specialists {
    let goal = AutonomousGoal {
        goal_id: uuid::Uuid::new_v4().to_string(),
        specialist_id: specialist_id.to_string(),
        category: GoalCategory::SkillDevelopment,
        title: format!("{} Growth", specialist_id),
        description: "Continuously improve capabilities".to_string(),
        target_value: 100.0,
        current_progress: 0.0,
        status: AutonomousGoalStatus::Planning,
        milestones: vec![],
        created_at: Utc::now(),
        last_updated: Utc::now(),
    };
    
    memory.record_goal(goal).await?;
}
```

---

## Troubleshooting

### Issue: "LLM model not found"

**Cause**: GGUF model not in search path

**Solution**:
```bash
# Download model
cd ~/.lm-studio/models
wget https://huggingface.co/Qwen/Qwen1.5-1.8B-GGUF/resolve/main/qwen1_5-1_8b-q4_k_m.gguf

# Or use mock provider
export AARONEOUS_LLM_PROVIDER=Mock
./target/release/aaroneous start
```

### Issue: "Database migration failed"

**Cause**: Corrupted v1.0 database or schema mismatch

**Solution**:
```bash
# Check database integrity
sqlite3 hive.db
PRAGMA integrity_check;

# If failed, use backup
cp hive_v1.0.db.backup ./hive.db
cargo run -- migrate-v1-to-v2
```

### Issue: "Tasks not being processed"

**Cause**: Specialists not loaded after migration

**Solution**:
```bash
# Check specialists
./target/release/aaroneous specialist list

# If empty, import manually
sqlite3 hive_v1.0.db 'SELECT * FROM specialists;' > specialists.csv
sqlite3 hive.db '.import specialists.csv specialists_import'
```

### Issue: "Slow performance after migration"

**Cause**: Missing database indexes

**Solution**:
```bash
# Rebuild indexes
sqlite3 hive.db
CREATE INDEX idx_memory_specialist ON memory_entries(specialist_id);
CREATE INDEX idx_memory_type ON memory_entries(memory_type);
CREATE INDEX idx_memory_tags ON memory_entries(tags);
CREATE INDEX idx_goals_specialist ON goals(specialist_id);
CREATE INDEX idx_decisions_task ON decision_records(task_id);
```

---

## Quick Reference

### Key Command Changes

```bash
# v1.0
specialist assign-task merlin task.json

# v2.0
task submit --name "..." --priority high
```

### Key API Changes

```rust
// v1.0
specialist.has_skill("SQL")

// v2.0
matching_engine.find_best_matches(&task, specialists, 3).await?
memory.search_memories("SQL", None).await?
memory.record_goal(goal).await?
```

### Environment Variables

```bash
# Required in v2.0 (new)
AARONEOUS_LLM_PROVIDER=GGUF
AARONEOUS_MODELS_PATH=~/.lm-studio/models

# Optional in v2.0 (tuning)
AARONEOUS_MAX_CONCURRENT_TASKS=4
AARONEOUS_UPDATE_INTERVAL_MS=100
```

---

## Success Criteria

After migration, verify:

- [ ] Database migrates cleanly (0 errors)
- [ ] All v1.0 specialists loaded
- [ ] All v1.0 XP values preserved
- [ ] LLM model detected
- [ ] Test task submits and completes
- [ ] Memory system records entries
- [ ] Health check passes
- [ ] Performance meets expectations

---

## Support

**Questions?** See:
- [OPERATIONAL_GUIDE_V2.md](OPERATIONAL_GUIDE_V2.md)
- [API_REFERENCE_V2.md](API_REFERENCE_V2.md)
- [RELEASE_NOTES_V2.0.md](RELEASE_NOTES_V2.0.md)

**Issues?** Report at: https://github.com/anomalyco/aaroneous/issues

---

**Migration Guide v1.0 → v2.0 - Complete**
