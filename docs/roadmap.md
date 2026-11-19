# Aaroneous Product Roadmap & Pillar Status

**Status:** Canonical Evidence-Labeled Roadmap  
**Workspace version:** `0.4.0` (`Cargo.toml`). Other version stamps in the repository are unreconciled; see the correction note below.  
**Last re-checked:** 2026-11-19 against `origin/main` `cc794cb3`, for the entries named in the correction note only. Every other entry still carries its original evidence date.  
**Governance Source:** devtools `governance/WORKLIST.md` & `governance/COORDINATION_QUEUE.md`

> **Correction note (2026-11-19).** This file had drifted from the code in four places, now corrected: (1) the baseline and Phase 1 state (alias removal and the `iroh` upgrade are done; section 3.1 and 3.3); (2) the capability broker entry M32 (dispatch has been caller-bound since C80, no longer a self-issued token; section 3.2); (3) the gate count, which this file no longer restates (section 1 and 3.1); (4) the AST audit figures (section 3.1).
> **Not re-verified in this correction:** every other evidence line, the M12 entry (`AGENTS.md` now lists `rfc0006_host` and `rfc0006_abi` crates, so "next: implementation" may be out of date), and Phases 2 to 5 of the schedule.
> **Version stamps:** this file says `0.4.0` from `Cargo.toml`. `RELEASE_NOTES.md` says v1.2.0, `CHANGELOG.md` has sections down to `[1.6.0]`, and the git tags are `v0.3.2-decoupled-cratified` and `v1.3.0-pure-logic`. Which scheme is the product version is the owner's decision; none of those files is changed here.

---

## 1. Overview & Verification Standards

Aaroneous is a type-safe **Rust Component Framework** built for deterministic reduction, zero-allocation hot paths, injected dependencies, and explicit memory contracts.

All roadmap features must pass `cargo xtask gate`. The authoritative gate list is `AGENTS.md` section 6, which `xtask/src/gate.rs` implements; this file does not restate it. An earlier copy here listed 11 gates and drifted as gates were added (3.5 to 3.8, 4a to 4f, 9.5 and others).

---

## 2. Core Architectural Pillars

```text
┌─────────────────────────────────────────────────────────────────────────────────┐
│                      AARONEOUS UNIFIED STUDIO & HUD (P5)                        │
│         (Native egui / eframe 0.34 over wgpu Presentation Layer)                │
└────────┬─────────────────────────────┬─────────────────────────────┬────────────┘
         │                             │                             │
         ▼                             ▼                             ▼
┌──────────────────┐         ┌──────────────────┐         ┌──────────────────────┐
│ PLATFORM INGRESS │         │ ADAPTIVE RUNTIME │         │ COMPUTE & SSM ENGINE │
│   & TRANSDUCERS  │         │   & SCHEDULER    │         │ (SiForge, .si format,│
│ (Win32 HID, DXGI,│         │ (Typestate Task  │         │ HiPPO SSM Recurrence,│
│ WASAPI Loopback) │         │   Assimilation)  │         │ Cranelift JIT Graph) │
└────────┬─────────┘         └────────┬─────────┘         └──────────┬───────────┘
         │                            │                              │
         └────────────────────────────┼──────────────────────────────┘
                                      ▼
         ┌───────────────────────────────────────────────────────────┐
         │             .si SOLID-STATE CARTRIDGE RUNTIME             │
         │  (Frozen Core + Streaming LoRA Matrix + Skill Stack DAGs) │
         └───────────────────────────────────────────────────────────┘
```

| Pillar | Subsystem | Primary Crates | Status Summary |
|---|---|---|---|
| **P1: Ingress & Transducers** | OS integration, Win32 HID, DXGI capture, WASAPI audio | `crates/platform_bridge` | **Implemented** — Native Win32 HID injection, DXGI screen capture (`windows-capture`), and WASAPI loopback. |
| **P2: Compute & SSM Engine** | Continuous HiPPO state-space recurrence, `.si` format, Cranelift JIT | `crates/compute`, `crates/si_format`, `crates/si_ir` | **Implemented** — Memory-mapped `.si` containers (`memmap2`), 4-layer SSM recurrence, zero-copy alignment (`align(64)`). |
| **P3: Capability & Model Host** | `UniversalTool` registry, GGUF ingestion, local LLM gateway | `crates/capabilities`, `crates/autonomic_adaptation`, `crates/llm_gateway` | **Implemented** — UniversalTool dual-face execution (MCP JSON + $\mathbb{R}^{256}$ VRAM tensors), MCP server (`hypervisor mcp`). |
| **P4: Adaptive Runtime Host** | Microkernel host, 3-phase scan loop, lock-free IPC, tier scheduler | `core/hypervisor`, `crates/ipc_bus`, `crates/orchestrator` | **Implemented** — 3-phase scan reduction ($S_{t+1} = f(S_t, I)$), lock-free SWMR ring buffers (`ipc_bus`), multi-hive P2P daemon. |
| **P5: Presentation HUD** | Desktop cockpit UI, 3D Galaxy graph, interactive telemetry | `crates/studio_hud`, `crates/api`, `crates/omni` | **Implemented** — In-process UI plugin loader (`UiCartridge`), egui/wgpu presentation layer (`aaroneous` binary). |

