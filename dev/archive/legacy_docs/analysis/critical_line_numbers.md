# SPECIFIC LINE NUMBERS - CRITICAL STUBS REQUIRING FIXES

## IMMEDIATE FIX LOCATIONS

### CRITICAL ISSUE #1: Registry Adapter Synchronization Stubs (30 locations)

**File**: `core/hypervisor/src/registry_adapters.rs`

```
Line 27-28:   UnifiedRegistryAdapter::initialize() → Ok(())
Line 42-44:   UnifiedRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 63-65:   FederationModelRegistryAdapter::initialize() → Ok(())
Line 79-81:   FederationModelRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 100-102: LinkRegistryAdapter::initialize() → Ok(())
Line 115-117: LinkRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 136-138: LLMModelRegistryAdapter::initialize() → Ok(())
Line 151-153: LLMModelRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 172-174: ComponentRegistryAdapter::initialize() → Ok(())
Line 187-189: ComponentRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 208-210: SpecialistRegistryAdapter::initialize() → Ok(())
Line 223-225: SpecialistRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 244-246: ChromosomeRegistryAdapter::initialize() → Ok(())
Line 259-261: ChromosomeRegistryAdapter::synchronize_state() → Ok(()) ❌
Line 280-282: (additional adapter)::initialize() → Ok(())
Line 295-297: (additional adapter)::synchronize_state() → Ok(()) ❌
Line 316-318: (additional adapter)::initialize() → Ok(())
Line 331-333: (additional adapter)::synchronize_state() → Ok(()) ❌
```

**Files**: `core/hypervisor/src/registry_adapters/*.rs` (10 individual files)

Each file has same pattern:
```
Line ~15-20:  initialize() → Ok(())
Line ~20-25:  synchronize_state() → Ok(()) ❌
```

Individual adapter files:
- `chromosome_registry_adapter.rs` - Lines 15, 20 ❌
- `component_registry_adapter.rs` - Lines 31, 49 ❌
- `distributed_specialist_registry_adapter.rs` - Lines 15, 20 ❌
- `federation_model_registry_adapter.rs` - Lines 16, 42 ❌
- `hox_registry_adapter.rs` - Lines ~15, ~20 ❌
- `link_registry_adapter.rs` - Lines 16, ~17 ❌
- `llm_model_registry_adapter.rs` - Lines ~15, ~25 ❌
- `specialist_registry_adapter.rs` - Lines ~15, ~25 ❌
- `unified_registry_adapter.rs` - Lines 27, 42 ❌
- (Plus 1 more - verify list)

**TOTAL**: 30+ synchronize_state() implementations are stubbed ❌

---

### CRITICAL ISSUE #2: Enzyme Result Extraction

**File**: `core/hypervisor/src/enzyme_runner.rs`

```
Line 90:  Ok(vec![])  // ← CRITICAL STUB
```

**Problem**:
```rust
pub async fn spawn_enzyme(&self, wasm_path: &str, task_id: &str) -> Result<Vec<u8>> {
    // ... (successful WASM execution) ...
    println!("[EnzymeRunner] WASM Execution Completed Successfully.");
    
    Ok(vec![])  // ← Line 90: Returns empty result
}
```

**Should return**: Actual WASM execution results from memory/return value

**Impact**: ALL task outputs discarded, digestion feedback broken

---

### CRITICAL ISSUE #3: Dopamine Not Integrated

**Files**: 
- `core/hypervisor/src/dopamine_system.rs` (~100)
- `core/hypervisor/src/autonomic_loop.rs` (missing call)

**Problem Location**: 
- Dopamine system computes reward but autonomic loop never queries it
- No integration point found

**Search**: 
```
autonomic_loop.rs - search for "dopamine" or "reward"
Result: NOT FOUND (should be called in run_autonomic_cycle)
```

**Impact**: Reward signal is computed but never used

---

### CRITICAL ISSUE #4: Task Routing Ignores Classification

**File**: `core/hypervisor/src/enzyme_runner.rs`

**Problem**: 
```
task_analysis::classify_task() is called but result never used
All tasks regardless of type go to spawn_enzyme()
```

**Location**: Lines where task routing should use classification
- Look for: "spawn_enzyme" calls that should check task type first
- Current behavior: Unconditional enzyme spawning

