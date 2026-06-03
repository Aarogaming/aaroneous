# AARONEOUS INTEGRATION & DEFRAGMENTATION PLAN

**Objective**: Transform 37% integrated system → 100% cohesive production system  
**Timeline**: 3-4 weeks (120-140 hours)  
**Success Metric**: All feedback loops working + all computed data used + 90%+ integration tests passing  

---

## EXECUTIVE SUMMARY

### Current State
- **101 modules** (fragmented)
- **70-75K LOC functional** (but disconnected)
- **20-25K LOC stubbed** (broken integration)
- **15 modules integrated** (core only)
- **61 modules unused/underused** (bloat)

### Target State
- **40 core modules** (focused)
- **90K LOC all functional** (fully integrated)
- **0 stubbed implementations** (complete)
- **40+ modules integrated** (coherent)
- **0 unused modules** (cleaned up)

### Three Phases
1. **Phase I: Fix Critical Breaks** (Week 1) - 20 hours
2. **Phase II: Complete Integration** (Weeks 2-3) - 60 hours
3. **Phase III: Clean Up & Optimize** (Week 4) - 40 hours

---

## PHASE I: FIX CRITICAL BREAKS (WEEK 1)

**Goal**: Make core feedback loops work  
**Effort**: 20 hours  
**Success Criteria**:
- ✓ Dopamine feeds back to learning
- ✓ Tokens consumed on execution
- ✓ Enzyme results extracted correctly
- ✓ 4 integration tests passing

---

### ISSUE #1: Extract Enzyme Results (2 hours)

**Problem**: WASM task outputs lost during extraction

**Current Code** (enzyme_runner.rs:78-134):
```rust
// Lines 122-128: Falls back to empty result
fn run_enzyme(&mut self, task_id: &str, _input: Vec<u8>) -> Result<Vec<u8>, String> {
    // ...
    Ok(vec![])  // ← Returns empty output
}
```

**Why It's Broken**:
- Line 86: Comment says "CRITICAL FIX: Extract actual results"
- Lines 119-128: Attempts extraction but fails silently
- Caller gets no task output

**Fix Implementation**:

**File**: `core/hypervisor/src/enzyme_runner.rs`

1. **Fix result extraction logic** (lines 120-130)
   ```rust
   // BEFORE:
   Ok(vec![])  // Empty output
   
   // AFTER:
   let mut results = Vec::new();
   
   // Get data from WASM linear memory
   if let Some(data_ptr) = self.get_exported_value("result_ptr") {
       if let Ok(extracted) = self.extract_wasm_memory(data_ptr) {
           results = extracted;
       }
   }
   
   Ok(results)  // Return actual output
   ```

2. **Add memory extraction helper** (new method)
   ```rust
   fn extract_wasm_memory(&self, ptr: u32) -> Result<Vec<u8>, String> {
       // Read from WASM linear memory at pointer
       // Deserialize JSON/binary result format
       // Return parsed output
   }
   ```

3. **Add test**:
   ```rust
   #[test]
   fn test_enzyme_returns_results() {
       let mut runner = EnzymeRunner::new();
       let result = runner.run_enzyme("task1", vec![1, 2, 3]);
       assert!(!result.unwrap().is_empty());  // Results extracted
   }
   ```

**Verification**:
- [ ] Enzyme outputs no longer empty
- [ ] Task results reach caller
- [ ] Test passes
- [ ] grep shows no fallback to empty results

**Time**: 2 hours

---

### ISSUE #2: Activate Token System (6 hours)

**Problem**: Tokens tracked but never consumed or checked

**Current Code**:
- biology.rs:25-50 - Tokens defined and tracked ✓
- autonomic_loop.rs:580+ - Never consults or consumes ✗

**Why It's Broken**:
- Tokens exist in SpecialistMetabolism
- Never checked before execution
- Never decremented on execution
- Never regenerated per cycle

**Fix Implementation**:

**File**: `core/hypervisor/src/autonomic_loop.rs`

1. **Before task execution** (around line 560):
   ```rust
   // BEFORE execution, check tokens
   if let Some(specialist) = self.get_specialist(&specialist_id) {
       if specialist.metabolism.current_tokens < MIN_TOKENS_TO_EXECUTE {
           // Log and skip this specialist
           continue;  // Don't execute
       }
   }
   ```

2. **After execution** (around line 600):
   ```rust
   // AFTER execution, consume tokens
   if let Some(specialist) = self.get_specialist_mut(&specialist_id) {
       let cost = calculate_execution_cost(&task);  // 1-10 tokens
       specialist.metabolism.current_tokens = 
           specialist.metabolism.current_tokens.saturating_sub(cost);
   }
   ```

3. **Per autonomic tick** (around line 400):
   ```rust
   // EACH TICK, regenerate tokens
   for specialist in self.specialists.iter_mut() {
       let regeneration_rate = if specialist.metabolism.thermal_state == Hot {
           0.5  // Half regeneration when hot
       } else {
           1.0  // Full regeneration normal
       };
       
       specialist.metabolism.current_tokens = 
           (specialist.metabolism.current_tokens + regeneration_rate)
               .min(specialist.metabolism.max_tokens);
   }
   ```

4. **Add test**:
   ```rust
   #[test]
   fn test_tokens_depleted_halts_execution() {
       let mut loop_obj = autonomic_loop();
       
       // Deplete tokens
       loop_obj.specialists[0].metabolism.current_tokens = 0.0;
       
       // Try to execute
       loop_obj.tick();
       
       // Specialist should not have executed
       assert_eq!(executed_count, 0);
   }
   ```

**Verification**:
- [ ] Tokens consumed per execution
- [ ] Specialists skip when tokens empty
- [ ] Tokens regenerate per tick
- [ ] Test passes
- [ ] Monitor shows token consumption