---

## 3. Evidence-Labeled Feature Status

### 3.1 Implemented & Verified Capabilities

- [x] **CI/Local Gate Parity (`cargo xtask gate`)**
  - **Evidence:** `xtask/src/gate.rs` enforces text encoding, clippy `-D warnings`, workspace tests, release check, ast_auditor, and feature combinations. Automated parity tests in `gate.rs` (for example `gate_commands_are_present_in_ci_workflow`) compare it with `.github/workflows/ci.yml`.
- [x] **Static AST Invariant Audit (`ast_auditor`)**
  - **Evidence:** `cargo run -p ast_auditor -- audit core/ crates/ dev/` reported 0 violations across 738 files at the time; as of `6321a63` (2026-09-23) the gate scope (`core/ crates/ dev/emulator_harness/`) reports 0 violations across 756 files, with five library files exempted via `#[allow(ambient_authority)]` (see CRATIFY_SPEC section 7.1; the exemption count was not re-checked). Re-run 2026-11-19 at `cc794cb3`: the same gate scope reports 0 violations across 752 files. Prohibits `todo!()`, `unimplemented!()`, manual `unsafe impl Pod`, and ambient `std::env::var` calls.
- [x] **Deterministic Temporal Synchronization (M19)**
  - **Evidence:** `crates/omni/src/matrix/sab_matrix.rs` test refactored using `std::fs::FileTimes` backdating (<5ms runtime); `core/hypervisor/src/bus_test.rs` artificial sleep removed. `governance/TEMPORAL_TEST_SYNCHRONIZATION_GUIDANCE.md` published.
- [x] **Lock-Free Zero-Copy IPC Transport (`crates/ipc_bus`)**
  - **Evidence:** `SwrnRingBuffer` SPMC/SWMR ring buffers derive `bytemuck::Pod` + `Zeroable` over memory-mapped files (`memmap2`) with explicit alignment padding. `IpcEvent` layout tested at 24 bytes, 8-byte aligned.
- [x] **Sound In-Process UI Plugin Lifecycle (C2 / M4)**
  - **Evidence:** `studio_hud::plugin_api::PluginManager` exposes safe in-process `load_cartridge(Box<dyn UiCartridge>)`. Historical fat-pointer FFI across DLL boundaries (`*mut dyn UiCartridge`) removed as unsound.
- [x] **Deprecation & Terminology Canonicalization (M7, M22, M23)**
  - **Evidence:** Deprecated 17 legacy biological/mythological type aliases and 3 path methods (`#[deprecated(since = "0.3.3")]`). Added `TypeId` equivalence unit tests. Published `docs/DEPRECATION_POLICY.md` scheduling removal at v0.4.0. Removal was executed in `5b9dada1` (2026-09-25, an ancestor of `main`). Re-checked 2026-11-19: a word-level grep of `crates/` and `core/` finds none of the policy's alias or method names as identifiers. `GeneticLocus` survives only in doc comments under `crates/coordinator/src/federation/` (for example `graph/analyzer.rs:24` and `profiles/mod.rs:14`), which still use the retired terminology. `docs/DEPRECATION_POLICY.md` still lists the aliases as deprecated and was not changed here.
- [x] **Root README Assurance Reconciliation (M21)**
  - **Evidence:** Reconciled root `README.md` with current binary CLI (`hypervisor` subcommands `start`, `boot`, `mesh`, `daemon`, `evolve`, `forge`, `si`, `distill-all`, `mcp`) and explicitly labeled latency figures as design targets.
- [x] **Iroh P2P Dependency Upgrade (C20 / M20)**
  - **Evidence:** Bumped `iroh` to `1.2` in `core/hypervisor/Cargo.toml` (commit `f747f69`). Unblocked transitive `hickory-net`, `atomic-polyfill`, and `lru` vulnerabilities (`cargo-audit` / `cargo-deny` green).
- [x] **Property-Oriented Parser Property Tests (M26)**
  - **Evidence:** Added `proptest` boundary fuzz test suites in `crates/wire/tests/boundary_property_tests.rs` (4 tests) and `crates/ipc_bus/tests/ipc_property_tests.rs` (2 tests). Zero-panic COBS and zero-copy POD conversions verified.
- [x] **Baseline Performance Benchmarking (M27)**
  - **Evidence:** Published `dev/emulator_harness/BENCHMARK_CONTRACT.md` and Criterion benchmark suite `reduction_benchmark.rs` measuring single-pass trace reduction throughput (<2.5µs / 1k events target).

