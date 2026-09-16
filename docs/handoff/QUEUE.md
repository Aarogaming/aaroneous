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
- [ ] (claiming: Claude session, 2026-09-16) crates/ipc_bus/src/swmr_shm.rs: fix 3 confirmed bugs found by compliance_auditor's verified review — (1) the shared-memory busy-bit has no crash-recovery path (a killed writer permanently wedges the segment), (2) `open_or_create` takes the same busy claim as `publish()` with no retry, so it can spuriously fail under normal concurrent use, (3) an incompatible/old-version segment causes `open_or_create` to error and the call site swallows it via `.ok()` with zero logging, silently disabling the shm bridge. **Do not independently touch swmr_shm.rs's busy-claim/open_or_create logic while this is unchecked** — coordinate here first if your work also needs to change it (e.g. a `.unwrap()`-removal pass should land after or alongside this, not separately).
- [x] (completed: Antigravity session, 2026-09-16) .github/workflows/ingestion.yml: pruned broken workflow referencing legacy sdk/python and scripts/ingest.ps1.
- [x] (completed: Antigravity session, 2026-09-16) Wave 9: zero-warning remediation (deprecated `into_path()` calls in orchestrator replaced with TempDir RAII, unused vars/imports fixed in compute/hypervisor/platform_bridge/orchestrator, zero-warning workspace target achieved across all modified crates).
- [ ] (open, unclaimed — Antigravity holding until Claude completes swmr_shm.rs) Wave 10: hot-path `.unwrap()` audit in ipc_bus, **excluding swmr_shm.rs's busy-claim/open_or_create paths** (claimed above) — machine_packet.rs and any other ipc_bus hot-path unwraps are open.
- [x] (completed: Antigravity session, 2026-09-16) Expand aaroneous-devtools migration: sdk/python/, data/extensions/python/ and win32_intercept/, standalone PowerShell deployment utilities (install.ps1, package_release.ps1, uninstall.ps1). Cloned to d:\aaroneous-devtools, committed, and pushed to Aarogaming/aaroneous-devtools (commit b7f403b).
- [ ] (open, unclaimed — Antigravity track) Wave 11: Systems architecture & deep terminology cleanup (rename `crates/autonomic_adaptation` → `crates/adaptation_plane`, systems refactor of `auto_wrapper.rs`, excise legacy compatibility aliases in `hypervisor` and `ipc_bus`).
- [ ] (open, unclaimed) Wave 13: Full verification gate protocol (`cargo check --all-targets`, `cargo test`, `ast_auditor`, zero-stub grep, `emulator_harness`, `cargo xtask gate`) and push `codex/repository-hardening`.
