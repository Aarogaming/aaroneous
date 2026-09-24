# Aaroneous Product Roadmap & Pillar Status

**Status:** Canonical Evidence-Labeled Roadmap  
**Baseline Version:** v0.3.3 (2026-09-16)  
**Governance Source:** devtools `governance/WORKLIST.md` & `governance/COORDINATION_QUEUE.md`

---

## 1. Overview & Verification Standards

Aaroneous is a type-safe **Rust Component Framework** built for deterministic reduction, zero-allocation hot paths, injected dependencies, and explicit memory contracts.

All roadmap features must satisfy the **11 Sequential Verification Gates** (`cargo xtask gate`):
1. Text Encoding Contract (`check-encoding`)
2. Format Contract (`cargo fmt --all -- --check`)
3. Strict Clippy (`cargo clippy --workspace -- -D warnings`)
4. Full Workspace Compilation (`cargo check --workspace --all-targets`)
5. Functional Test Suite (`cargo test --workspace`)
6. AST Invariant Audit (`cargo run -p ast_auditor -- audit core/ crates/ dev/`)
7. Soundness & Zero-Stub Inspection (`git grep` check for `todo!`, `unimplemented!`, manual `unsafe impl Pod`)
8. Golden Harness Verification (`cargo test -p emulator_harness`)
9. Release Binary Build (`cargo check --release --bin hypervisor`)
10. Hypervisor Feature Flags (`llama-gguf,gpu-metrics,fleet,testing,standalone`)
11. P2P Mesh Feature (`p2p-iroh`)

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

- [x] **11-Gate CI/Local Parity (`cargo xtask gate`)**
  - **Evidence:** `xtask/src/gate.rs` enforces text encoding, clippy `-D warnings`, workspace tests, release check, ast_auditor, and feature combinations. Automated parity tests verify `gate.rs` matches `.github/workflows/ci.yml`.
- [x] **Static AST Invariant Audit (`ast_auditor`)**
  - **Evidence:** `cargo run -p ast_auditor -- audit core/ crates/ dev/` reported 0 violations across 738 files at the time; as of `6321a63` (2026-09-23) the gate scope (`core/ crates/ dev/emulator_harness/`) reports 0 violations across 756 files, with five library files exempted via `#[allow(ambient_authority)]` (see CRATIFY_SPEC section 7.1). Prohibits `todo!()`, `unimplemented!()`, manual `unsafe impl Pod`, and ambient `std::env::var` calls.
- [x] **Deterministic Temporal Synchronization (M19)**
  - **Evidence:** `crates/omni/src/matrix/sab_matrix.rs` test refactored using `std::fs::FileTimes` backdating (<5ms runtime); `core/hypervisor/src/bus_test.rs` artificial sleep removed. `governance/TEMPORAL_TEST_SYNCHRONIZATION_GUIDANCE.md` published.
- [x] **Lock-Free Zero-Copy IPC Transport (`crates/ipc_bus`)**
  - **Evidence:** `SwrnRingBuffer` SPMC/SWMR ring buffers derive `bytemuck::Pod` + `Zeroable` over memory-mapped files (`memmap2`) with explicit alignment padding. `IpcEvent` layout tested at 24 bytes, 8-byte aligned.
- [x] **Sound In-Process UI Plugin Lifecycle (C2 / M4)**
  - **Evidence:** `studio_hud::plugin_api::PluginManager` exposes safe in-process `load_cartridge(Box<dyn UiCartridge>)`. Historical fat-pointer FFI across DLL boundaries (`*mut dyn UiCartridge`) removed as unsound.
- [x] **Deprecation & Terminology Canonicalization (M7, M22, M23)**
  - **Evidence:** Deprecated 17 legacy biological/mythological type aliases and 3 path methods (`#[deprecated(since = "0.3.3")]`). Added `TypeId` equivalence unit tests. Published `docs/DEPRECATION_POLICY.md` scheduling removal at v0.4.0.
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
  - **Status:** In Progress (`crates/capabilities/src/broker.rs`, `crates/governance/src/health_governor.rs`, `core/hypervisor/src/state_publisher.rs`). Active development of signed token capability sandbox and thermodynamic backpressure controls.

### 3.3 Phased Release Schedule

```text
v0.3.3 (Current) ──► v0.4.0 (Deprecation Removal) ──► v0.5.0 (Plugin ABI RFC-0006) ──► v0.6.0 (Cranelift JIT) ──► v1.0.0 (Iroh P2P Mesh)
```

| Phase | Version | Focus | Primary Deliverables |
|---|---|---|---|
| **Phase 1: Terminology Cleanup & Hardening** | `v0.4.0` | Alias removal & dep remediation | Remove v0.3.3 deprecated type aliases per `DEPRECATION_POLICY.md`; execute `iroh` upgrade (M20). |
| **Phase 2: Stable Plugin ABI** | `v0.5.0` | Dynamic plugin hot-reloading | Implement `repr(C)` command-buffer replay engine for `studio_hud` plugins per RFC-0006. |
| **Phase 3: Compiler & Cranelift JIT** | `v0.6.0` | Native code generation | `cranelift-codegen` JIT for `MachineOpcode` computational graphs in `crates/compute`. |
| **Phase 4: Cartridge Foundry & Distillation** | `v0.7.0` | `.si` tooling & skill stacks | `SiForge` builder enhancements, GGUF tensor extraction, episodic skill DAG inspection. |
| **Phase 5: Multi-Node P2P Mesh** | `v1.0.0` | Distributed execution | Full Iroh QUIC p2p mesh, work-stealing scheduler, and P2P `.si` cartridge sync. |

### 3.4 Superseded & Retired Architectures

- **WASM / WIT Component Subsystem (Retired)**
  - *Status:* Removed in M6 per `docs/archive/05_WASM_PHASEOUT_AND_DEPRECATION_PLAN.md`. All 103 unconsumed `.wit` files deleted; native `.si` containers are the sole execution target.
- **Unsound DLL Trait Object FFI (Retired)**
  - *Status:* Removed in C2. Passing Rust `dyn UiCartridge` fat-pointers across shared library boundaries was replaced with safe, in-process trait composition.
- **Legacy Monikers (`a_run`, `a_hud`, `specialists` crate name)**
  - *Status:* Replaced by standard systems names: `hypervisor` binary CLI, `studio_hud` GUI binary, and `capabilities` domain crate.
