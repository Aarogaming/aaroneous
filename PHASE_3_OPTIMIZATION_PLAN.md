# Aaroneous Phase 3 - Optimization Plan

**Focus**: Make v2.0 faster, more efficient, more capable  
**Duration**: 6 weeks  
**Target Improvements**: 3-10x performance gains in LLM analysis, memory operations, and query performance  
**Goal**: v2.1 with production-grade performance optimization

---

## Overview

Phase 3 builds on v2.0's solid foundation to optimize performance across three critical areas:

1. **LLM Inference** - Batch requests for 3x faster analysis
2. **Memory System** - Compression and archival for scalability
3. **Query Performance** - Caching layer for sub-10ms responses

These optimizations will enable:
- 100+ concurrent tasks (vs 10+ in v2.0)
- Sub-second memory searches
- 3-5x faster task analysis
- Reduced storage footprint (50% compression)

---

## Phase 3 Week 1: Batch LLM Request System

### Problem
Current system analyzes one task at a time. Each task waits 2-5 seconds for LLM response.

**Current Flow**:
```
Task 1 submit → LLM analyze (2-5s) → Plan → Execute
Task 2 submit → Wait for Task 1 → LLM analyze (2-5s) → Plan → Execute
Task 3 submit → Wait for Tasks 1-2 → LLM analyze (2-5s) → Plan → Execute

Total: 3 tasks × 5s analysis = 15 seconds
```

### Solution: Batch Request System

**New Architecture**:
```
Task 1 submit ┐
Task 2 submit ├→ Batch Queue → Single LLM call (5-7s) → 3 responses
Task 3 submit ┘

Total: 3 tasks in ~7 seconds (2.1x faster)
```

### Implementation

#### 1. Batch Queue Manager

```rust
pub struct LLMBatchQueue {
    pending_requests: Arc<Mutex<Vec<PendingRequest>>>,
    batch_size: usize,
    batch_timeout_ms: u64,
    sender: mpsc::Sender<Batch>,
}

pub struct PendingRequest {
    task_id: String,
    context: TaskContext,
    response_tx: tokio::sync::oneshot::Sender<LLMResponse>,
    submitted_at: Instant,
}

pub struct Batch {
    requests: Vec<PendingRequest>,
    created_at: Instant,
}

impl LLMBatchQueue {
    pub fn new(batch_size: usize, timeout_ms: u64) -> Self {
        // Initialize queue
    }
    
    pub async fn submit_request(&self, request: PendingRequest) -> Result<LLMResponse> {
        // Add to queue
        // If batch full or timeout reached, send to processor
        // Wait for response
    }
    
    async fn process_batch(&self, batch: Batch) -> Result<Vec<LLMResponse>> {
        // Group all requests into single prompt
        // Send to LLM once
        // Distribute responses back to waiters
    }
}
```

#### 2. Batch Prompt Construction

```rust
pub fn construct_batch_prompt(requests: &[PendingRequest]) -> String {
    let mut prompt = String::new();
    prompt.push_str("Analyze the following tasks:\n\n");
    
    for (i, req) in requests.iter().enumerate() {
        prompt.push_str(&format!("Task {}:\n", i + 1));
        prompt.push_str(&format!("Name: {}\n", req.context.task_name));
        prompt.push_str(&format!("Description: {}\n", req.context.description));
        prompt.push_str("---\n\n");
    }
    
    prompt.push_str("Provide analysis for each task in order.");
    prompt
}

pub fn parse_batch_response(response: &str, count: usize) -> Vec<TaskAnalysis> {
    // Parse multi-task response
    // Extract analysis for each task
    // Return vector of analyses
}
```

#### 3. Configuration

```toml
[llm]
batch_enabled = true
batch_size = 5              # Process 5 tasks per batch
batch_timeout_ms = 1000     # Or 1 second, whichever comes first
max_queue_size = 100        # Prevent unbounded memory growth
```

### Expected Improvements

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Single task analysis | 2-5s | 2-5s | (no change) |
| 3 sequential tasks | 15s | 7s | 2.1x faster |
| 10 sequential tasks | 50s | 15s | 3.3x faster |
| 100 sequential tasks | 500s | 150s | 3.3x faster |
| Task analysis throughput | 0.2 tasks/sec | 0.67 tasks/sec | 3.3x |

---

## Phase 3 Week 2: Memory Compression & Archival

### Problem
- SQLite database grows unbounded
- 1M memory entries = 1GB+ storage
- Full table scans for old entries slow

**Current Storage**:
```
memory_entries: 100K entries = 100MB
decision_records: 50K records = 50MB
strategies: 10K strategies = 10MB
goals: 5K goals = 5MB
Total: 165MB for active operations
```

