# Test-Name Duplication & Overlap: Findings & Remediation

> **STATUS**: LIVING FINDINGS DOCUMENT
> **SOURCE**: `capabilities::tools::DuplicateTestNamesTool` (`workspace.duplicate_test_names`), added in the same change series as this document.
> **LAST UPDATED**: 2026-09-24

---

## 1. Why this exists

`cargo test <name>` matches by substring against **every** test binary in the
workspace. Two `#[test]`/`#[tokio::test]` functions with the same name in
different files/modules are therefore not merely a style nit: a CI script or
a developer targeting one specific test by name silently runs all of them at
once, and a reviewer skimming a diff has no signal that a "new" test is
actually a second copy of an existing one.

Running `workspace.duplicate_test_names` against this repository at the
commit this document was written against found **78 exact-name collisions**
across 2,370 unique test names. This document records what was found, what
was fixed, and what remains — so the next pass doesn't have to re-derive it
from scratch.

## 2. Resolved

### 2.1 `core/hypervisor/src/core_integration_tests.rs` — deleted

**The finding.** This 675-line file's own header comment read:

> `// COMPREHENSIVE CORE INTEGRATION TESTS`
> `// Consolidated from Phase I and Phase II tests`

It was a hand-merged copy of `phase1_integration_tests.rs` (252 lines) and
`phase2_integration_tests.rs` (433 lines) — clearly an earlier, monolithic
draft that predated the phase1/phase2 split, never deleted once that split
landed. All three modules were still `mod`-declared in `lib.rs` and ran on
every `cargo test`.

**Verification before deleting:**

- 22 of `core_integration_tests.rs`'s 25 test functions had an exact name
  match in `phase1_integration_tests.rs` + `phase2_integration_tests.rs`
  (`comm -23` on the sorted function-name lists confirms the other direction
  is a superset).
- Diffing test bodies (not just names) for a sample of those 22 — including
  `test_dopamine_signals_drive_learning`, `test_token_system_prevents_overload`,
  `test_high_gpu_load_triggers_backpressure`, `test_master_registry_aggregates`
  — showed byte-for-byte identical (or cosmetically-only-different, e.g. a
  changed `println!` label) bodies. Nothing was lost by removing the
  duplicate copy.
- The 3 functions unique to `core_integration_tests.rs`
  (`test_overall_system_status`, `test_phase1_completion_summary`,
  `test_phase1_core_feedback_loop_complete`) contain **zero** `assert!`/
  `assert_eq!` calls of any kind — they are `println!`-only ASCII-art status
  banners that pass unconditionally regardless of actual system behavior.
  Deleting them loses no verification coverage because they provided none.

**Action taken:** deleted `core/hypervisor/src/core_integration_tests.rs` and
its `mod core_integration_tests;` declaration in `core/hypervisor/src/lib.rs`.
`cargo test -p hypervisor --lib` still passes at 1117 tests (unchanged
coverage from `phase1`/`phase2`, minus the 3 non-asserting banners, whose
removal is the point). This alone accounted for **22 of the 78** collisions
(78 → 56).