**Time**: 6 hours

---

### ISSUE #3: Wire Dopamine to Learning (8 hours)

**Problem**: Dopamine signal computed but never influences learning

**Current Code**:
- dopamine_system.rs:8-40 - Computes reward ✓
- autonomic_loop.rs:580 - Calls dopamine ✓
- unified_learning.rs - Never receives dopamine signal ✗

**Why It's Broken**:
- autonomic_loop calls `dopamine_system.process_event()`
- Result is ignored
- Learning system never notified of success/failure
- Same mistakes repeat

**Fix Implementation**:

**File**: `core/hypervisor/src/unified_learning.rs`

1. **Add dopamine learning method** (new):
   ```rust
   pub fn learn_from_dopamine(&mut self, signal: &DopamineSignal) {
       match signal.signal_type {
           SignalType::Success => {
               // Increase specialist ambition
               if let Some(specialist) = self.specialist_states.get_mut(&signal.specialist_id) {
                   specialist.ambition = 
                       (specialist.ambition + 0.15).min(1.0);
               }
               
               // Strengthen this task → specialist association
               self.routing_weights.entry(signal.specialist_id)
                   .or_insert(1.0) *= 1.1;
           }
           SignalType::Failure => {
               // Increase specialist strictness (be more selective)
               if let Some(specialist) = self.specialist_states.get_mut(&signal.specialist_id) {
                   specialist.strictness = 
                       (specialist.strictness + 0.15).min(1.0);
               }
               
               // Weaken this task → specialist association
               self.routing_weights.entry(signal.specialist_id)
                   .or_insert(1.0) *= 0.9;
           }
           SignalType::Overload => {
               // Mark specialist as under stress
               // Reduce routing to them
           }
       }
   }
   ```

**File**: `core/hypervisor/src/autonomic_loop.rs`

2. **After execution** (around line 610):
   ```rust
   // Execute task
   let outcome = self.execute_on_specialist(&task, &specialist_id)?;
   
   // Get dopamine signal
   let dopamine_signal = self.dopamine_system.process_event(&outcome);
   
   // Wire to learning ← THIS IS THE FIX
   self.unified_learning.learn_from_dopamine(&dopamine_signal);
   
   // Continue with other logic
   ```

3. **Add test**:
   ```rust
   #[test]
   fn test_dopamine_updates_specialist_ambition() {
       let mut learning = UnifiedLearning::new();
       let signal = DopamineSignal {
           specialist_id: "spec_a",
           signal_type: SignalType::Success,
           magnitude: 0.8,
       };
       
       let before = learning.specialist_states["spec_a"].ambition;
       learning.learn_from_dopamine(&signal);
       let after = learning.specialist_states["spec_a"].ambition;
       
       assert!(after > before);  // Ambition increased
   }
   ```

**Verification**:
- [ ] Dopamine signal reaches learning
- [ ] Success increases specialist ambition
- [ ] Failure increases specialist strictness
- [ ] Ambition changes affect routing
- [ ] Test passes

**Time**: 8 hours

---

### ISSUE #4: Add Integration Tests (4 hours)

**Problem**: No tests verify end-to-end system behavior

**File**: Create `core/hypervisor/src/phase1_integration_tests.rs`

```rust
#[cfg(test)]
mod phase1_integration {
    use crate::autonomic_loop::*;
    use crate::dopamine_system::*;
    use crate::unified_learning::*;
    use crate::biology::*;

    #[test]
    fn test_dopamine_changes_behavior() {
        // Create system
        let mut loop_obj = AutonomicNervousSystem::new();
        
        // Record initial routing weights
        let initial_weight = loop_obj.learning.get_specialist_weight("spec_a");
        
        // Simulate task execution with success
        let outcome = ExecutionOutcome::Success;
        let signal = loop_obj.dopamine_system.process_event(&outcome);
        loop_obj.learning.learn_from_dopamine(&signal);
        
        // Verify routing weight increased
        let new_weight = loop_obj.learning.get_specialist_weight("spec_a");
        assert!(new_weight > initial_weight);
    }

    #[test]
    fn test_token_depletion_stops_execution() {
        let mut loop_obj = AutonomicNervousSystem::new();
        
        // Set specialist tokens to zero
        loop_obj.specialists[0].metabolism.current_tokens = 0.0;
        
        // Try to execute 10 times
        for _ in 0..10 {
            loop_obj.tick();
        }
        
        // Specialist should have zero executions
        assert_eq!(loop_obj.specialists[0].execution_count, 0);
    }

    #[test]
    fn test_enzyme_results_propagate() {
        let mut runner = EnzymeRunner::new();
        
        // Create task with expected output
        let output_data = vec![42, 99, 12];
        let result = runner.run_enzyme("test_task", output_data.clone());
        
        // Verify output reached caller
        assert_eq!(result.unwrap(), output_data);
    }

    #[test]
    fn test_core_feedback_loop() {
        // Full loop test: Execution → Dopamine → Learning → Behavior change
        let mut loop_obj = AutonomicNervousSystem::new();
        
        // Baseline: Specialist weight
        let baseline = loop_obj.learning.get_specialist_weight("spec_a");
        
        // Execute task
        loop_obj.execute_task(Task::new("spec_a", "process"));
        
        // System processes outcome
        loop_obj.tick();
        
        // Verify behavior changed
        let new_weight = loop_obj.learning.get_specialist_weight("spec_a");
        assert_ne!(baseline, new_weight);  // Learning happened
    }
}
```

**Verification**:
- [ ] 4 integration tests added
- [ ] All tests passing
- [ ] Tests verify end-to-end behavior
- [ ] Add to CI pipeline

**Time**: 4 hours

---

### PHASE I SUMMARY

