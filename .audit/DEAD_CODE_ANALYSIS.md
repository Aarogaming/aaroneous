# Audit Findings Report - Dead Code & Technical Debt
**Date:** 2026-09-04  
**Scope:** `core/hypervisor/src/` module tree

---

## Summary

This audit identified **13+ dead/unlinked files** in `core/hypervisor/src/` that are NOT exported or referenced in `lib.rs`, plus significant technical debt items requiring attention.

---

## 1. Dead Code Files (Not Exported in lib.rs)

The following files exist in `core/hypervisor/src/` but are **not** imported, referenced, or re-exported anywhere in the codebase:

| File | Purpose (from content) | Status |
|------|----------------------|--------|
| `chaos_monkey.rs` | Chaos engineering simulator stub | Dead |
| `curiosity_enzyme.rs` | Unlinked enzyme runner | Dead |
| `diplomat_enzyme.rs` | Diplomacy protocol stub | Dead |
| `hox_breeding_simulator.rs` | Genetic simulation stub | Dead |
| `advanced_intelligence.rs` | Unused AI framework | Dead |
| `advanced_model_selection.rs` | Model selection stub | Dead |
| `cognitive_weighting.rs` | Cognitive architecture stub | Dead |
| `compliance_gatekeeper.rs` | Compliance checking stub | Dead |
| `data_ingestion.rs` | Data ingestion stub | Dead |
| `execution_enzyme.rs` | Execution enzyme stub | Dead |
| `execution.rs` | Generic execution stub | Dead |
| `self_correction_enzyme.rs` | Self-correction stub | Dead |
| `simulation_testbed.rs` | Simulation framework stub | Dead |

**Total:** 13+ dead modules requiring removal or linking.

---

## 2. Simulated Stubs Requiring Implementation

The following files contain simulated/dummy implementations marked with comments (`// Simulated`, `// Dummy`, `// In production`) that need real implementations:

| File | Current Implementation | Required Replacement |
|------|----------------------|---------------------|
| `retina_module.rs` | sin() dummy vectors | candle ViT/CLIP embeddings |
| `delta_orchestrator.rs` | Simulated KV cache extraction | Real KV cache from model state |
| `research_enzyme.rs` | Hardcoded response strings | Actual research data pipeline |
| `system_metrics.rs` | Simulated CPU thermal checks | NVML/ETW real telemetry |
| `burn_gpu.rs` | CPU loops pretending to be GPU compute | DirectML/nvapi GPU kernels |
| `silicon_backend.rs` | Fake NPU/DirectML acceleration | Real hardware acceleration stack |

**Impact:** These stubs cause false positives in benchmarks, mislead performance monitoring, and prevent proper feature gating.

---

## 3. Duplicate Code Across Crates

The following code patterns exist in multiple crate locations, indicating duplication:

| Pattern | Locations | Action Required |
|---------|-----------|-----------------|
| `z3_prover.rs` | Multiple crates | Consolidate to governance crate |
| `compaction_engine.rs` | Multiple crates | Centralize in orchestrator crate |
| `workspace.rs` | Multiple crates | Unify paths crate or remove duplicates |
| `protocol_bridge.rs` | Multiple crates | Single source in platform_bridge |
| `ast_parser.rs` | Multiple crates | Consolidate in transpiler crate |
| `dev_tools.rs` | Multiple crates | Create dedicated dev-tools crate |

---

## 4. Fabrication Directory Issue

Location: `data/fabrication/`  
Contains: 153+ independent crate prototypes with their own `Cargo.lock` files  
Status: **NOT PART OF WORKSPACE** — these are experimental prototypes, not production code.

---

## 5. `.unwrap()` Anti-Pattern Audit

**Count:** 720+ `.unwrap()` calls in production `src/` files  
**Risk:** Unnecessary panics in release builds, poor error propagation  
**Required Action:** Migrate all to `anyhow::Result` with proper error contexts  

---

## Impact Assessment

### Critical
- Simulated stubs causing incorrect performance metrics
- `.unwrap()` calls risking runtime panics in production

### High
- 13+ dead modules cluttering module tree
- Duplicate code increasing maintenance burden

### Medium
- Fabrication directory confusion (needs documentation)

---

## Recommended Action Plan

### Phase 1: Safety & Stability (Week 1-2)
1. Audit all `.unwrap()` calls, create migration plan
2. Remove dead code files from src/ tree
3. Document fabrication directory as experimental

### Phase 2: Implementation (Week 3-6)  
4. Replace simulated stubs with real implementations:
   - retina_module → candle ViT integration
   - system_metrics → NVML bindings
   - burn_gpu → DirectML/nvapi kernels

### Phase 3: Consolidation (Week 7-8)
5. Remove duplicate code, establish single sources of truth
6. Update all import paths and dependencies

### Phase 4: Verification (Week 9-10)
7. Run `cargo check --workspace` to verify compilation
8. Execute benchmark suite to validate performance claims
9. Document API changes in changelog

---

## Files Created by This Audit

- `.audit/DEAD_CODE_ANALYSIS.md` ← This file
- `.audit/SIMULATED_STUBS_REPORT.md` (to be created)
- `.audit/DUPLICATE_CODE_ANALYSIS.md` (to be created)

---

*Generated automatically by opencode audit pipeline*