### Solution: Compression & Archival

#### 1. Memory Compression

**Before Compression**:
```rust
{
    id: "mem-123",
    specialist_id: "merlin",
    memory_type: "Lesson",
    content: "Parallel processing improves throughput by 10x compared to sequential processing for large datasets",
    source: "Experience",
    confidence: 0.95,
    tags: ["performance", "optimization", "parallel"],
    created_at: "2026-04-01T10:30:00Z",
    last_accessed: "2026-04-15T14:22:00Z",
}
```

**After Compression** (50% reduction):
```rust
{
    id: "mem-123",
    specialist_id: "m1",              // ID compressed
    memory_type: 0,                   // Enum as u8
    content: "parallel 10x > seq",    // Summarized content
    source: 1,                        // Enum as u8
    confidence: 95,                   // u8 (0-100 instead of f64)
    tags: "perf,opt,par",            // Packed string
    created_at: 1743641400,           // Unix timestamp (4 bytes)
    last_accessed: 1744074120,        // Unix timestamp (4 bytes)
}
```

**Compression Algorithm**:
```rust
pub struct MemoryCompressor;

impl MemoryCompressor {
    pub fn compress(entry: MemoryEntry) -> CompressedMemoryEntry {
        CompressedMemoryEntry {
            id: entry.id,
            specialist_id: Self::compress_id(&entry.specialist_id),
            memory_type: entry.memory_type as u8,
            content: Self::summarize_content(&entry.content),
            source: entry.source as u8,
            confidence: (entry.confidence * 100.0) as u8,
            tags: entry.tags.join(","),
            created_at: entry.created_at.timestamp() as u32,
            last_accessed: entry.last_accessed.timestamp() as u32,
        }
    }
    
    pub fn decompress(compressed: CompressedMemoryEntry) -> MemoryEntry {
        // Reverse transformation
    }
}
```

#### 2. Archival Strategy

**Hot/Warm/Cold Tiers**:
```
HOT (0-7 days):
  - All recent entries
  - Full resolution
  - Indexed for fast search
  - In-memory cache
  
WARM (7-30 days):
  - Summarized content
  - Compressed records
  - SQL indexes
  - Accessed periodically
  
COLD (30+ days):
  - Archived to archive_memories table
  - Compressed to 50% size
  - No indexes
  - Accessed rarely
  - Can be exported for backup
```

**Archive Management**:
```rust
pub struct MemoryArchiveManager {
    db: Arc<SqlitePool>,
    archive_threshold_days: u32,
}

impl MemoryArchiveManager {
    pub async fn archive_old_entries(&self) -> Result<ArchiveStats> {
        // Find entries older than threshold
        // Compress them
        // Move to archive table
        // Delete from main table
        
        // Returns: moved count, space freed
    }
    
    pub async fn restore_from_archive(&self, entry_id: &str) -> Result<MemoryEntry> {
        // Retrieve from archive
        // Decompress
        // Return as if current
    }
    
    pub async fn export_archive(&self, path: &Path) -> Result<()> {
        // Export archive to file for backup
    }
}
```

#### 3. Cleanup Policies

**Automatic Cleanup**:
```toml
[memory]
max_entries = 100000           # Never exceed
compression_threshold_days = 7 # Compress after 1 week
archival_threshold_days = 30   # Archive after 1 month
auto_cleanup_enabled = true
cleanup_interval_hours = 6     # Run every 6 hours

[strategies]
max_strategies = 10000
keep_top_n = 5000             # By success rate
cleanup_unused = true         # Remove never-used
unused_threshold_days = 90

[goals]
archive_completed = true       # Move finished goals to history
completed_retention_days = 365 # Keep 1 year of history
```

### Expected Improvements

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Memory entry size | 1KB | 500 bytes | 2x compression |
| 100K entries storage | 100MB | 50MB | 2x |
| Search hot entries | 10ms | 5ms | 2x faster |
| Storage for 1M entries | 1GB | 500MB | 2x |
| Cleanup time per entry | N/A | <1ms | Automated |

---

## Phase 3 Week 3: Query Result Caching Layer

### Problem
- Repeated searches take 10-50ms
- Same capability searches run multiple times
- Memory health checks run frequently

**Current Load**:
```
Top queries by frequency:
  - search_memories("async") - 100x/hour
  - get_specialists_by_skill("SQL") - 50x/hour
  - get_active_goals() - 50x/hour
  - memory_health() - 200x/hour (every event loop)
```

### Solution: Multi-Layer Caching

#### 1. Query Cache Architecture