| Fix | Hours | Status |
|-----|-------|--------|
| Extract enzyme results | 2 | 🔄 In Progress |
| Activate token system | 6 | 🔄 In Progress |
| Wire dopamine to learning | 8 | 🔄 In Progress |
| Add integration tests | 4 | 🔄 In Progress |
| **TOTAL** | **20** | - |

**Gate**: All 4 integration tests must pass before moving to Phase II

---

## PHASE II: COMPLETE INTEGRATION (WEEKS 2-3)

**Goal**: All computed data is actually used  
**Effort**: 60 hours  
**Success Criteria**:
- ✓ Task classification influences routing
- ✓ Load predictions cause backpressure
- ✓ Registry synchronization working
- ✓ Specialist memory consulted
- ✓ All 25+ TODOs resolved
- ✓ 15+ integration tests passing

---

### INTEGRATION #1: Task Classification → Routing (6 hours)

**Problem**: Tasks perfectly classified but routing ignores classification

**Current Flow**:
```
Task → Classify perfectly ✓ → Route to WASM anyway ✗
```

**Target Flow**:
```
Task → Classify (CPU vs I/O vs WASM) → Route to appropriate executor
```

**Files Involved**:
- task_analysis.rs:77-140 - Classification (working ✓)
- task_routing.rs:50-80 - Routing (broken ✗)

**Implementation**:

**File**: `core/hypervisor/src/task_routing.rs`

1. **Import classification** (around line 10):
   ```rust
   use crate::task_analysis::{TaskAnalyzer, TaskClassification};
   ```

2. **Route based on classification** (replace lines 70-80):
   ```rust
   // BEFORE: Always WASM
   route_to_wasm_enzyme(task)
   
   // AFTER: Route by classification
   let analysis = self.task_analyzer.analyze_task(&task)?;
   
   match analysis.classification {
       TaskClassification::CpuIntensive => {
           route_to_thread_pool(task)  // Use thread pool
       }
       TaskClassification::IoIntensive => {
           route_to_async_executor(task)  // Use async
       }
       TaskClassification::WasmSuitable => {
           route_to_wasm_enzyme(task)  // Use WASM
       }
       TaskClassification::GpuAccelerable => {
           route_to_gpu_specialist(task)  // GPU if available
       }
   }
   ```

3. **Add routing strategies** (new methods):
   ```rust
   fn route_to_thread_pool(&mut self, task: Task) -> Result<ExecutionRoute, String> {
       // Find thread pool specialist
       // Return route to thread pool
   }
   
   fn route_to_async_executor(&mut self, task: Task) -> Result<ExecutionRoute, String> {
       // Find async specialist
       // Return route to async executor
   }
   ```

4. **Add test**:
   ```rust
   #[test]
   fn test_cpu_intensive_not_routed_to_wasm() {
       let mut router = TaskRouter::new();
       
       let cpu_task = Task::with_classification(
           "matrix_multiply",
           TaskClassification::CpuIntensive
       );
       
       let route = router.route_task(cpu_task).unwrap();
       
       assert_ne!(route.executor_type, ExecutorType::WASM);
       assert_eq!(route.executor_type, ExecutorType::ThreadPool);
   }
   ```

**Verification**:
- [ ] Tasks classified → Routing changes
- [ ] CPU tasks skip WASM
- [ ] I/O tasks use async
- [ ] GPU tasks routed accordingly
- [ ] Test passes

**Time**: 6 hours

---

### INTEGRATION #2: Load Predictions → Backpressure (6 hours)

**Problem**: Load predicted but never consulted; no backpressure

**Current Flow**:
```
High load → Predict "Reduce" ✓ → Ignore it ✗ → Queue explodes
```

**Target Flow**:
```
High load → Predict "Reduce" → Reject new tasks → Backpressure
```

**Files Involved**:
- predictive_load_balancer.rs:140-200 - Predictions (working ✓)
- autonomic_loop.rs - Never consults (broken ✗)

**Implementation**:

**File**: `core/hypervisor/src/autonomic_loop.rs`

1. **Import load balancer** (around line 50):
   ```rust
   use crate::predictive_load_balancer::PredictiveLoadBalancer;
   ```

2. **Before accepting task** (around line 500):
   ```rust
   // BEFORE: Accept all tasks
   fn accept_task(&mut self, task: Task) -> Result<(), String> {
       self.task_queue.push(task);
       Ok(())
   }
   
   // AFTER: Check load first
   fn accept_task(&mut self, task: Task) -> Result<(), String> {
       // Get load prediction
       let prediction = self.load_balancer.predict_load()?;
       
       match prediction.recommendation {
           LoadRecommendation::AcceptMore => {
               self.task_queue.push(task);
               Ok(())
           }
           LoadRecommendation::Balanced => {
               if self.task_queue.len() < MAX_QUEUE_SIZE {
                   self.task_queue.push(task);
                   Ok(())
               } else {
                   Err("Queue full, backpressure active".to_string())
               }
           }
           LoadRecommendation::Reduce => {
               // Reject new tasks
               Err("System under load, rejecting tasks".to_string())
           }
           LoadRecommendation::Reject => {
               // Hard stop
               Err("System overloaded, emergency stop".to_string())
           }
       }
   }
   ```

3. **Per tick: update load balancer** (around line 400):
   ```rust
   // Update load metrics for prediction
   self.load_balancer.update_metrics(
       self.task_queue.len(),
       self.specialist_utilization(),
       self.avg_response_time,
   );
   ```

4. **Add test**:
   ```rust
   #[test]
   fn test_high_load_triggers_backpressure() {
       let mut loop_obj = AutonomicNervousSystem::new();
       
       // Fill queue to high level
       for _ in 0..100 {
           loop_obj.task_queue.push(dummy_task());
       }
       
       // Update load prediction
       loop_obj.load_balancer.update_metrics(
           loop_obj.task_queue.len(),
           0.95,  // 95% utilization
           500,   // 500ms response time
       );
       
       // Try to add new task
       let result = loop_obj.accept_task(new_task());
       
       assert!(result.is_err());  // Should be rejected
   }
   ```

