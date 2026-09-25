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
- [x] (completed: Claude session, 2026-09-18) Substrate Hardening: Audited and hardened `crates/wire` and `crates/ipc_bus`. `wire` was missing `#![deny(unsafe_code)]` (AGENTS.md requires it for standard domain crates; had zero unsafe usage, now enforced). `ipc_bus` had 14 undocumented unsafe blocks across 4 files and no crate-level unsafe policy at all - added `// SAFETY:` comments to all of them (2 more were eliminated entirely by switching `MachinePacket::as_bytes`/`from_bytes` to the crate's own already-established safe `bytemuck` idiom). Found and fixed a real bug while writing those comments: `intent_log.rs`'s `LOG_ENTRY_HEADER_SIZE` was hand-written as `48`, four bytes short of the real 56-byte `align(8)`-padded struct size, silently dropping every persisted entry's `generation` field to zero on both write and read since day one - fixed to compute from `size_of`, added a struct-layout regression test and an end-to-end round-trip test. Also wired `ast_auditor`'s `SafetyCommentVisitor` into the actual audit pipeline (it existed but was never invoked or reported anywhere) - surfaced 108 pre-existing undocumented-unsafe violations workspace-wide, none in wire/ipc_bus after this fix, 90 remaining elsewhere (core/hypervisor, compute, orchestrator, platform_bridge, si_ir, studio_hud, dev/emulator_harness). Left `has_failures()` excluding it for now, mirroring `deny.toml`'s own staged-tightening precedent - **follow-up: flip `safety_comment_violations` into `has_failures()` once those 90 are cleared.** Hot-path `.unwrap()`/`.expect()` audit found nothing new to fix (wire: test-only; ipc_bus: `create_writer()`'s `.expect()` is test-only, `get_ptr_sync()`'s only production caller is `ChaosMonkey` fault-injection tooling, `rkyv::Infallible` deserialize is safe by construction) - reconfirms Wave 10's prior finding. `#![warn(unsafe_code)]` was deliberately **not** added to `ipc_bus` despite AGENTS.md naming it for "performance/kernel" crates: this repo's `-D warnings` gate promotes `warn`-level lints to hard errors, making it functionally identical to `#![deny(unsafe_code)]` and unusable for a crate with legitimate unsafe needs - documented in a comment at the top of `ipc_bus/src/lib.rs`. See commit history on `chore/wire-ipc_bus-hardening`.

- [x] ~~Universal Compliance & Portable Deployment: Treat review-aaroneous.md as the authoritative backlog~~ — review-aaroneous.md was pruned in wave 0/1 (55b312c) before its P0 items were captured elsewhere; superseded by the compliance_auditor crate below, which replaces it as the durable review mechanism.

- [x] (completed: Claude session, 2026-09-18) Safety-comment debt cleared: annotated all 70 remaining undocumented `unsafe` blocks (core/hypervisor: hid_driver, win32_intercept, native_ingestion, supervisory_loop, cellular_automata, federation/forge, federation/profiles, screen_capture, signal_bridge, state/ring_buffer, substrate, wgpu_reflex_pipeline, bin/hypervisor.rs; crates/compute: cranelift_jit, isolated_desktop, si_jit, si_macro, si_packer, si_solid_state, si_spec, si_ssm, si_tool, wx_memory) with real, code-grounded `// SAFETY:` rationale, verified `cargo check --workspace --all-targets` and `cargo test -p ast_auditor -p hypervisor -p compute` all green. Flipped `UnifiedAuditReport::has_failures()` in crates/ast_auditor/src/lib.rs to include `safety_comment_violations`, closing out the follow-up from the 2026-09-18 substrate-hardening entry above. `cargo run -p ast_auditor` now reports 0 violations workspace-wide and the gate hard-fails on any future undocumented unsafe block.

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

- [x] (completed: Claude session, 2026-09-18) `hypervisor inject` CLI hardening, part 2: the concurrent-write race flagged (but left undone) alongside the offset fix above is now actually closed. `LegacySharedMemorySynapse::write`/`read` in core/hypervisor/src/supervisory_loop.rs take an exclusive/shared advisory lock (`std::fs::File::lock`/`lock_shared`/`unlock`, stable since Rust 1.89 - bumped workspace `rust-version` from 1.85 to 1.89 for this rather than pull in the `fs4` polyfill crate, confirmed via `unused import` warning that std's inherent methods already satisfied every call site) on the synapse's backing file for the duration of the mmap byte-copy, and the `inject` CLI command takes the same exclusive lock around its own direct mmap writes - so the daemon's tick-loop `write_state` and a separately-invoked `hypervisor inject` process (two different processes, two different open-file-descriptions) can no longer interleave their `copy_nonoverlapping` calls into the same region. Added a regression test, `test_synapse_concurrent_write_read_never_tears`, that opens *separate* `LegacySharedMemorySynapse` instances per thread (a single shared instance/fd wouldn't exercise `flock`'s per-open-file-description exclusion at all) and asserts every read is internally consistent; verified it actually catches tearing by temporarily stripping the lock calls and confirming the test fails reliably (3/3 runs), then restored the fix. `cargo test -p hypervisor` (1142 passed), `cargo check --workspace --all-targets`, and `ast_auditor` (0 violations) all green.

- [x] (completed: Claude session, 2026-09-18) `hypervisor inject` CLI hardening, part 3: Codex review on part 2 above (PR #47) flagged that the blocking `flock`/`LockFileEx` lock in `LegacySharedMemorySynapse::write`/`read` could freeze the daemon's tick loop indefinitely if a writer (e.g. `inject`) stalled while holding it - the tick watchdog can't recover mid-tick, and AGENTS.md bans blocking sync primitives on hot paths. Replaced with a non-blocking seqlock: an 8-byte sequence word prepended to the synapse mmap (even = stable, odd = writer active); `write` CAS-spins to claim it, `read` retries until it observes a stable sequence, neither ever calling a blocking OS primitive - a stalled writer bounds either at `SYNAPSE_SEQLOCK_MAX_SPINS` before erroring out (absorbed by `read_state`'s existing zero-state fallback) rather than hanging. Added `write_intent()` (writes `intent_vector_id`+`intent_payload` as one seqlock transaction) and `new_at`/`open_existing_at` explicit-path constructors - the latter because Codex's second finding was that the part-2 concurrency test resolved its path through the ambient `WorkspacePathsConfig` instead of a tempdir, violating AGENTS.md's test-sandboxing rule; the test now uses `tempfile::tempdir()`. Also reverted a workspace `rust-version` 1.85->1.89 bump from part 2 (needed only for the now-removed `File::lock`) after it turned out to have an unrelated side effect: it silently unlocked new MSRV-gated clippy lints (`collapsible_if`'s let-chain form, `chunks_exact_to_as_chunks`) across every crate inheriting the workspace default, failing CI on pre-existing code in `core-contracts` and `ast_auditor`'s own rule files that this PR never touched.

  Separately, CI then caught a real gap in part-1's "0 violations workspace-wide" claim: `ast_auditor`'s no-args invocation only scans a 4-crate `default_targets()` list (`core/hypervisor`, `crates/compute`, `crates/orchestration_plane`, `crates/llm_gateway`), not the full `core/ crates/ dev/emulator_harness/` scope CI's `agent_check.sh` actually gates on - so 20 more pre-existing unsafe blocks outside that default list (`crates/orchestrator/src/tier_allocator.rs`, `crates/platform_bridge/src/{native_win32.rs,window_target.rs,observability/rdtsc.rs}`, `crates/si_ir/src/lib.rs`, `crates/studio_hud/src/{state.rs,summon.rs}`, `dev/emulator_harness/tests/zero_allocation_allocator.rs`) were never actually checked or fixed by part 1, despite flipping `has_failures()` to hard-fail on them. Annotated all 20 with real SAFETY comments matching the established standard. `cargo run -p ast_auditor -- audit core/ crates/ dev/emulator_harness/` (CI's exact invocation) now shows 0 violations across 739 files; `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, and `cargo test` across all touched crates all green.

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

## Backlog — Further Capability-Catalog Growth

`workspace.health_audit` (PR #44) proved the pattern: a pure-Rust, no-subprocess
diagnostic `UniversalTool` that immediately finds real, pre-existing bugs just
by running it. Same shape, new targets — each is a new tool in
`crates/capabilities/src/tools.rs` (or a new file if it gets large), registered
in `build_standard_tool_registry()`, with its own regression test run against
the real workspace like `the_real_aaroneous_workspace_has_no_orphaned_crate_dirs`:

- [x] (completed: Claude session, 2026-09-24/25) `workspace.duplicate_test_names`: scan `#[test]`/`#[tokio::test]` function names across the workspace and flag exact-name collisions across different files/modules — easy to introduce by copy-paste, easy for `cargo test <name>` to silently run the wrong one. Found 78 collisions on first run. Fixed two real clusters: (1) `core/hypervisor/src/core_integration_tests.rs` was a stale, un-deleted pre-split merge of `phase1_integration_tests.rs`+`phase2_integration_tests.rs` — deleted (22 collisions). (2) `crates/mcp_server` vs. `hypervisor::mcp_service` (18 collisions) — resolved by *extracting* `mcp_service` out of `core/hypervisor` and into `crates/mcp_server` (the architecturally correct direction per AGENTS.md's own topology and this crate's already-aspirational architecture docs), not by deleting the stub: introduced an `IntentBackend` trait so `mcp_service` never names `Federation` (required — Cargo forbids the circular dependency deleting-vs-keeping either copy would otherwise force), moved the real code, wired `hypervisor` to depend on `mcp_server`. 78 → 56 → 39 collisions; full writeup and remaining (intentionally unactioned) collisions in `docs/TECH_DEBT_TEST_DUPLICATION.md`.
- [ ] `workspace.stale_todo_sweep`: grep-equivalent AST scan for `TODO`/`FIXME`/`XXX` comments, cross-referenced against `git blame` age, surfacing ones older than a configurable threshold (e.g. 90 days) as likely-abandoned.
- [ ] `workspace.unused_pub_api`: cross-reference `pub fn`/`pub struct` declarations against actual in-workspace call sites (excluding `#[cfg(test)]`) to find dead public API surface a normal `cargo check` can't catch (since `pub` items are never "unused" from a single crate's own perspective).
- [ ] `workspace.unsafe_without_safety_comment`: redundant with `ast_auditor`'s `SafetyCommentVisitor` (built the same week, independently, and since gated into `has_failures()` — see `chore/wire-ipc_bus-hardening` and the safety-comment-debt-cleared entry above) — **don't build this one**, wire the capability tool to shell out to `ast_auditor` instead if this is ever needed at the MCP/LLM tool layer.

## Backlog — Standing Items

- [x] (completed: Claude session, 2026-09-19) **Branch cleanup, PR resolution**: closed the one remaining open PR (#16, stale grouped dependabot PR predating the current `dependabot.yml`), merged #21/#22/#36 (candle-core, prettyplease, bincode — the last two needed real fixes, not just approval; see PR descriptions), and identified all 27 stale branches for deletion (21 merged, 1 closed-stale, 4 orphaned pre-history-rewrite relics with no common ancestor with `main`, 1 superseded). Branch *deletion* itself remains blocked: `git push --delete` gets a 403 from the git proxy (this session's token has push but not ref-delete scope) and no GitHub MCP tool exposes ref deletion — needs manual cleanup from GitHub's branch list or `gh api -X DELETE repos/{owner}/{repo}/git/refs/heads/{branch}` from a session with that scope.
- [ ] **RFC-0006 real implementation**: the proof-of-concept (`dev/rfc0006_poc/`, merged) proved the command-buffer plugin ABI design satisfies all five Section 8 acceptance criteria. `crates/api` is still a 9-line empty shell — the actual `studio_hud` plugin-loading integration described in `docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md` doesn't exist yet. This is the next real frontier, not yet broken down into sub-tasks — worth a dedicated planning pass (not a single-session claim) before splitting into claimable items here.
