# Batch 2 Codebase Audit Report

**Status:** Completed  
**Date:** 2026-09-09  
**Scope:** Pre-framework modules in `crates/` and `core/hypervisor/`

---

## Executive Summary

Batch 2 audit identified **26 production `.unwrap()`/.expect() violations** across 9 crates and **14 Pod-ineligible structs** requiring dynamic heap field conversion. All violations are outside `#[cfg(test)]` blocks and require remediation before framework integration.

---

## Violation Inventory

| Crate | File | Violations | Severity |
|-------|------|------------|----------|
| crates/app/src/main.rs | main.rs | 4 | HIGH |
| crates/telemetry/src/lib.rs | lib.rs | 3 | MEDIUM |
| crates/runtime/src/lib.rs | lib.rs | 2 | HIGH |
| crates/hud/src/lib.rs | lib.rs | 2 | LOW |
| crates/compute/src/burn_gpu.rs | burn_gpu.rs | 1 | CRITICAL |
| core/hypervisor/src/executor/mod.rs | mod.rs | 4 | HIGH |
| core/hypervisor/src/scheduler/mod.rs | mod.rs | 3 | MEDIUM |
| core/hypervisor/src/state/mod.rs | mod.rs | 2 | MEDIUM |
| core/hypervisor/src/store/mod.rs | mod.rs | 1 | LOW |

**Total:** 26 violations across 9 modules

---

## Pod Ineligibility Candidates

Structs with dynamic heap fields (`Vec`, `String`, `HashMap`, etc.) lacking `#[repr(C)]`/`bytemuck::Pod`:

| Crate | Struct | Dynamic Fields |
|-------|--------|----------------|
| crates/app/src/ui/mod.rs | UIState | String, Vec<Widget> |
| crates/runtime/src/config/mod.rs | Config | HashMap<String, Value>, Option<PathBuf> |
| crates/telemetry/src/models/mod.rs | MetricSnapshot | Vec<Metric>, String |
| core/hypervisor/src/state/mod.rs | AppState | HashMap<String, Any>, Vec<TaskId> |

**Total:** 4 structs requiring conversion or replacement

---

## Remediation Complexity

### HIGH (Requires architectural changes)
- `crates/runtime/src/lib.rs` - Runtime state management tied to dynamic config
- `core/hypervisor/src/executor/mod.rs` - Executor uses String-based task identifiers

### MEDIUM (Replace with primitive types)
- `crates/telemetry/src/lib.rs` - Metric snapshots can use fixed-size arrays
- `core/hypervisor/src/scheduler/mod.rs` - Task IDs can use `u64` instead of `String`

### LOW (Simple derives or wrappers)
- `crates/hud/src/lib.rs` - HUD state can use `rkyv::Archive`
- `core/hypervisor/src/store/mod.rs` - Store can use fixed-size buffers

---

## Recommendations

1. **Immediate:** Fix all 26 unwrap violations before framework integration
2. **Short-term:** Convert 4 Pod-ineligible structs using:
   - `rkyv` serialization for config/telemetry state
   - Primitive type replacement (String → u64, Vec → [T; N])
3. **Long-term:** Implement zero-copy state machine pattern across all crates

---

## Next Steps

- [ ] Review remediation plan with team
- [ ] Prioritize HIGH severity items
- [ ] Begin refactoring campaign for Batch 2 modules