**Verification**:
- [ ] Load consulted before accepting tasks
- [ ] High load → rejection active
- [ ] Queue size bounded
- [ ] Test passes

**Time**: 6 hours

---

### INTEGRATION #3: Registry Synchronization (16 hours)

**Problem**: 18+ adapters return `Ok(())` without syncing; master registry empty

**Current Flow**:
```
Sub-registry changes → Call sync → Returns Ok() ✓ but no actual sync ✗
```

**Target Flow**:
```
Sub-registry changes → Sync to master → Master has unified view
```

**Files Involved**: 18+ adapter files in `registry_adapters/`

**Implementation**:

**File**: `core/hypervisor/src/registry_adapters/specialist_registry_adapter.rs` (template for all 18)

1. **Update trait** (if needed):
   ```rust
   pub trait RegistryAdapter {
       fn synchronize_state(&mut self, ctx: &WorkspaceContext) -> Result<SyncResult, String>;
       fn get_entities(&self) -> Vec<EntityInfo>;
   }
   ```

2. **Implement actual sync logic** (replace lines 60-62):
   ```rust
   // BEFORE: Stub
   fn synchronize_state(&mut self, _ctx: &WorkspaceContext) -> Result<(), String> {
       Ok(())  // ← Fake
   }
   
   // AFTER: Real sync
   fn synchronize_state(&mut self, ctx: &WorkspaceContext) -> Result<SyncResult, String> {
       // 1. Collect all entities from this sub-registry
       let mut entities = Vec::new();
       for specialist in self.specialists.values() {
           entities.push(EntityInfo {
               entity_type: "specialist".to_string(),
               entity_id: specialist.id.clone(),
               data: serde_json::to_value(specialist)?,
               timestamp: std::time::SystemTime::now(),
           });
       }
       
       // 2. Send to master registry
       let sync_result = ctx.master_registry
           .register_entities(&entities)?;
       
       // 3. Verify sync completed
       let verified = ctx.master_registry.verify_entities(&entities)?;
       
       if verified {
           Ok(SyncResult {
               entities_synced: entities.len() as u32,
               timestamp: std::time::SystemTime::now(),
           })
       } else {
           Err("Verification failed after sync".to_string())
       }
   }
   ```

3. **Apply to all 18 adapters** (6 hours):
   - [ ] specialist_registry_adapter.rs
   - [ ] chromosome_registry_adapter.rs
   - [ ] component_registry_adapter.rs
   - [ ] llm_model_registry_adapter.rs
   - [ ] link_registry_adapter.rs
   - [ ] federation_model_registry_adapter.rs
   - [ ] hox_registry_adapter.rs
   - [ ] unified_registry_adapter.rs
   - [ ] distributed_specialist_registry_adapter.rs
   - [ ] (... 9 more)

4. **Create master registry aggregator** (4 hours):
   ```rust
   // File: core/hypervisor/src/master_registry_aggregator.rs
   pub struct MasterRegistryAggregator {
       entities: HashMap<String, EntityInfo>,
       sync_timestamp: SystemTime,
   }
   
   impl MasterRegistryAggregator {
       pub fn register_entities(&mut self, entities: &[EntityInfo]) -> Result<(), String> {
           for entity in entities {
               self.entities.insert(entity.entity_id.clone(), entity.clone());
           }
           self.sync_timestamp = SystemTime::now();
           Ok(())
       }
       
       pub fn verify_entities(&self, entities: &[EntityInfo]) -> Result<bool, String> {
           for entity in entities {
               if !self.entities.contains_key(&entity.entity_id) {
                   return Ok(false);  // Not synced
               }
           }
           Ok(true)
       }
       
       pub fn get_all_entities(&self) -> Vec<EntityInfo> {
           self.entities.values().cloned().collect()
       }
       
       pub fn query_entity(&self, entity_id: &str) -> Option<EntityInfo> {
           self.entities.get(entity_id).cloned()
       }
   }
   ```

5. **Wire into autonomic loop** (2 hours):
   ```rust
   // autonomic_loop.rs, each tick:
   pub fn tick(&mut self) {
       // ... existing code ...
       
       // NEW: Sync all registries
       for adapter in self.registry_adapters.iter_mut() {
           let result = adapter.synchronize_state(&self.workspace_ctx)?;
           log!("Synced {} entities", result.entities_synced);
       }
       
       // Now master registry has unified view
       let all_entities = self.master_registry.get_all_entities();
       // Use for decision-making
   }
   ```

6. **Add test** (2 hours):
   ```rust
   #[test]
   fn test_registry_sync_chain() {
       let mut master = MasterRegistryAggregator::new();
       
       // Create entities from sub-registry
       let entities = vec![
           EntityInfo {
               entity_type: "specialist".to_string(),
               entity_id: "spec_1".to_string(),
               data: json!({"name": "Specialist 1"}),
               timestamp: SystemTime::now(),
           },
       ];
       
       // Sync
       master.register_entities(&entities).unwrap();
       
       // Verify
       assert!(master.verify_entities(&entities).unwrap());
       
       // Query
       let entity = master.query_entity("spec_1").unwrap();
       assert_eq!(entity.entity_id, "spec_1");
   }
   ```

**Verification**:
- [ ] All 18 adapters sync real data
- [ ] Master registry populated
- [ ] No more `Ok()` stubs
- [ ] Query returns results
- [ ] Test passes

**Time**: 16 hours

---

### INTEGRATION #4: Specialist Memory Consultation (6 hours)

**Problem**: Memory stored but never queried; specialist starts fresh each time

