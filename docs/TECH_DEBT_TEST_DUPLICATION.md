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

### 2.2 `crates/mcp_server` — resolved by extraction, not deletion

**The finding** (as originally recorded here): `crates/mcp_server` was a
workspace member with no `[[bin]]` target and zero in-workspace references,
duplicating a much-earlier, smaller snapshot of `core/hypervisor`'s
`mcp_service` module (`service.rs`: 343 lines there vs. 1612 in hypervisor;
`capability.rs`/`config.rs`/`transport.rs` byte-identical stale copies) —
see git history for the full original writeup, kept here in §2.2 only in
outline since the resolution below supersedes it.

**Why deletion was the wrong fix.** The live `mcp_service` code that grew up
inside `core/hypervisor` was never supposed to live there long-term: MCP is
an HTTP/SSE/JSON-RPC protocol adapter — a presentation-layer concern per
AGENTS.md's own component topology (`core/hypervisor`: "Headless microkernel
host & execution loop" vs. `crates/api`/`crates/studio_hud`: "Presentation
layer") — and `crates/mcp_server`'s existence (and the architecture docs
already describing it as the real Ring-3 MCP crate) show that separation was
the intended design all along, just never finished. Deleting the stale
`crates/mcp_server` stub would have permanently fused MCP into the kernel
crate instead of finishing the separation it was clearly meant to have.

**What was actually done instead**, across two commits:

1. **Encapsulation** (prerequisite): `mcp_service::McpService` used to reach
   directly into `Federation`'s fields (`dynamic`, `results`, `biology`) and
   specialist internals in ~15 places. Introduced `mcp_service::backend`'s
   `IntentBackend` trait — everything MCP needs from "the running hive,"
   expressed in Federation-agnostic types — and `impl IntentBackend for
   Federation` in `core/hypervisor/src/federation/cluster/mcp_backend.rs`.
   Verified behavior-preserving: `cargo test -p hypervisor --lib` unchanged
   at 1117 tests before the move.
2. **Extraction**: moved `mcp_service/` (now depending only on the
   `IntentBackend` trait, never on `Federation` by name — a hard Cargo
   requirement, not a style choice: `crates/mcp_server` depending on
   `hypervisor`'s lib while `hypervisor`'s own binary depends on
   `mcp_server` would be a circular package dependency, which Cargo rejects
   regardless of which target uses which) into `crates/mcp_server`, deleted
   the stale duplicate content that was there before (`action_executor.rs`,
   `capability_broker.rs`, `decision_engine.rs`, `micro_vm.rs`,
   `mcp_bridge/` — all unrelated, unreferenced early stubs), added
   `mcp_server` as a dependency of `hypervisor`'s `Cargo.toml`, and
   repointed `core/hypervisor/bin/hypervisor.rs`'s `run_mcp_pipeline` at it.
   `core/hypervisor/src/mcp_service/` no longer exists — there is exactly
   one copy of this code now, and it lives where the architecture docs
   already said it should.

**Result**: the five root-level docs (`MASTER_ROADMAP.md`,
`STRATEGIC_VISION.md`, `WHAT_EXISTS_TODAY.md`,
`docs/architecture/MASTER_ARCHITECTURE.md`,
`docs/architecture/architecture_overview.md`) that describe `crates/mcp_server`
as the live Ring-3 MCP crate "wired directly to `capabilities::ToolRegistry`"
needed **no correction** — they were aspirationally accurate and are now
simply accurate. `cargo test -p hypervisor --lib` (1100 tests, exactly
1117 − 17 for the `mcp_service` unit tests that moved with the code),
`cargo test -p mcp_server` (17 passed + 1 doctest), `cargo check --workspace
--all-targets`, `cargo fmt`, and `cargo run -p ast_auditor -- audit
core/hypervisor/ crates/mcp_server/` (0 violations, 287 files) all confirm
this. Resolved 17 of the original 18 `mcp_server`/hypervisor collisions (the
18th, `test_default_config`, turned out to be a coincidental match against
an unrelated third file, `core/hypervisor/src/config/predictive_models_config.rs`
— real, but a §3-class footgun, not this section's architectural
duplication).

**Side finding, not acted on**: `run_mcp_pipeline` (the `hypervisor mcp`
CLI subcommand) has never actually attached a live `Federation` to the
`McpService` it starts — `with_federation`/`with_backend` has zero callers
in `bin/hypervisor.rs`. Every `ask_*` tool call through that subcommand
always hits the mock-LLM fallback in production today. Pre-existing,
unrelated to this extraction (confirmed unchanged before/after), and out
of scope here — whether that's a bug or a deliberate lightweight mode is
a product decision for whoever picks it up next.

## 3. Remaining collisions (informational, not actioned)

After §2.1 and §2.2's fixes, 39 collisions remain (down from the original
78: 78 → 56 → 39). The rest fall into two buckets that are **not**
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
  but mass-renaming ~39 tests across unrelated crates for a cosmetic-only
  win is out of scope for this pass. Left as backlog if a future session
  wants to pursue AGENTS.md-style hygiene renames.

Re-run `workspace.duplicate_test_names` (via the `capabilities` crate's own
`build_standard_tool_registry()`, or `cargo test -p capabilities
the_real_aaroneous_workspace_has_a_known_duplicate_test_name`) for the current,
authoritative list — the counts above are a snapshot, not a permanently
pinned assertion.