```rust
pub struct QueryCache {
    memory_cache: Arc<DashMap<String, CachedResult>>,
    ttl_config: CacheTTLConfig,
    stats: Arc<CacheStats>,
}

pub struct CachedResult<T> {
    data: T,
    created_at: Instant,
    hit_count: u64,
    last_accessed: Instant,
}

pub struct CacheTTLConfig {
    memory_search: Duration,           // 1 minute
    capability_search: Duration,       // 2 minutes
    goal_search: Duration,             // 30 seconds
    health_check: Duration,            // 10 seconds
    statistics: Duration,              // 5 seconds
}

impl QueryCache {
    pub fn new(config: CacheTTLConfig) -> Self {
        // Initialize cache with config
    }
    
    pub async fn get_or_compute<T, F>(&self, key: &str, f: F) -> Result<T>
    where
        T: Clone,
        F: FnOnce() -> Result<T>,
    {
        // Check cache
        if let Some(cached) = self.memory_cache.get(key) {
            if !cached.is_expired() {
                self.stats.hit_count.fetch_add(1, Ordering::Relaxed);
                return Ok(cached.data.clone());
            }
        }
        
        // Cache miss, compute
        let result = f()?;
        self.stats.miss_count.fetch_add(1, Ordering::Relaxed);
        
        // Store in cache
        self.memory_cache.insert(
            key.to_string(),
            CachedResult {
                data: result.clone(),
                created_at: Instant::now(),
                hit_count: 1,
                last_accessed: Instant::now(),
            },
        );
        
        Ok(result)
    }
    
    pub fn invalidate(&self, pattern: &str) {
        // Invalidate by pattern (e.g., "memory:*", "skill:sql:*")
    }
}
```

#### 2. Selective Caching

**Cache Strategy by Query Type**:

```rust
pub enum CacheStrategy {
    // High-value: cache aggressively
    MemorySearch {
        ttl_ms: 60_000,      // 1 minute
        key_pattern: String,
    },
    
    // Medium-value: cache moderately
    CapabilitySearch {
        ttl_ms: 120_000,     // 2 minutes
        key_pattern: String,
    },
    
    // Low-value: short cache
    HealthCheck {
        ttl_ms: 10_000,      // 10 seconds
        key_pattern: String,
    },
    
    // Never cache: real-time
    NoCache,
}

pub struct CachedQuery {
    query_type: CacheStrategy,
    key: String,
    fetch_fn: Box<dyn Fn() -> Result<Vec<u8>>>,
}
```

#### 3. Cache Invalidation

**Automatic Invalidation Triggers**:
```rust
pub enum InvalidationTrigger {
    // When data changes
    MemoryEntryAdded(String),        // Invalidate "memory:*"
    StrategyUpdated(String),         // Invalidate "strategy:*"
    GoalStatusChanged(String),       // Invalidate "goal:*"
    SpecialistSkillChanged(String),  // Invalidate "skill:*"
    
    // Time-based
    TTLExpired(String),
    
    // Manual
    UserInitiated,
    Shutdown,
}

impl QueryCache {
    pub fn on_invalidation(&self, trigger: InvalidationTrigger) {
        match trigger {
            InvalidationTrigger::MemoryEntryAdded(_) => {
                self.invalidate("memory:*");
            }
            InvalidationTrigger::StrategyUpdated(_) => {
                self.invalidate("strategy:*");
            }
            // ...
        }
    }
}
```

### Expected Improvements

| Query | Before | After | Hit Rate | Improvement |
|-------|--------|-------|----------|-------------|
| search_memories() | 25ms | 1ms (cache hit) | 70% | 25x avg |
| get_specialists_by_skill() | 15ms | 0.5ms (cache hit) | 60% | 30x avg |
| get_active_goals() | 20ms | 0.1ms (cache hit) | 80% | 200x avg |
| memory_health() | 50ms | 1ms (cache hit) | 90% | 50x avg |

**Real-world impact**:
- Memory search: 10-50ms → 1-5ms (5-10x)
- Capability matching: 25ms → 5ms (5x)
- Goal tracking: 20ms → 2ms (10x)

---

## Phase 3 Week 4: Advanced Model Selection

### Problem
- Single model for all tasks
- Complex tasks slow (Qwen 1.8B)
- Simple tasks waste capacity (Mistral 7B)

**Current Selection**:
```
All tasks → Qwen 1.8B (default)
  Tradeoff: Medium speed, medium quality
```

### Solution: Per-Task Model Selection

#### 1. Model Selection Engine