**Current Flow**:
```
Route specialist → Check memory ✓ (in module) → Ignore memory ✗ → Execute fresh
```

**Target Flow**:
```
Route specialist → Check memory → Use history to inform decision → Execute
```

**Files Involved**:
- specialist_memory.rs:1-50 - Storage (working ✓)
- autonomic_loop.rs - Never queries (broken ✗)

**Implementation**:

**File**: `core/hypervisor/src/autonomic_loop.rs`

1. **Before specialist selection** (around line 480):
   ```rust
   // NEW: Query specialist memory before selection
   fn select_best_specialist(&mut self, task: &Task) -> Result<SpecialistId, String> {
       let mut candidates = self.get_suitable_specialists(task)?;
       
       // Score each candidate by memory
       for candidate in &mut candidates {
           // Query memory for this specialist
           if let Ok(memories) = self.specialist_memory
               .query_memories_for_specialist(&candidate.id) {
               
               // Analyze success rate from memory
               let success_rate = self.compute_success_rate(&memories);
               
               // Boost score for high-success specialists
               candidate.score *= (1.0 + success_rate * 0.5);
           }
       }
       
       // Select highest-scoring candidate
       candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
       Ok(candidates[0].id.clone())
   }
   ```

2. **After execution: Store memory** (around line 620):
   ```rust
   // AFTER: Store outcome in specialist memory
   let outcome = self.execute_on_specialist(&task, &specialist_id)?;
   
   self.specialist_memory.store_memory(
       &specialist_id,
       MemoryEntry {
           memory_type: MemoryType::Execution,
           timestamp: SystemTime::now(),
           data: json!({
               "task_id": task.id,
               "outcome": outcome.status,
               "duration_ms": outcome.duration_ms,
               "success": outcome.status == ExecutionStatus::Success,
           }),
       }
   )?;
   ```

3. **Compute success rate** (new method):
   ```rust
   fn compute_success_rate(&self, memories: &[MemoryEntry]) -> f32 {
       if memories.is_empty() {
           return 0.5;  // Neutral if no history
       }
       
       let successes = memories.iter()
           .filter(|m| {
               m.data["success"].as_bool().unwrap_or(false)
           })
           .count();
       
       successes as f32 / memories.len() as f32
   }
   ```

4. **Add test**:
   ```rust
   #[test]
   fn test_specialist_memory_influences_routing() {
       let mut loop_obj = AutonomicNervousSystem::new();
       
       // Store successful history for spec_a
       for _ in 0..10 {
           loop_obj.specialist_memory.store_memory(
               "spec_a",
               MemoryEntry {
                   memory_type: MemoryType::Execution,
                   timestamp: SystemTime::now(),
                   data: json!({"success": true}),
               }
           ).unwrap();
       }
       
       // Store failing history for spec_b
       for _ in 0..10 {
           loop_obj.specialist_memory.store_memory(
               "spec_b",
               MemoryEntry {
                   memory_type: MemoryType::Execution,
                   timestamp: SystemTime::now(),
                   data: json!({"success": false}),
               }
           ).unwrap();
       }
       
       // Route task
       let selected = loop_obj.select_best_specialist(&task).unwrap();
       
       // Should select spec_a (high success rate)
       assert_eq!(selected, "spec_a");
   }
   ```

**Verification**:
- [ ] Memory queried before selection
- [ ] Success rates influence routing
- [ ] History drives decision-making
- [ ] Test passes

**Time**: 6 hours

---

### INTEGRATION #5: Resolve All TODOs (20 hours)

**Problem**: 25+ TODO comments throughout codebase marking broken integration points

**Approach**:

1. **Find all TODOs** (1 hour):
   ```bash
   grep -r "TODO:" core/hypervisor/src/ --include="*.rs" | wc -l
   ```

2. **Categorize TODOs** (2 hours):
   - **Learning TODOs**: 8 items (unified_learning.rs, dopamine_system.rs)
   - **Routing TODOs**: 6 items (task_routing.rs, task_analysis.rs)
   - **Execution TODOs**: 4 items (autonomic_loop.rs, enzyme_runner.rs)
   - **Memory TODOs**: 3 items (specialist_memory.rs)
   - **Registry TODOs**: 2 items (registry_adapters/)
   - **Other TODOs**: 2 items (misc)

3. **Fix each category** (15 hours distributed):

**Learning TODOs** (3 hours):
   - Wire dopamine to learning ← Done in Phase I
   - Store execution results in learning state
   - Query learning weights during routing

**Routing TODOs** (2 hours):
   - Use task classification ← Done above
   - Query load predictions
   - Consult specialist memory ← Done above

**Execution TODOs** (4 hours):
   - Extract enzyme results properly ← Done in Phase I
   - Store task outputs
   - Update execution metrics
   - Record task completion status

**Memory TODOs** (2 hours):
   - Query memories properly
   - Store outcomes in memory
   - Cleanup old memories

**Registry TODOs** (2 hours):
   - Sync all adapters ← Done above
   - Verify sync completion
   - Query master registry

4. **Create TODO resolution tracker**:
   ```rust
   // File: core/hypervisor/src/todo_resolution.md
   # TODO Resolution Tracker
   
   ## PHASE II TODOs
   - [ ] Learning integration (autonomic_loop.rs:156)
   - [ ] Task data wiring (task_routing.rs:166)
   - [ ] Enzyme result handling (enzyme_runner.rs:86)
   - [ ] Registry sync implementation (registry_adapters/*:60)
   - ... (25+ total)
   ```

**Verification**:
- [ ] All 25+ TODOs resolved
- [ ] Each TODO has implementation
- [ ] Each TODO has test
- [ ] No remaining "TODO:" in code

**Time**: 20 hours

---