### 3.2 Active & In-Progress Development Lanes (Phase 38 / Horizon 6)

- [ ] **M12: Stable Plugin Command-Buffer ABI (RFC-0006)**
  - **Status:** Specification complete (`docs/rfcs/RFC-0006-PLUGIN_LIFECYCLE_AND_STABLE_UI_CARTRIDGE_ABI.md`). Next: implementation of a `repr(C)` command-buffer protocol for dynamic hot-reload plugins.
- [ ] **Phase 38 / M32: Capability Broker & Resource Governance Integration**
  - **Status:** Primitives complete and unit-tested (real RFC 2104 HMAC-SHA256, process-local random signing key). Dispatch has been caller-bound since C80 (`80feaa26`, 2026-10-27; contract in `docs/architecture/MCP_SERVICE_CONTRACT.md`). In `crates/mcp_server/src/mcp_service/service.rs`, `tools/call` evaluates the authenticated `CallerContext` (revocation, expiry, capability class, sandbox policy, scope) and ignores any caller-supplied `params.caller` or `params.scope`. `issue_caller_dispatch_token` then mints a token scoped to the single requested tool, bound to the caller's principal and sandbox policy, and dispatch goes through `execute_tool_with_token`, where signature, expiry, sandbox-policy and `ThermalBackpressureLevel` checks apply (`test_tools_call_is_actually_gated_by_thermal_backpressure`). This replaces the state this entry described on 2026-09-27, a self-issued `FullPrivilege` token with subject `mcp-service`.
  - **Remaining (partly re-checked 2026-11-19):** `core/hypervisor/src/capability_broker.rs`, named here before, no longer exists. `sync_with_state_publisher` is defined in `crates/coordinator/src/capability_broker.rs` and, by grep, called only from that file's tests. `studio_hud` re-exports the coordinator broker and calls its `execute` (`crates/studio_hud/src/app.rs:180`); whether that path applies token checks was not examined. The `hypervisor` binary builds a broker only to export the capability list (`run_export_capabilities`). `crates/coordinator/src/mcp_service/` carries a byte-identical copy of `crates/mcp_server/src/mcp_service/capability.rs`; unifying the two is not part of this entry.

### 3.3 Phased Release Schedule

```text
v0.4.0 (workspace version; Phase 1 below) ──► v0.5.0 (Plugin ABI RFC-0006) ──► v0.6.0 (Cranelift JIT) ──► v1.0.0 (Iroh P2P Mesh)
```

| Phase | Version | Focus | Primary Deliverables |
|---|---|---|---|
| **Phase 1: Terminology Cleanup & Hardening** | `v0.4.0` | Alias removal & dep remediation | Remove v0.3.3 deprecated type aliases per `DEPRECATION_POLICY.md`; execute `iroh` upgrade (M20). |
| **Phase 2: Stable Plugin ABI** | `v0.5.0` | Dynamic plugin hot-reloading | Implement `repr(C)` command-buffer replay engine for `studio_hud` plugins per RFC-0006. |
| **Phase 3: Compiler & Cranelift JIT** | `v0.6.0` | Native code generation | `cranelift-codegen` JIT for `MachineOpcode` computational graphs in `crates/compute`. |
| **Phase 4: Cartridge Foundry & Distillation** | `v0.7.0` | `.si` tooling & skill stacks | `SiForge` builder enhancements, GGUF tensor extraction, episodic skill DAG inspection. |
| **Phase 5: Multi-Node P2P Mesh** | `v1.0.0` | Distributed execution | Full Iroh QUIC p2p mesh, work-stealing scheduler, and P2P `.si` cartridge sync. |

*Phase 1 status (re-checked 2026-11-19):* both deliverables are recorded as done in section 3.1 (alias removal in `5b9dada1`, with the leftover doc-comment mentions noted there; the `iroh` 1.2 upgrade, M20). Phases 2 to 5 were not re-checked.

### 3.4 Superseded & Retired Architectures

- **WASM / WIT Component Subsystem (Retired)**
  - *Status:* Removed in M6 per `docs/archive/05_WASM_PHASEOUT_AND_DEPRECATION_PLAN.md`. All 103 unconsumed `.wit` files deleted; native `.si` containers are the sole execution target.
- **Unsound DLL Trait Object FFI (Retired)**
  - *Status:* Removed in C2. Passing Rust `dyn UiCartridge` fat-pointers across shared library boundaries was replaced with safe, in-process trait composition.
- **Legacy Monikers (`a_run`, `a_hud`, `specialists` crate name)**
  - *Status:* Replaced by standard systems names: `hypervisor` binary CLI, `studio_hud` GUI binary, and `capabilities` domain crate.