```rust
pub struct ModelSelectionEngine {
    models: Vec<ModelProfile>,
    task_complexity_scorer: TaskComplexityScorer,
}

pub struct ModelProfile {
    name: String,
    variant: ModelVariant,
    size_mb: u32,
    latency_per_token_ms: f64,
    quality_score: f64,     // 0.0-1.0
    reasoning_depth: f64,   // 0.0-1.0
    cost_per_inference: f64, // Relative
}

pub enum ModelVariant {
    QwenSmall,      // 0.5B - fast, OK quality
    QwenMedium,     // 1.8B - balanced (recommended)
    QwenLarge,      // 7B - high quality, slower
    MistralMedium,  // 7B - best reasoning
}

impl ModelSelectionEngine {
    pub fn select_model(&self, task: &Task, context: &TaskContext) -> ModelVariant {
        // Evaluate complexity
        let complexity = self.task_complexity_scorer.score(task);
        
        // Evaluate urgency
        let urgency = task.deadline_secs.map(|d| d as f64 / 300.0).unwrap_or(0.5);
        
        // Evaluate reasoning requirement
        let reasoning_needed = self.estimate_reasoning_complexity(task);
        
        // Score each model
        let mut scores = Vec::new();
        for model in &self.models {
            let score = self.score_model(model, complexity, urgency, reasoning_needed);
            scores.push((model.variant.clone(), score));
        }
        
        // Return best match
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores[0].0.clone()
    }
    
    fn score_model(
        &self,
        model: &ModelProfile,
        complexity: f64,
        urgency: f64,
        reasoning: f64,
    ) -> f64 {
        let quality_match = 1.0 - (model.quality_score - complexity).abs();
        let latency_match = 1.0 - (model.latency_per_token_ms / 100.0).min(1.0) * urgency;
        let reasoning_match = 1.0 - (model.reasoning_depth - reasoning).abs();
        
        (quality_match * 0.5) + (latency_match * 0.3) + (reasoning_match * 0.2)
    }
}
```

#### 2. Task Complexity Scoring

```rust
pub struct TaskComplexityScorer;

impl TaskComplexityScorer {
    pub fn score(&self, task: &Task) -> f64 {
        let mut score = 0.0;
        
        // Skill requirement complexity
        score += task.required_skills.len() as f64 * 0.1;
        
        // Description length
        score += (task.description.len() as f64 / 1000.0).min(0.3);
        
        // Keywords indicating complex reasoning
        let complex_keywords = vec!["analyze", "design", "optimize", "architecture"];
        let keyword_count = complex_keywords.iter()
            .filter(|kw| task.description.to_lowercase().contains(kw))
            .count();
        score += keyword_count as f64 * 0.15;
        
        // Priority (urgent = simpler routing)
        if task.priority == TaskPriority::Critical {
            score *= 0.7; // Reduce complexity weight for speed
        }
        
        score.clamp(0.0, 1.0)
    }
}
```

#### 3. Runtime Selection

```toml
[model_selection]
enabled = true
strategy = "adaptive"  # or "complexity", "speed", "quality"

[[models]]
name = "Qwen0.5B"
variant = "QwenSmall"
latency_per_token_ms = 30
quality_score = 0.70
reasoning_depth = 0.40
preferred_complexity_range = [0.0, 0.3]

[[models]]
name = "Qwen1.8B"
variant = "QwenMedium"
latency_per_token_ms = 60
quality_score = 0.85
reasoning_depth = 0.70
preferred_complexity_range = [0.25, 0.75]

[[models]]
name = "Qwen7B"
variant = "QwenLarge"
latency_per_token_ms = 120
quality_score = 0.95
reasoning_depth = 0.90
preferred_complexity_range = [0.65, 1.0]
```

### Expected Improvements

| Task Type | Before | After | Improvement |
|-----------|--------|-------|-------------|
| Simple task (0.2 complexity) | 5s (1.8B) | 2s (0.5B) | 2.5x faster |
| Complex task (0.8 complexity) | 5s (1.8B) | 8s (7B) | 1.6x better quality |
| Urgent task (60s deadline) | 5s (1.8B) | 2s (0.5B) | 2.5x faster |
| Average task mix | 5s | 3.5s | 1.4x faster |

**Real-world impact**:
- Simple task analysis: 5s → 2s
- Complex task quality: 85% → 95%
- Throughput: 0.2/s → 0.28/s
- Cost per task: 100% → 85%

---

## Phase 3 Week 5: Benchmarking & Tuning

### Benchmark Suite

Create comprehensive performance benchmarks:

```rust
#[bench]
fn bench_memory_search_cached(b: &mut Bencher) {
    // Warm cache
    // Measure search latency
    // Expected: <1ms
}

#[bench]
fn bench_capability_matching_100_specialists(b: &mut Bencher) {
    // Score 100 specialists
    // Measure latency
    // Expected: <25ms
}

#[bench]
fn bench_batch_llm_analysis_10_tasks(b: &mut Bencher) {
    // Batch analyze 10 tasks
    // Measure throughput
    // Expected: 0.67 tasks/sec
}

#[bench]
fn bench_goal_status_machine_transitions(b: &mut Bencher) {
    // Transition goals through states
    // Measure latency
    // Expected: <1ms per transition
}

#[bench]
fn bench_error_recovery_strategy_generation(b: &mut Bencher) {
    // Generate recovery for 8 error types
    // Measure latency
    // Expected: <500ms
}
```

### Tuning Parameters

```toml
[performance]
# LLM batching
batch_enabled = true
batch_size = 5
batch_timeout_ms = 1000

# Memory management
memory_compression = true
archive_after_days = 30
max_entries = 100000

# Caching
cache_enabled = true
cache_size_mb = 100
ttl_memory_search_ms = 60000
ttl_health_check_ms = 10000

# Threading
async_batch_threads = 2
query_cache_threads = 1
memory_archival_threads = 1

# Buffer sizes
task_queue_size = 1000
memory_queue_size = 5000
goal_queue_size = 500
```

---

## Phase 3 Week 6: Optimization Documentation

### Performance Guide

Create comprehensive documentation:

1. **Optimization Architecture** - How batching, caching, compression work
2. **Configuration Tuning** - How to tune for different workloads
3. **Benchmark Results** - Detailed performance numbers
4. **Profiling Guide** - How to profile and identify bottlenecks
5. **Scaling Strategies** - How to scale to 100+ tasks

---

## Expected Phase 3 Results

### Performance Improvements

| Metric | v2.0 | v2.1 | Improvement |
|--------|------|------|-------------|
| **Task Analysis** | 2-5s | 0.7-2.5s | 2-3x |
| **Memory Search** | 25ms | 1-5ms | 5-25x |
| **Query Performance** | 15-50ms | 1-2ms | 10-50x |
| **Storage (1M entries)** | 1GB | 500MB | 2x |
| **Concurrent Tasks** | 10+ | 100+ | 10x |
| **Throughput** | 0.2 tasks/s | 0.67 tasks/s | 3.3x |

### Code Additions

- **Batch LLM System**: 400-500 lines
- **Memory Compression**: 300-400 lines
- **Query Caching**: 400-500 lines
- **Model Selection**: 300-400 lines
- **Benchmarks**: 200-300 lines
- **Tests**: 400-500 lines

**Total**: ~1,800-2,500 lines of optimization code

### Test Coverage

- 20+ batch system tests
- 15+ compression/archival tests
- 20+ caching tests
- 15+ model selection tests
- 15+ benchmark tests
- 10+ integration tests

**Total**: ~95 new tests (v2.1 target: 325/325 passing)

---

## Success Criteria

✅ Batch LLM reduces task analysis time by 2-3x  
✅ Memory compression reduces storage by 50%  
✅ Query caching reduces latency by 10-50x  
✅ Model selection improves task success rate by 5-10%  
✅ All 325+ tests passing (v2.1)  
✅ Benchmarks show 3-10x improvements  
✅ Zero regressions in v2.0 functionality  
✅ Complete optimization documentation  

---

## Rollout Strategy

### Gradual Optimization

**Week 1**: Batch LLM (optional, can disable)  
**Week 2**: Memory archival (automatic, non-breaking)  
**Week 3**: Query caching (transparent, no API changes)  
**Week 4**: Model selection (opt-in, backward compatible)  
**Week 5**: Tuning & benchmarking (measurement only)  
**Week 6**: Documentation & release (v2.1)

### Backward Compatibility

✅ All optimizations backward compatible  
✅ Can disable any feature via config  
✅ No API changes  
✅ No database schema changes  
✅ Easy rollback to v2.0  

---

## Summary

**Phase 3 Optimization** will deliver:

1. 🚀 **3-10x faster** LLM analysis (batching)
2. 💾 **50% smaller** memory footprint (compression)
3. ⚡ **10-50x faster** query performance (caching)
4. 🎯 **5-10% better** success rates (smart model selection)
5. 📊 **100x more tasks** supported (100+ concurrent)
6. 📈 **Full visibility** into performance (benchmarks)

Estimated completion: **6 weeks**  
Confidence level: **High** (all techniques proven in industry)  
Risk level: **Low** (backward compatible, can disable features)  

---

**Ready to build v2.1? Start Phase 3 Week 1! 🚀**
