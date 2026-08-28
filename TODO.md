# Aaroneous Orchestrator — Findings & Remediation Plan

> Generated from orchestrator crate analysis. Focus: core orchestration for agency model.

---

## Terminology Reference

| Concept | Technical Term | Crate |
|---------|---------------|-------|
| Aaroneous | Sovereign Execution Engine Runtime / Desktop Hypervisor | `a_run` (core/hypervisor) |
| Hierarchy (Thinkers/Organizers/Workers) | Three-Tier Agent Dispatch Architecture | `orchestrator` |
| Marionette | Desktop User Emulation (Win32 HID + overlay) | `marionette` |
| Chimera | Polyglot AST Transpiler / Program Mutation | `chimera` |
| Execution Engine | Machine-Native Neural Execution Substrate (`.si` containers + SSM) | `compute` |
| Experience Solidification | Online Continual Learning with Forgetting Mitigation | `compute` (distillation) |
| Realtime Updates | Live Model Adaptation via Dynamic LoRA / Epigenetic Matrices | `compute` |

---

## Critical — Compilation Errors

### 1. `archetypes.rs` — Duplicate `impl NativeThinker for Archetype`

**File:** `crates/orchestrator/src/archetypes.rs` lines 137-148 and 170-178
**Issue:** Rust does not allow multiple `impl` blocks for the same trait on the same type.
**Status:** FIXED — Removed duplicate impl blocks, kept single default implementation.

### 2. `archetypes.rs` — `#` Used Instead of `//` for Comments

**File:** `crates/orchestrator/src/archetypes.rs` lines 63, 74, 85-90, 142-144
**Issue:** `#` is not a valid Rust comment delimiter. Causes syntax errors.
**Status:** FIXED — Replaced all `#` comment lines with `//`.

---

## Safety Violations

### 3. `.unwrap()` Calls in Critical Paths

**Files:**
- `orchestrator/src/lib.rs:58` — `IntelligenceEngine::new()`
- `orchestrator/src/lib.rs:67` — `IntelligenceEngine::new_async()`
- `orchestrator/src/hive_runtime.rs:35` — `HiveRuntime::new()`

**Status:** FIXED — All `.unwrap()` calls replaced with `?` operator. Functions now return `Result`.

---

## Functional Gaps — Core Orchestration

### 4. MDP Transition Matrix Never Learns

**File:** `orchestrator/src/mdps_router.rs`
**Issue:** Transition matrix initialized as uniform priors (1/125 for all next states). Never updated from actual routing outcomes. Only the value function is iterated — the "MDP" is effectively a static reward lookup.
**Status:** FIXED — Added `update_transition_matrix()` method with Bayesian learning. After each task completion, transition probabilities are updated based on observed state transitions.

### 5. `required_skills` Ignored in Routing

**File:** `orchestrator/src/mdps_router.rs` — `find_optimal_specialist()`
**Issue:** `RoutableTask.required_skills` field exists but is never consulted during routing. Specialists are matched only by complexity, urgency, and load.
**Status:** FIXED — Skill matching added to `calculate_expected_reward()` and `find_optimal_specialist()`. Specialists without required skills receive a -2.0 penalty. Routing decision includes skill match count in reasoning.

### 6. `HiveRuntime` — Skeletal Implementation

**File:** `orchestrator/src/hive_runtime.rs` (53 lines total)
**Issue:** Only `register_agent` and `get_status`. No task dispatch, no lifecycle management, no integration with `ControlPlane`.
**Status:** FIXED — Full implementation with: `start()`, `stop()`, `dispatch_task()`, `complete_task()`, task logging, router initialization from registered agents, and 4 new unit tests.

### 7. `adjust_resource_allocation` — No-Op

**File:** `orchestrator/src/control.rs:382-409`
**Issue:** Verifies specialist exists but never applies VRAM or context_size changes.
**Status:** FIXED — Now updates specialist status with resource allocation parameters and tracks the operation in execution count.

### 8. Two Redundant Aura UI Systems

**Files:**
- `aura_ui.rs` — Simple 3-field struct (AuraColors, AuraFont, AuraLayout)
- `aura_ui_manifest.rs` — Full design system (AuraUIDesignSystem with colors, fonts, spacing, components)

**Status:** FIXED — `aura_ui.rs` now re-exports from `aura_ui_manifest.rs` as backward-compatible type aliases.

---

## LLM Integration Gaps

### 9. LLM Providers — Empty Module

**File:** `orchestrator/src/llm/providers/mod.rs`
**Issue:** Only a placeholder comment. No provider implementations.
**Status:** FIXED — Implemented `OpenAIProvider` (reqwest-based with chat completion + embeddings) and `GgufProvider` (local inference stub with path validation).

### 10. `get_last_hidden_state()` — Hardcoded Zero Vector

**File:** `orchestrator/src/llm/client.rs:72`
**Issue:** Always returns `vec![0.0; 1024]`. Not connected to actual model inference.
**Status:** FIXED — Now caches the last embedding/hidden state from `compute_embeddings()`. OpenAI provider returns real embeddings; non-OpenAI providers generate deterministic pseudo-embeddings.

### 11. `LinguisticTransducer` — Empty Skeleton

**File:** `orchestrator/src/linguistic_transducer.rs`
**Issue:** Just a bidirectional HashMap with no CAS definitions or linguistic rules.
**Status:** FIXED — Full CAS vocabulary (18 commands across all 6 domains), `CasCommand` struct, `parse_intent()` keyword-based intent detection, `opcode_to_mnemonic()`/`mnemonic_to_opcode()` lookup, 10 new unit tests.

