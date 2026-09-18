# Aaroneous Autonomous Task Queue

- [x] Batch 3: Migrate ring.rs, harvest.rs, fascia.rs, and workspace.rs from aaroneous/crates/cratify/src/ into crates/adaptation_engine/src/, wire into lib.rs, and verify compilation
- [x] Batch 4: Slim crates/cratify down to a thin CLI binary that delegates commands to ast_auditor, transpiler, and adaptation_engine
- [x] Cleanup: Purge migrated source files from aaroneous/crates/cratify/ and run full workspace verification (cargo check --workspace --all-targets and ast_auditor)
- [x] Platform Bridge: Reconcile and wire unlinked adapters (midi_osc.rs, ndi_broadcast.rs, hooking/injector.rs) into crates/platform_bridge/src/lib.rs with optional feature gating
- [x] Hypervisor Testbed: Wire simulation_testbed.rs, chaos_monkey.rs, and task_worker.rs into core/hypervisor/src/lib.rs under the testing/simulation feature flag
- [x] Workspace Verification: Run full workspace test suite and ast_auditor to ensure all 724+ files pass with 0 errors
- [x] Capability Miner Item #1: Zero-Copy Shared-Memory IPC Bridge for Out-of-Process Shell Isolates (ipc_bus::swmr_shm ⟶ studio_hud)
- [x] Capability Miner Item #2: SMT-Interlocked AST Code Mutation Engine (governance::smt_action_interlock ⟶ adaptation_engine::pattern_rewriter)
- [x] Capability Miner Item #3: AST-Driven Prompt Context & KV-Cache Pinner (transpiler::DemandDrivenAstCache ⟶ llm_gateway::AstPrefixCacheManager)
- [x] SHELL-03: Detached Transparent Window Pipeline (Click-Through Win32 HUD in platform_bridge & studio_hud)
- [x] SWARM-01: Multi-Hive Swarm Sub-Agent Offloading (SwarmOffloader & LiveP2PDaemon in core/hypervisor)
- [x] DIST-02: Zero-Copy Multi-Process Shared Memory Ring Buffer (64-slot SWMR seqlock double-barrier ring in ipc_bus & studio_hud)
- [x] ADAPT-01: Streaming Self-Correction & Autonomous Pacing Regulation (autonomic_adaptation::streaming_adaptation ⟶ core::hypervisor::supervisory_loop)
- [x] DIST-03: Black-Box Flight Recorder & Deterministic Event Replayer (16MB circular binary flight log in ipc_bus & supervisory_loop)

- [x] ~~Universal Compliance & Portable Deployment: Treat review-aaroneous.md as the authoritative backlog~~ — review-aaroneous.md was pruned in wave 0/1 (55b312c) before its P0 items were captured elsewhere; superseded by the compliance_auditor crate below, which replaces it as the durable review mechanism.

## Coordination — two sessions are active on this branch concurrently