**Impact**: Wrong execution method for all tasks

---

## HIGH-PRIORITY ISSUE LOCATIONS

### ISSUE #5: Thermal Metrics Placeholder

**File**: `core/hypervisor/src/hardware_layer.rs`

```
Line ~100:  pub fn get_gpu_load(&self) -> f64 {
                0.5  // ← Placeholder
            }

Line ~105:  pub fn get_thermal_status(&self) -> ThermalStatus {
                ThermalStatus::Unknown  // ← Placeholder
            }
```

**File**: `components/biology/src/thermodynamic_governor.rs`

```
Line ~100:  pub fn compute_thermal_load(&self) -> f64 {
                // Placeholder: GPU and thermal remain placeholder
                // (require NVML/Metal APIs).
                0.5  // ← Returns stub value
            }

Line ~150:  pub fn get_thermal_action(&self) -> ThermodynamicAction {
                // Uses stub thermal data
            }
```

**Impact**: No real thermal throttling

---

### ISSUE #6: Specialist Memory Not Queried

**File**: `core/hypervisor/src/specialist_memory.rs`

**Files using it**: `core/hypervisor/src/autonomic_loop.rs`

**Problem**: 
- Specialist memory stores episodic experiences
- But autonomic_loop never calls `query()` or `get_context()`
- Search autonomic_loop for reference to specialist_memory: NOT FOUND

**Impact**: Experience is collected but never used

---

### ISSUE #7: Unified Learning Incomplete

**File**: `core/hypervisor/src/unified_learning.rs`

```
Line ~150:  pub fn update_reward(&mut self, experience: &Experience) {
                // TODO: integrate with dopamine system
                self.experiences.push(experience.clone());
                // Just accumulates, no actual learning
            }
```

**Problem**: Collects experiences but doesn't train models

**Impact**: No learning from experience

---

## MEDIUM-PRIORITY ISSUE LOCATIONS

### ISSUE #8: Concept Drift Not Called

**File**: `core/hypervisor/src/concept_drift.rs`

```
pub fn detect_shift(&self, state: &[f32]) -> bool {
    // Returns true/false but never called from autonomic loop
}
```

**Called from**: autonomic_loop.rs - NOT FOUND ❌

---

### ISSUE #9: UIGraph Building Incomplete

**File**: `core/hypervisor/src/visual_perception.rs`

```
Line ~185:  pub fn build_ui_graph(&self, elements: &[UIElement]) -> Result<UIGraph> {
                // Sketch only
                // Actual traversal logic incomplete
            }
```

---

### ISSUE #10: Hox Registry Persistence Missing

**File**: `core/hypervisor/src/hox_registry.rs`

```
Line ~250:  pub fn save_to_disk(&self) -> Result<()> {
                // Only logs, doesn't actually save
                info!("Saving registry (not implemented)");
                Ok(())
            }
```

**Should**: Actually persist to disk using rocksdb or SQLite

---

### ISSUE #11: Genetic Distance Metric Missing

**File**: `components/genetics/src/genetics.rs`

**Search for**: `pub fn distance()` - NOT FOUND ❌

**Should exist**: Function to calculate genetic distance between two genomes
- Current: Random breeding
- Needed: Distance-based intelligent pairing

---

### ISSUE #12: Genome Trait Loader Unused

**File**: `core/hypervisor/src/genome_trait_loader.rs`

```
pub fn load_traits(&self, genome: &Genome) -> TraitSet {
    // Loads traits but where are they used?
}
```

**Used in**: autonomic_loop.rs - NOT FOUND ❌

---

### ISSUE #13: Neural Pruning Incomplete

**File**: `core/hypervisor/src/neural_pruning.rs`

```
pub fn prune_weight(&self, _ref: WeightRef) -> Result<()> {
    Ok(())  // Does nothing
}
```

**Should**: Actually prune weights from model

---

### ISSUE #14: Epigenetic Gating Not Applied

**File**: `core/hypervisor/src/epigenetic_orchestrator.rs`

```
pub fn apply_gating(&mut self, specialist_id: u64) -> bool {
    let gated = self.gate_expression(specialist_id);
    // Returns gated state but caller ignores it
    gated
}
```

**Used in**: autonomic_loop.rs - NOT FOUND ❌

---