### INTEGRATION #6: Add 15+ Integration Tests (12 hours)

**Create comprehensive integration tests**:

**File**: `core/hypervisor/src/phase2_integration_tests.rs`

```rust
#[cfg(test)]
mod phase2_integration {
    // 15+ integration tests covering:
    
    #[test]
    fn test_task_classification_influences_routing() { }
    
    #[test]
    fn test_load_high_blocks_new_tasks() { }
    
    #[test]
    fn test_registry_sync_propagates_changes() { }
    
    #[test]
    fn test_specialist_memory_persists_across_cycles() { }
    
    #[test]
    fn test_token_consumption_prevents_overload() { }
    
    #[test]
    fn test_dopamine_learning_shapes_behavior() { }
    
    #[test]
    fn test_enzyme_results_available_to_learning() { }
    
    #[test]
    fn test_full_cycle_with_feedback() { }
    
    // ... 7 more
}
```

**Verification**:
- [ ] 15+ tests created
- [ ] All tests passing
- [ ] 90%+ code coverage

**Time**: 12 hours

---

### PHASE II SUMMARY

| Integration | Hours | Status |
|------------|-------|--------|
| Task classification → routing | 6 | 🔄 In Progress |
| Load predictions → backpressure | 6 | 🔄 In Progress |
| Registry synchronization | 16 | 🔄 In Progress |
| Specialist memory consultation | 6 | 🔄 In Progress |
| Resolve all TODOs | 20 | 🔄 In Progress |
| Integration tests | 12 | 🔄 In Progress |
| **TOTAL** | **66** | - |

**Gate**: 15 integration tests must pass + 90%+ code coverage before Phase III

---

## PHASE III: CLEAN UP & OPTIMIZE (WEEK 4)

**Goal**: Defragment codebase, remove bloat, prepare for production  
**Effort**: 40 hours  
**Success Criteria**:
- ✓ Reduced from 101 → 40 modules
- ✓ All vestigial code archived
- ✓ Clean dependency graph
- ✓ Production readiness verified

---

### CLEANUP #1: Archive Unused Modules (10 hours)

**Modules to Archive** (10+ items):

```
curiosity_enzyme.rs                 ← Never called from autonomic_loop
diplomat_enzyme.rs                  ← Never called from autonomic_loop
self_correction_enzyme.rs           ← Imported but never called
neural_pruning.rs                   ← Imported but never called
epigenetic_gate.rs                  ← Never imported
batch_processor.rs                  ← Premature optimization
adaptive_learning_rate.rs           ← Needs gradients (don't exist)
distributed_checkpoint.rs           ← Can't serialize (disabled for now)
concept_drift.rs                    ← Imported but never called
(and 10+ more)
```

**Implementation** (2 hours per module):

1. **Create archive directory** (0.5 hours):
   ```bash
   mkdir core/hypervisor/src/_archive
   ```

2. **Move each unused module** (0.5 hours each):
   ```bash
   mv core/hypervisor/src/curiosity_enzyme.rs core/hypervisor/src/_archive/
   mv core/hypervisor/src/diplomat_enzyme.rs core/hypervisor/src/_archive/
   # ... etc
   ```

3. **Remove imports from lib.rs** (5 hours):
   - Remove 10+ `pub mod` lines
   - Remove 10+ `pub use` lines
   - Clean up documentation

4. **Create archive manifest** (1 hour):
   ```rust
   // File: core/hypervisor/src/_archive/README.md
   # Archived Modules
   
   These modules are not currently used but may be valuable in future:
   
   - curiosity_enzyme.rs - Implemented curiosity-driven learning
   - diplomat_enzyme.rs - Inter-specialist negotiation
   - self_correction_enzyme.rs - Self-correcting error handling
   - neural_pruning.rs - Synaptic pruning logic
   - epigenetic_gate.rs - Epigenetic regulation
   - batch_processor.rs - Task batching (premature)
   - adaptive_learning_rate.rs - Learning rate tuning (waiting for gradients)
   - distributed_checkpoint.rs - Distributed state (waiting for serialization)
   - concept_drift.rs - Drift detection
   
   To reactivate a module:
   1. Move from _archive/ back to src/
   2. Re-add exports to lib.rs
   3. Update autonomic loop integration if needed
   4. Add tests
   5. Verify integration
   ```

**Verification**:
- [ ] 10+ modules archived
- [ ] lib.rs cleaned
- [ ] Build succeeds
- [ ] Tests still pass

**Time**: 10 hours

---

### CLEANUP #2: Consolidate Duplicate Modules (8 hours)

**Duplicates to Consolidate**:

```
Multiple registry implementations:
├─ unified_registry.rs
├─ hox_registry.rs
├─ hybrid_master_registry.rs
└─ master_registry_aggregator.rs (new from Phase II)
→ Keep: master_registry_aggregator (unified)
→ Archive: others

Multiple learning systems:
├─ unified_learning.rs
├─ adaptive_learning_rate.rs
└─ learning_system.rs (if exists)
→ Keep: unified_learning
→ Archive: others

Multiple routing implementations:
├─ task_routing.rs
├─ intelligent_routing.rs
└─ consensus_routing.rs (if exists)
→ Keep: task_routing (now with classification)
→ Archive: others
```

**Implementation**:

1. **Audit module relationships** (2 hours):
   - Map each duplicate's purpose
   - Verify one is superset of others
   - Plan consolidation

2. **Move logic to keeper module** (3 hours):
   - Copy relevant functions from duplicates
   - Rename to avoid conflicts
   - Update logic to use unified approach

3. **Remove duplicates** (2 hours):
   - Delete duplicate files
   - Remove imports from lib.rs
   - Archive originals

4. **Test** (1 hour):
   - Build succeeds
   - All tests pass

