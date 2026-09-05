# Active High-Priority Audit Queue (Compact)
*Optimized for local model context windows (Qwen 3.5 9B)*

## ☠️ TIER 0: BLOCKER PRIORITY
- [ ] **BLOCKER-03: Dependency CVE Advisories (SEC-05)**
  - Crates: eqwest, iroh, urn
  - Action: Run cargo update and resolve semver vulnerabilities.

## 🔴 TIER 1: CRITICAL PRIORITY (Immediate Code Fixes)
- [ ] **CRIT-04: Cross-DLL Fat Pointer & Heap Corruption (SEC-03)**
  - File: core/hypervisor/src/hud/plugin_api.rs
  - Action: Replace *mut dyn UiCartridge with #[repr(C)] FFI vtable struct and host free function.
- [ ] **CRIT-05: Sandbox Canonicalization Path Traversal Bypass (SEC-06)**
  - File: core/hypervisor/src/action_executor.rs
  - Action: Canonicalize parent directory of target path rather than raw fallback.
- [ ] **CRIT-06: API Key Timing Attack Side-Channel (SEC-04)**
  - File: core/hypervisor/src/mcp_service/http_api.rs
  - Action: Replace == with subtle::ConstantTimeEq.
- [ ] **CRIT-07: Lock Inversion Deadlock in Hive Runtime**
  - File: crates/orchestrator/src/hive_runtime.rs
  - Action: Acquire 	ask_log lock before outer lock across all methods.
- [ ] **CRIT-08: Swarm Balancer TOCTOU Race Condition**
  - File: crates/orchestrator/src/swarm_balancer.rs
  - Action: Retain write lock during worker allocation to prevent double-assignment.
- [ ] **CRIT-09: Lock-Poisoning Panics (DEBT-14A)**
  - Files: core/hypervisor/src/advanced_intelligence.rs & compaction_engine.rs
  - Action: Replace 13+ .write().unwrap() and .read().unwrap() with Result bubbling.
- [ ] **CRIT-10: Genetic Algorithm NaN-Panic Sort Comparators (DEBT-14B)**
  - File: crates/autonomic_adaptation/src/genetics.rs:362, :413
  - Action: Replace .partial_cmp().unwrap() with .unwrap_or(Ordering::Equal).
- [ ] **CRIT-11: Mutual Information NaN-Panic Comparator (DEBT-14C)**
  - File: crates/omni/src/matrix/sab_tensor.rs:130
  - Action: Replace .2.partial_cmp(&a.2).unwrap() with .unwrap_or(Ordering::Equal).