**Related, larger, explicitly out-of-scope finding:** `phase1_integration_tests.rs`
and `phase2_integration_tests.rs` themselves *also* contain zero
`assert!`/`assert_eq!` calls — every test in both files is `println!`-only
narrative output describing what the system is supposed to do, not a
machine-checked assertion that it does it. This is a real test-quality gap
(these "integration tests" can't fail no matter what the code does), but
rewriting them to make real assertions is a substantially larger, separate
undertaking from removing an already-redundant copy, and is **not** done as
part of this pass. Flagged here so it isn't lost.

### 2.2 `crates/mcp_server` — recommended for removal, **not yet executed**

**The finding.** `crates/mcp_server` is a workspace member
(`Cargo.toml` → `[workspace] members`) that:

- Has **no `[[bin]]` target** of its own.
- Is **never referenced** anywhere in the workspace — `grep -rn "use mcp_server"`
  and `grep -rn "mcp_server::"` across every `.rs` file in the repository
  return zero hits. Nothing depends on it, nothing runs it.
- Duplicates a subset of `core/hypervisor`'s modules
  (`mcp_service/{auth,capability,config,http_api,mod,service,transport}.rs`,
  plus `action_executor.rs`, `capability_broker.rs`, `decision_engine.rs`,
  `micro_vm.rs`) at a **much earlier, smaller stage of development**:

  | File | `crates/mcp_server` | `core/hypervisor` |
  |---|---:|---:|
  | `mcp_service/service.rs` | 343 lines | 1612 lines |
  | `mcp_service/mod.rs` | 11 lines (no docs, no `transport` re-export) | 76 lines (full module docs, `TransportConfig`, `HttpServer`, `OAuth2Auth` re-exports) |
  | `action_executor.rs` | 75 lines | 585 lines |
  | `capability_broker.rs` | 207 lines | 1098 lines |
  | `decision_engine.rs` | 49 lines | 1002 lines |
  | `micro_vm.rs` | 13 lines | 485 lines |

  `mcp_service/capability.rs`, `config.rs`, and `transport.rs` are **byte-for-byte
  identical** between the two copies — further evidence `mcp_server` is a stale
  snapshot rather than an intentionally-independent implementation.
- The **actual, live** MCP server this workspace runs is
  `hypervisor::mcp_service`, instantiated directly in
  `core/hypervisor/bin/hypervisor.rs` (`McpService::new(config)`,
  `HttpServer::new(addr, mcp_cfg)`).
- Root-level docs (`MASTER_ROADMAP.md`, `STRATEGIC_VISION.md`,
  `WHAT_EXISTS_TODAY.md`, `docs/architecture/MASTER_ARCHITECTURE.md`,
  `docs/architecture/architecture_overview.md`) describe `crates/mcp_server`
  as **"Complete"**, **"fully integrated,"** and the live Ring-3 MCP server
  "wired directly to `capabilities::ToolRegistry`" — this description is
  **no longer accurate**; that role is filled by `hypervisor::mcp_service`.

This is the single largest cluster of the 78 collisions: 18 of the 56
remaining names in §3 below are `mcp_server` vs. `hypervisor::mcp_service`
pairs.

**Why this wasn't executed in this pass:** removing an entire workspace
member (directory deletion + `Cargo.toml` membership edit + correcting five
root-level architecture/roadmap documents) was blocked by this session's own
auto-mode guardrails as a "modify shared resources" action requiring
explicit human sign-off, rather than something to route around via another
tool. That's the right call for a change this size — it deletes a whole
crate and contradicts several docs that currently claim it's a completed,
load-bearing component — so it's recorded here as a **ready-to-execute,
fully-evidenced recommendation** rather than forced through.

**Recommended action**, once authorized:
1. Delete `crates/mcp_server/` entirely.
2. Remove `"crates/mcp_server"` from `Cargo.toml`'s `[workspace] members`.
3. Correct the five doc files above: either remove the `crates/mcp_server`
   references or repoint them at `hypervisor::mcp_service` as the actual
   live implementation.
4. Re-run `workspace.duplicate_test_names` / `cargo run -p ast_auditor` and
   the full workspace gate to confirm nothing depended on it after all.

## 3. Remaining collisions (informational, not actioned)

After §2.1's fix, 56 collisions remain (down from 78). Of those, **18** are
the `mcp_server`/`hypervisor` pairs in §2.2, resolved by the same follow-up
that removes the crate. The rest fall into two buckets that are **not**
recommended for action:

- **Parallel-implementation test suites that are supposed to look alike.**
  E.g. `core/hypervisor/src/federation/specialists/{archivist,omnipresent,
  phygital,symbiotic,visionary}.rs` each declare `test_capabilities` and
  `test_execute` — these are five specialists implementing the same trait,
  each with a smoke test of the same name for the same method. That's
  parallel structure, not accidental duplication; renaming them to be
  unique would only make the pattern harder to see at a glance.
- **Coincidental same names for genuinely unrelated tests** in different
  domains — e.g. `test_reset` (`native_ingestion/simd_xor_delta.rs` vs.
  `llm_gateway/rate_limiter.rs`), `test_default_config` (three unrelated
  config modules), `test_statistics` (three unrelated stats-tracking
  structs). These are a real (if minor) footgun for `cargo test <name>`,
  but mass-renaming ~35 tests across unrelated crates for a cosmetic-only
  win is out of scope for this pass. Left as backlog if a future session
  wants to pursue AGENTS.md-style hygiene renames.

Re-run `workspace.duplicate_test_names` (via the `capabilities` crate's own
`build_standard_tool_registry()`, or `cargo test -p capabilities
the_real_aaroneous_workspace_has_a_known_duplicate_test_name`) for the current,
authoritative list — the counts above are a snapshot, not a permanently
pinned assertion.