Two agents are working `codex/repository-hardening` at once (confirmed via
git history and a live session exchange, 2026-09-16). To avoid stepping on
each other's in-flight edits, claim a line item here with `(claiming: <who>,
<date>)` before starting it, and check it off with the commit SHA when done.
Do not start an item another session has already claimed.

- [x] (claimed: Claude session, 2026-09-16) Aaroneous compliance auditor: native Rust multi-angle diff-review crate (crates/compliance_auditor) + CI wiring — dependabot, cargo-audit/cargo-deny, Miri (wire/core-contracts), typos+actionlint, cargo-semver-checks (core-contracts/sdk), PR-comment reporting, weekly rolling sweep. See commits c11dd4e..dc9803b.
- [x] (completed: Antigravity session, 2026-09-16) Wave 8: test sandboxing & working tree hygiene (dev_tools.rs output_dir DI, artifact_pruning tempdir isolation, router.rs cargo_state.json to temp_dir, purged models/organs, 0 untracked test leaks). Commits f7876fc, 6080426.
- [x] (completed: Claude session, 2026-09-16) crates/ipc_bus/src/swmr_shm.rs: fixed all 3 confirmed bugs (crash-recovery via a heartbeat word + version bump 3->4, open_or_create retry past transient Busy, logged the previously-silent open_or_create failure in core/hypervisor/src/state_snapshot.rs). **Released** — safe for Wave 10/11 to touch now. Note for Wave 11's "excise legacy compatibility aliases in ipc_bus": the resolved synapse filename changed `engine_state_v3` -> `engine_state_v4` in both core/hypervisor/src/state_snapshot.rs and crates/studio_hud/src/state_snapshot.rs, and the wire header grew by one word (32->40 bytes, new heartbeat word at index 4, slots now start at index 5 via the new `SNAPSHOT_HEADER_WORDS` const) — if an alias-cleanup pass touches either file, diff against commit 69e217b rather than an older base. Commit: 69e217b.
- [x] (completed: Antigravity session, 2026-09-16) .github/workflows/ingestion.yml: pruned broken workflow referencing legacy sdk/python and scripts/ingest.ps1.
- [x] (completed: Antigravity session, 2026-09-16) Wave 9: zero-warning remediation (deprecated `into_path()` calls in orchestrator replaced with TempDir RAII, unused vars/imports fixed in compute/hypervisor/platform_bridge/orchestrator, zero-warning workspace target achieved across all modified crates).
- [x] (completed: Antigravity session, 2026-09-16) Wave 10: hot-path `.unwrap()` audit in ipc_bus (verified zero unhandled unwraps on critical paths; safe rkyv deserialize handled; tests isolated). Commit: 3541980.
- [x] (completed: Antigravity session, 2026-09-16) Expand aaroneous-devtools migration: sdk/python/, data/extensions/python/ and win32_intercept/, standalone PowerShell deployment utilities (install.ps1, package_release.ps1, uninstall.ps1). Cloned to d:\aaroneous-devtools, committed, and pushed to Aarogaming/aaroneous-devtools (commit b7f403b).
- [x] (completed: Antigravity session, 2026-09-16) Wave 11: Systems architecture & deep terminology cleanup (rename `crates/autonomic_adaptation` → `crates/adaptation_plane`, systems refactor of `auto_wrapper.rs`, excise legacy compatibility aliases in `hypervisor` and `ipc_bus`). Commit: 6a80fb2.
- [x] (completed: Antigravity session, 2026-09-16) Wave 13: Full verification gate protocol (`cargo check --all-targets`, `cargo test`, `ast_auditor` 730 files with 0 violations, zero-stub grep inspection, `emulator_harness`, `cargo xtask gate` PASS).

## Local Agent Autonomous Processing — RETIRED 2026-09-18, superseded by devtools

**This daemon is retired.** `dev/tools/task_queue.json` now carries a
`"retired": true` flag and `scripts/local_agent_daemon.ps1` refuses to run
against it without an explicit override (see that script's header). Do not
queue new work here.

What happened: this session built up `local_agent_daemon.ps1` this week to
write local-model output directly into the real working tree, self-certified
by nothing more than a passing `cargo check` — no human review gate.
`Aarogaming/aaroneous-devtools` already had (and still has) a stricter,
already-operational replacement for this exact lane: a running Windows
Scheduled Task (`Aaroneous-Devtools-LocalWorker`, see its
`governance/LOCAL_WORKER_SERVICE.md`) polling `worker-jobs/*.toml` every 2
minutes, under a hard policy (`governance/LOCAL_AGENT_CONTROL_PLANE.md`) that
local models may only ever produce unverified proposals, never write or
commit directly. `governance/COORDINATION_QUEUE.md` row C14 documents the
concrete bugs a bare-`cargo-check`-as-approval pattern like this one produces
in practice (an uncorrelated compile-gate crate name, no path-traversal
check, silent no-op rollback leaving broken files in the tree, zero file
locking) on a sibling prototype that had this same shape.
Confirmed 2026-09-18 (Gemini/Antigravity, running the actual 2-minute local
cycle): "All local agent control plane work... [is] 100% isolated inside
`aaroneous-devtools`... Zero changes are made to `Aaroneous`" — the product
repo is only ever read as evidence input. So this daemon was never the
system actually in use; it's redundant, unreviewed duplicate risk with no
offsetting benefit. If local-agent work is needed against this repo in the
future, queue it as a `worker-jobs/*.toml` job in `aaroneous-devtools`
instead — that pipeline already exists, is reviewed, and is what's running.

(Historical description of the retired design, kept for context.)
`scripts/local_agent_daemon.ps1` worked through `dev/tools/task_queue.json` against
a **local** Ollama model (`qwen3.5:9b-q6`, no billable API cost), one task at a
time, retrying up to `max_retries` with the compiler error fed back into the
prompt on failure. TASK-004..013 below were queued for it on 2026-09-18 under
the mistaken assumption it was the active local-agent path; they were never
picked up (the daemon was flagged and guarded before its next run) and remain
here only as examples of well-scoped test-coverage work — port any still
worth doing into a devtools `worker-jobs/*.toml` job instead of re-enabling
this daemon.

**Its own verification is `cargo fmt` + `cargo check -p <crate>` only — it
proves the generated file compiles, not that it's correct.** No `cargo test`,
no `clippy`, no semantic review. Treat every entry `local_agent_daemon.ps1`
marks `"completed"` in `dev/tools/agent_progress_log.md` as **pending human/Claude
review**, not done-done, until a session has actually run its tests and read
the diff. Only new, self-contained files are safe to queue for it (it
overwrites `target_file` wholesale — never point it at an existing file with
real logic already in it, only genuinely new test files or new standalone
modules). If you review a completed entry, note that in this file or the
progress log (`- reviewed by <who>, <date>: <verdict>`) so the next session
doesn't re-review it.

## Backlog — Unsafe Safety-Comment Documentation Sweep

`ast_auditor`'s `SafetyCommentVisitor` (wired into the audit pipeline in
`chore/wire-ipc_bus-hardening`, PR #46) found 90 pre-existing unsafe blocks
workspace-wide with no `// SAFETY:` comment, left out of scope for that PR.
It's collected and printed by `cargo run -p ast_auditor -- audit <paths>` but
**not yet gating** (`UnifiedAuditReport::has_failures()` deliberately excludes
it — see the doc comment on that method) until this debt is cleared. Each
crate below is an independent, separately-claimable unit — same pattern PR #46
used for `ipc_bus`/`wire`: read each flagged unsafe block, understand *why*
it's actually sound (or fix it if it isn't), write the rationale as a
`// SAFETY:` comment on the line immediately before the `unsafe` token (the
checker's window is narrow — see the doc comment on
`SafetyCommentVisitor::check_safety_comment` for the exact off-by-one before
writing multi-line comments). Re-run the audit command after each crate to
confirm its count hits zero. Once all seven are clear, flip
`safety_comment_violations` into `has_failures()` as a follow-up PR.

- [ ] `core/hypervisor` (largest — `hid_driver/platform.rs`, `wgpu_reflex_pipeline.rs`, `cellular_automata.rs`, `supervisory_loop.rs`, `state/ring_buffer.rs`, `signal_bridge.rs`, `bin/hypervisor.rs`, `native_ingestion/{simd_xor_delta,shmem_capture}.rs`, `screen_capture.rs`, `substrate.rs` — ~25 sites)
- [ ] `crates/compute` (`wx_memory.rs`, `si_ssm.rs`, `cranelift_jit.rs`, `si_tool.rs`, `si_jit.rs`, `si_spec.rs`, `isolated_desktop.rs`, `si_packer.rs`, `si_solid_state.rs`, `si_macro.rs` — ~22 sites; this is the JIT/W^X memory crate, so get the rationale right, not just present)
- [ ] `crates/platform_bridge` (`observability/rdtsc.rs`, `window_target.rs`, `native_win32.rs` — ~15 sites, mostly raw Win32 FFI boundary calls)
- [ ] `crates/orchestrator` (`tier_allocator.rs` — 1 site)
- [ ] `crates/si_ir` (`lib.rs:624` — 1 site)
- [ ] `crates/studio_hud` (`summon.rs`, `state.rs` — 2 sites)
- [ ] `dev/emulator_harness` (`tests/zero_allocation_allocator.rs` — 2 sites)

## Backlog — Further Capability-Catalog Growth

`workspace.health_audit` (PR #44) proved the pattern: a pure-Rust, no-subprocess
diagnostic `UniversalTool` that immediately finds real, pre-existing bugs just
by running it. Same shape, new targets — each is a new tool in
`crates/capabilities/src/tools.rs` (or a new file if it gets large), registered
in `build_standard_tool_registry()`, with its own regression test run against
the real workspace like `the_real_aaroneous_workspace_has_no_orphaned_crate_dirs`:

- [ ] `workspace.duplicate_test_names`: scan `#[test]`/`#[tokio::test]` function names across the workspace and flag exact-name collisions across different files/modules — easy to introduce by copy-paste, easy for `cargo test <name>` to silently run the wrong one.
- [ ] `workspace.stale_todo_sweep`: grep-equivalent AST scan for `TODO`/`FIXME`/`XXX` comments, cross-referenced against `git blame` age, surfacing ones older than a configurable threshold (e.g. 90 days) as likely-abandoned.
- [ ] `workspace.unused_pub_api`: cross-reference `pub fn`/`pub struct` declarations against actual in-workspace call sites (excluding `#[cfg(test)]`) to find dead public API surface a normal `cargo check` can't catch (since `pub` items are never "unused" from a single crate's own perspective).
- [ ] `workspace.unsafe_without_safety_comment`: this is now genuinely redundant with `ast_auditor`'s `SafetyCommentVisitor` above (built the same week, independently) — **don't build this one**, wire the capability tool to shell out to `ast_auditor` instead if this is ever needed at the MCP/LLM tool layer.

## Backlog — Standing Items

- [ ] **Branch cleanup**: 13 remote branches are fully merged (confirmed via ancestry or matching each squash-merged PR's timestamp with no later pushes) and ready to delete: `cleanup/dead-plugin-loading`, `docs/master-roadmap-corruption-fix`, `fix/axum-0.8-router-syntax`, `fix/eframe-0.36-show-inside-rename`, `fix/iroh-1.x-upgrade`, `fix/rusqlite-0.40-u64-cast`, `rfc0006/plugin-abi-poc`, `codex/repository-hardening`, `wip/engine-crates`, `feature/cratify-batch2-hypervisor`, `feature/cratify-legacy-refactor`, `refactor/strip-analogies`, `feat/capabilities-workspace-health-tool`. Blocked on `git push --delete` being denied by the sandboxed session's permission classifier ("Git Destructive") with no GitHub delete-branch MCP tool available as a workaround — needs either a Bash permission grant or manual cleanup from GitHub's branch list.
- [ ] **RFC-0006 real implementation**: the proof-of-concept (`dev/rfc0006_poc/`, merged) proved the command-buffer plugin ABI design satisfies all five Section 8 acceptance criteria. `crates/api` is still a 9-line empty shell — the actual `studio_hud` plugin-loading integration described in `docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md` doesn't exist yet. This is the next real frontier, not yet broken down into sub-tasks — worth a dedicated planning pass (not a single-session claim) before splitting into claimable items here.