**Verification**:
- [ ] Duplicates identified
- [ ] Logic consolidated
- [ ] Imports cleaned
- [ ] Tests pass

**Time**: 8 hours

---

### CLEANUP #3: Remove Premature Optimizations (6 hours)

**Premature Optimizations**:

```
batch_processor.rs          ← Added before learning works
distributed_checkpoint.rs   ← Added before serialization works
adaptive_learning_rate.rs   ← Added before gradients implemented
wgpu_reflex_pipeline.rs     ← GPU processing not integrated
SIMD acceleration code      ← Not benchmarked yet
```

**Decision Matrix**:

| Optimization | Status | Decision |
|---|---|---|
| Batch processor | Premature | Archive |
| Distributed checkpoint | Blocked | Archive |
| Adaptive learning rate | Incomplete | Archive |
| WGPU pipeline | Unused | Archive |
| SIMD code | Unused | Archive |

**Implementation**:

1. **Review each optimization** (1 hour per):
   - Is it tested?
   - Is it integrated?
   - Is it necessary?

2. **Archive if not integrated** (2 hours):
   - Move to _archive/
   - Document why archived
   - Keep notes on enabling in future

3. **Simplify code** (2 hours):
   - Remove unnecessary complexity
   - Remove optimization-specific code paths
   - Focus on correctness first

**Verification**:
- [ ] Premature code identified
- [ ] Archived appropriately
- [ ] Build clean
- [ ] Tests pass

**Time**: 6 hours

---

### CLEANUP #4: Production Readiness Verification (16 hours)

**Verification Checklist**:

**1. Code Quality Review** (4 hours):
```
- [ ] No TODO comments remaining
- [ ] No XXX/FIXME comments
- [ ] No debug logging
- [ ] No unwrap() calls (use ? or match)
- [ ] No panic!() calls
- [ ] Error handling complete
- [ ] All public APIs documented
```

**2. Testing Coverage** (4 hours):
```
- [ ] Unit tests: 90%+ coverage
- [ ] Integration tests: 15+ tests
- [ ] Stress tests: Pass 99.5%
- [ ] Failure cases: Tested
- [ ] Edge cases: Covered
- [ ] Performance: Within budget
```

**3. Performance Validation** (4 hours):
```
- [ ] Autonomic tick: < 10ms
- [ ] Task routing: < 5μs
- [ ] Learning update: < 500μs
- [ ] Memory usage: < 2GB
- [ ] CPU usage: < 80% sustained
- [ ] No memory leaks
- [ ] No performance regressions
```

**4. Dependency Audit** (2 hours):
```
- [ ] No circular dependencies
- [ ] Dependency graph clean
- [ ] All imports used
- [ ] No dead code
- [ ] Module count: 40 (down from 101)
```

**5. Documentation** (2 hours):
```
- [ ] README updated
- [ ] Architecture documented
- [ ] Integration guide written
- [ ] Deployment guide ready
- [ ] API reference complete
- [ ] Troubleshooting guide included
```

**Implementation**:

**File**: `core/hypervisor/PRODUCTION_READINESS_CHECKLIST.md`

```markdown
# Production Readiness Checklist

## Code Quality (/20)
- [ ] No TODOs remaining (5 pts)
- [ ] All APIs documented (5 pts)
- [ ] Error handling complete (5 pts)
- [ ] No unwrap() calls (5 pts)

## Testing (/20)
- [ ] 90%+ unit test coverage (5 pts)
- [ ] 15+ integration tests (5 pts)
- [ ] Stress test passing (5 pts)
- [ ] Edge cases covered (5 pts)

## Performance (/20)
- [ ] Autonomic tick < 10ms (5 pts)
- [ ] Memory < 2GB (5 pts)
- [ ] CPU < 80% sustained (5 pts)
- [ ] No memory leaks (5 pts)

## Architecture (/20)
- [ ] Clean dependency graph (5 pts)
- [ ] 40 modules (down from 101) (5 pts)
- [ ] All features integrated (5 pts)
- [ ] Feedback loops verified (5 pts)

## Documentation (/20)
- [ ] README complete (5 pts)
- [ ] API reference done (5 pts)
- [ ] Deployment guide ready (5 pts)
- [ ] Troubleshooting included (5 pts)

## TOTAL: __/100

Must achieve: 85+ to deploy
```

**Verification**:
- [ ] Checklist created
- [ ] All items verified
- [ ] Score: 85+
- [ ] Ready to sign off

**Time**: 16 hours

---

### PHASE III SUMMARY

| Cleanup | Hours | Status |
|---------|-------|--------|
| Archive unused modules | 10 | 🔄 In Progress |
| Consolidate duplicates | 8 | 🔄 In Progress |
| Remove premature optimizations | 6 | 🔄 In Progress |
| Production readiness verification | 16 | 🔄 In Progress |
| **TOTAL** | **40** | - |

**Gate**: Production Readiness Checklist score must be 85+ before deployment

---

## INTEGRATION TESTING ACROSS ALL PHASES

**Test Structure**:

```
core/hypervisor/src/
├─ phase1_integration_tests.rs       (4 tests)
├─ phase2_integration_tests.rs       (15+ tests)
├─ production_readiness_tests.rs     (10+ tests)
└─ e2e_integration_tests.rs          (5+ tests)
```

**Full Integration Test Suite**:

```rust
#[cfg(test)]
mod full_integration_tests {
    // E2E tests covering entire system
    
    #[test]
    fn test_complete_task_execution_flow() {
        // Task → Classification → Routing → Execution → Dopamine → Learning
    }
    
    #[test]
    fn test_system_adapts_under_load() {
        // Load high → Backpressure → Queue managed → System stable
    }
    
    #[test]
    fn test_specialist_learns_from_failures() {
        // Failure → Dopamine signal → Learning update → Behavior changes
    }
    
    #[test]
    fn test_registry_remains_consistent() {
        // Changes → Sync → Master registry → Queries return latest data
    }
    
    #[test]
    fn test_tokens_prevent_resource_exhaustion() {
        // High execution load → Tokens deplete → Execution halts → Regenerates
    }
}
```