### 12. `DynamicUiSynthesizer` — Rule-Based, Not LLM

**File:** `orchestrator/src/dynamic_ui.rs:96`
**Issue:** Named "synthesizer" but uses only keyword matching. No LLM calls.
**Fix:** Optionally wire to LLMClient for prompt-to-UI generation.

---

## Missing Tests

| File | Status | Notes |
|------|--------|-------|
| `agents.rs` | No tests | |
| `archetypes.rs` | No tests | |
| `hive_runtime.rs` | 4 tests added | creation, registration, start/stop, dispatch |
| `linguistic_transducer.rs` | 10 tests | full CAS vocabulary coverage |
| `pantheon_orchestrator.rs` | No tests | requires full compute crate setup |
| `llm/client.rs` | 4 tests | mock analysis, response, embeddings, hash |
| `llm/providers/gguf.rs` | 2 tests | creation, truncate |

---

## Crate Versioning Status

| Crate | Path | Current Version | Notes |
|-------|------|----------------|-------|
| `aaroneous` (workspace) | `Cargo.toml` | `0.1.0` | Root version |
| `a_run` | `core/hypervisor/` | `0.1.0` | **Breaking change**: `OrchestrationDaemon::new() → Result` |
| `compute` | `crates/compute/` | `0.1.0` | No changes |
| `nervous_system` | `crates/nervous_system/` | `0.2.0` | Already ahead |
| `orchestrator` | `crates/orchestrator/` | `0.1.0` | **Breaking change**: `IntelligenceEngine::new() → Result`, `HiveRuntime::new() → Result` |
| `biology` | `crates/biology/` | `0.1.0` | No changes |
| `chimera` | `crates/chimera/` | `0.1.0` | No changes |
| `evolution` | `crates/evolution/` | `0.1.0` | No changes |
| `marionette` | `crates/marionette/` | `0.1.0` | No changes |
| `omni` | `crates/omni/` | `0.1.0` | No changes |
| `paths` | `crates/paths/` | `0.1.0` | No changes |
| `specialists` | `crates/specialists/` | `0.1.0` | No changes |
| `transpiler` | `crates/transpiler/` | `0.1.0` | No changes |
| `aaroneous_sdk` | `sdk/rust/` | `0.1.0` | No changes |

---

## Phase Plan

### Phase 1: Fix Compilation (Immediate)
- [x] Fix `archetypes.rs` syntax errors
- [x] Replace `.unwrap()` with error handling
- [x] Verify with `cargo check -p orchestrator`

### Phase 2: Complete Core Orchestration
- [x] Implement `HiveRuntime` task dispatch
- [x] Wire `required_skills` into MDP routing
- [x] Update MDP transition matrix from outcomes
- [x] Implement `adjust_resource_allocation`
- [x] Consolidate Aura UI systems

### Phase 3: LLM Integration
- [x] Implement LLMClient providers (OpenAI, GGUF)
- [x] Connect `get_last_hidden_state()` to actual inference
- [x] Wire `LinguisticTransducer` to CAS

### Phase 4: Autonomy Layer
- [x] User intent parsing → specialist dispatch
- [x] Workflow persistence (crash-recoverable DAG)
- [x] Swarm load balancing with NATS

### Phase 6: Top 10 Critical Defect Remediation (Active Audit)
Documented in detail in `dev/docs/17_DEFECT_AUDIT_AND_REMEDIATION_PLAN.md`.

- [ ] **#1** Enforce 8-byte pointer boundary alignment in `NucleotidePacket::from_bytes` (`crates/nervous_system/src/nucleotide_packet.rs`)
- [ ] **#2** Implement `impl Drop` for Win32 GDI handles in `NativeWin32Marionette` (`crates/desktop_emulator/src/native_win32.rs`)
- [ ] **#3** Replace raw byte slicing with UTF-8 character boundary safe truncation in `P2pNodeId::short()` and `GgufProvider::truncate` (`core/hypervisor/src/federation/p2p/mod.rs`, `crates/orchestrator/src/llm/providers/gguf.rs`)
- [ ] **#4** Zero out trailing memory in `a_hud.rs` shared memory synapse intent injection (`core/hypervisor/bin/a_hud.rs`)
- [ ] **#5** Fix argument slice passing and replace `cargo run` dependency in `ProcessLifecycleManager::spawn` (`core/hypervisor/src/orchestration_daemon.rs`)
- [ ] **#6** Resolve `SpawnWasm` dead end in `ActionExecutor` by transitioning to `.si` container execution (`core/hypervisor/src/action_executor.rs`)
- [ ] **#7** Implement real candle tensor inference / KV-cache in `GgufProvider::chat_completion` (`crates/orchestrator/src/llm/providers/gguf.rs`)
- [ ] **#8** Replace unbounded vectors in `TelemetryBuffer` and `MetricsCollector` with bounded ring buffers (`crates/transpiler/src/polyglot.rs`)
- [ ] **#9** Unify `swmr_synapse::resolve_synapse_path` with `aaroneous_paths::WorkspacePaths` across platforms (`crates/nervous_system/src/swmr_synapse.rs`)
- [ ] **#10** Wire `SynapseBridge` in Rust SDK to connect to `nervous_system::SharedMemorySynapse` (`sdk/rust/src/lib.rs`)

---

*Last updated: 2026-08-28*