## SUMMARY TABLE

| Issue # | Severity | File | Line(s) | Type | Impact |
|---------|----------|------|---------|------|--------|
| 1 | 🔴 CRITICAL | registry_adapters/* | 30+ | Stub | Master registry non-functional |
| 2 | 🔴 CRITICAL | enzyme_runner.rs | 90 | Stub | Results discarded |
| 3 | 🔴 CRITICAL | dopamine_system.rs | N/A | Missing | No reward learning |
| 4 | 🔴 CRITICAL | enzyme_runner.rs | Routing | Bug | Wrong execution path |
| 5 | 🟠 HIGH | hardware_layer.rs | 100+ | Stub | No thermal mgmt |
| 6 | 🟡 MEDIUM | specialist_memory.rs | N/A | Missing | Experience unused |
| 7 | 🟡 MEDIUM | unified_learning.rs | 150 | Partial | No learning |
| 8 | 🟡 MEDIUM | concept_drift.rs | N/A | Missing | Drift unused |
| 9 | 🟡 MEDIUM | visual_perception.rs | 185 | Partial | UIGraph incomplete |
| 10 | 🟡 MEDIUM | hox_registry.rs | 250 | Stub | No persistence |
| 11 | 🟡 MEDIUM | genetics.rs | N/A | Missing | Random breeding |
| 12 | 🟡 MEDIUM | genome_trait_loader.rs | N/A | Missing | Traits unused |
| 13 | 🟡 MEDIUM | neural_pruning.rs | ~200 | Stub | Pruning not applied |
| 14 | 🟡 MEDIUM | epigenetic_orchestrator.rs | ~150 | Partial | Gating unused |

---

## FINDING PATTERNS

### Pattern 1: Empty Return Ok(())

Search for:
```
fn \w+.*\) -> Result<\(\), String> \{
\s+Ok\(\(\)\)
```

**Files affected**: All registry adapters (30 files)

### Pattern 2: Returns Empty Collection

Search for:
```
Ok\(vec!\[\]\)
Ok\(HashMap::new\(\)\)
Ok\(None\)
```

**Files affected**: enzyme_runner.rs (line 90)

### Pattern 3: Computed But Never Used

Search for: `let \w+ = .*_task\(` but variable never used afterward

**Files affected**: 
- task_analysis.rs (classify_task result)
- concept_drift.rs (detect_shift result)
- epigenetic_orchestrator.rs (gate_expression result)

### Pattern 4: Missing Integration Points

Search for: Functions that exist but are never called from main loops

**Files affected**:
- specialist_memory.rs (never called from autonomic_loop)
- dopamine_system.rs (never called from autonomic_loop)
- genome_trait_loader.rs (never called from autonomic_loop)

---

## VERIFICATION CHECKLIST

After fixes, verify each location:

- [ ] All 30 registry adapters have real synchronize_state() (not Ok(()))
- [ ] enzyme_runner.rs:90 returns actual results (not vec![])
- [ ] dopamine_system reward integrated into autonomic_loop
- [ ] task_analysis::classify_task() result used for routing
- [ ] hardware_layer.rs returns actual GPU/thermal metrics
- [ ] autonomic_loop queries specialist_memory
- [ ] unified_learning actually trains models
- [ ] concept_drift results used in decisions
- [ ] UIGraph building completed
- [ ] hox_registry.rs persists to disk
- [ ] Genetic distance metric implemented and used
- [ ] genome_trait_loader results used
- [ ] neural_pruning actually prunes weights
- [ ] epigenetic gating applied to execution

---

## QUICK FIXES (Under 1 hour each)

These can be quick wins:

1. Route by task classification (enzyme_runner.rs routing logic): 30 min
2. Add dopamine query to autonomic_loop: 30 min
3. Add specialist_memory query to autonomic_loop: 20 min
4. Add genome_trait_loader usage: 20 min

**Total quick wins: ~2 hours**

---

## BLOCKING FIXES (Multiple hours each)

These require more work:

1. Implement 30 registry adapters synchronize_state(): 100-150 hours ❌
2. Extract enzyme results properly: 8 hours
3. Implement thermal metrics: 25-30 hours
4. Complete unified learning: 15 hours
5. Complete UIGraph building: 10 hours

**Total blocking: ~170-210 hours**