**Minimum Test Requirements**:
- Phase I: 4 tests (all passing)
- Phase II: 15 tests (all passing)
- Phase III: 10 tests (all passing)
- **Total: 29+ integration tests**

---

## OVERALL INTEGRATION PLAN SUMMARY

### Timeline

```
WEEK 1: Phase I - Critical Breaks (20 hours)
├─ Mon-Tue: Fix enzyme results + token system (8h)
├─ Wed-Thu: Wire dopamine + integration tests (12h)
└─ Fri: Verify all Phase I tests passing ✓

WEEK 2-3: Phase II - Complete Integration (60 hours)
├─ Mon-Tue: Task classification + load predictions (12h)
├─ Wed-Thu: Registry sync + specialist memory (22h)
├─ Fri (Wk2): TODOs + tests partial (8h)
├─ Mon-Tue (Wk3): Finish TODOs + all tests (12h)
├─ Wed-Thu (Wk3): Final verification + cleanup (6h)
└─ Fri (Wk3): Verify all Phase II tests passing ✓

WEEK 4: Phase III - Cleanup & Optimization (40 hours)
├─ Mon-Tue: Archive modules + consolidate (18h)
├─ Wed: Premature optimization removal (6h)
├─ Thu-Fri: Production readiness (16h)
└─ Fri EOD: Sign off for production ✓
```

### Effort Summary

| Phase | Hours | Week | Status |
|-------|-------|------|--------|
| I: Critical Breaks | 20 | Week 1 | 🔄 |
| II: Complete Integration | 66 | Weeks 2-3 | 🔄 |
| III: Cleanup | 40 | Week 4 | 🔄 |
| **TOTAL** | **126** | **4** | 🔄 |

### Deliverables by Phase

**Phase I**:
- ✓ Enzyme results working
- ✓ Token system active
- ✓ Dopamine → Learning wired
- ✓ 4 integration tests

**Phase II**:
- ✓ Task classification → routing
- ✓ Load prediction → backpressure
- ✓ Registry sync working
- ✓ Specialist memory consulted
- ✓ All 25+ TODOs resolved
- ✓ 15+ integration tests

**Phase III**:
- ✓ 101 → 40 modules
- ✓ No duplicate code
- ✓ All premature optimizations archived
- ✓ Production Readiness Score: 85+
- ✓ Ready to deploy

### Success Metrics

**Code Quality**:
- ✓ 100% of TODO comments resolved
- ✓ 90%+ code coverage
- ✓ 0 unwrap() calls in critical path
- ✓ Clean dependency graph

**Integration**:
- ✓ All feedback loops working
- ✓ All computed data used
- ✓ 40 fully integrated modules
- ✓ 30+ integration tests passing

**Performance**:
- ✓ Autonomic tick < 10ms
- ✓ Task routing < 5μs
- ✓ Memory < 2GB
- ✓ 99%+ system availability

**Defragmentation**:
- ✓ 101 → 40 modules (60% reduction)
- ✓ 10+ vestigial modules archived
- ✓ 0 duplicate implementations
- ✓ Single source of truth per feature

---

## EXECUTION STRATEGY

### Daily Standups

Each day (5pm):
- [ ] What was completed
- [ ] What's in progress
- [ ] Blockers
- [ ] Tomorrow's plan

### Weekly Reviews

Each Friday (end of day):
- [ ] Phase checkpoint
- [ ] Gate review (tests passing?)
- [ ] Adjust timeline if needed
- [ ] Next week preview

### Phase Gates (Must Pass Before Next Phase)

**Phase I → II Gate**:
- [ ] All 4 Phase I tests passing
- [ ] No build errors
- [ ] Enzyme, tokens, dopamine verified
- [ ] Sign-off: Yes/No

**Phase II → III Gate**:
- [ ] All 15+ Phase II tests passing
- [ ] 90%+ coverage
- [ ] No TODOs remaining
- [ ] Sign-off: Yes/No

**Phase III → Production Gate**:
- [ ] Production Readiness Score 85+
- [ ] All 30+ integration tests passing
- [ ] Performance benchmarks met
- [ ] Documentation complete
- [ ] Sign-off: Yes/No → **DEPLOY**

---

## RISK MITIGATION

### Risk: Phase I delays block Phase II

**Mitigation**:
- Start Phase II preparatory work in parallel
- Have integration test suite ready
- Pre-audit code for dependencies

### Risk: Registry sync is more complex than estimated

**Mitigation**:
- Start with 3 adapters, scale to 18
- Create helper utilities to reduce code duplication
- Budget additional 5 hours

### Risk: New integration issues discovered

**Mitigation**:
- Reserve 10-hour contingency in Phase II
- Document issues as they arise
- Plan for next iteration if needed

### Risk: Performance doesn't meet targets

**Mitigation**:
- Establish baselines in Phase I
- Profile early and often
- Archive optimizations in Phase III if needed

---

## SUCCESS DEFINITION

**Integration plan is complete when**:

✓ All 40 hours Phase I work complete  
✓ All 66 hours Phase II work complete  
✓ All 40 hours Phase III work complete  
✓ 30+ integration tests all passing  
✓ Production Readiness Score: 85+  
✓ Coherence score improves: 5.8 → 9.0+  
✓ Module count reduced: 101 → 40  
✓ Feedback loops all operational  
✓ Zero stubbed implementations remaining  
✓ Ready to deploy to production  

---

**This integration plan will transform Aaroneous from a 37% integrated system into a 100% cohesive, production-ready platform.**

