# What Exists Today in Aaroneous

> **Provenance**: regenerated 2026-09-27 from `Cargo.toml`'s actual `[workspace] members`, each crate's own `Cargo.toml`/`lib.rs` description, and commands run against `main` at the time of writing (cited inline). The previous version of this file claimed a 183-test `cratify` suite (`audit_harness`/`ring_buffer_harness`/`saturation_harness`/`translation_harness`) that a full-tree `grep` proved has never existed under any name, plus corrupted terminology-table text and two crate names that no longer exist. See `aaroneous-devtools/governance/AUDIT_2026-09-26_DOCUMENTATION_AND_PROCESS.md` for the full finding. Nothing below is hand-typed prose about test counts without a command backing it — if a number here goes stale, that's normal drift; if it's wrong the day it's written, that's a bug in this file, so keep it that way when editing.

---

## Workspace topology

**37 workspace packages** as of this writing (`grep -c` on `Cargo.toml`'s `members` array; re-run that yourself before trusting this number if it's been more than a few days):

```
core/hypervisor              # Central headless runtime engine (a_run, profile_compiler, shm_dump)

crates/
├── ipc_bus                  # Machine-native linking protocol & zero-copy SPMC shared-memory bus
├── capabilities             # Universal capability toolset, MCP service tools, domain execution substrates
├── orchestrator             # Multi-agent federation, hive runtime, MDP task routing, control plane
├── adaptation_plane         # Continuous adaptive control engine, hyperparameter optimization, GGUF ingestion
├── adaptation_engine        # Universal software adaptation, binary deconstruction, AST mutation, code repair
├── omni                     # 3D galaxy semantic data navigation, star-node clustering, visual search
├── transpiler                # SI <-> conventional-AI inter-intelligence translation and model conversion
├── api                      # Public client API bindings (presentation profile)
├── scratchpad               # Presentation-profile scratch/prototyping crate
├── governance               # Hardware thermal governor and compute-token resource manager
├── compute                  # SSM engine, .si containers, CKA+InfoNCE distillation, Cranelift JIT, SiForge
├── si_ir                    # Machine-native SI intermediate representation, type lattices, dimensional units
├── si_format                # Shared utilities for .si container format alignment, verification, serialization
├── platform_bridge          # Frontend user emulation, visual perception, backend probing, datalogging
├── paths                    # Centralized workspace path discovery and directory resolution
├── wire                     # #![no_std]-compatible wire protocol, COBS framing, telemetry serialization
├── scan_core                # Dependency-free no_std deterministic scan kernel
├── core-contracts           # Fixed-memory contracts shared by hosts and embedded components
├── ast_auditor               # Static analysis auditor enforcing architectural/semantic invariants (Gate 1, 6)
├── cratify                  # Sovereign cratification CLI & orchestration dispatcher
├── studio_hud               # Desktop Studio & Telemetry HUD (native egui/wgpu presentation layer)
├── llm_gateway               # Decoupled LLM provider gateway, local discovery, prompt caching, MCP bridge
├── llm_gateway_types         # Sync-safe type definitions for LLM gateway configuration/model registry
├── runtime_monitor           # Runtime Monitor fast-path crate
│   └── runtime_monitor_bench # Its benchmark harness
├── mcp_server                # MCP protocol server: tool registry, capability broker, dispatch
├── orchestration_plane       # Headless orchestration daemon/service layer
├── compliance_auditor        # Multi-angle diff review with independent finder passes, adversarial verification
└── local_inference           # Rust-owned local inference core (in progress — see Codex's C68 native-dependency audit)

dev/
├── emulator_harness          # Golden dogfooding harness (gate 8)
├── chaos_injector             # Fault-injection harness
└── rfc0006_poc/{abi,host}     # RFC-0006 stable plugin ABI proof-of-concept (development evidence, not production)

sdk/rust                      # External SDK crate
xtask                         # Workspace verification gate runner (`cargo xtask gate`)
benches                       # Criterion benchmark suite
```

**Not currently workspace members** (present as source in some checkouts, but not compiled or shipped): `crates/hotload`, `crates/plugin_api` — removed from `Cargo.toml` on 2026-09-24 (`242e134a`) because they were the source of a fixed, unauthenticated dynamic-DLL-loading vulnerability. If you see these directories on disk, they are stale local leftovers, not part of the build — verify with `grep hotload Cargo.toml` before assuming otherwise.

---

## Verified boundaries

| Boundary | Physical reality, checked how |
| :--- | :--- |
| Hypervisor ↔ Presentation | UI lives in `crates/studio_hud`; `core/hypervisor`'s binaries build and run headless. |
| Hypervisor ↔ LLM/inference | Provider routing lives in `crates/llm_gateway` (36 unit tests passing as of this writing: `cargo test -p llm_gateway --lib`). |
| Native dependency provenance | `cargo xtask check-native` (Gate 3.5) passes against a checked-in 27-entry allowlist (`native-policy.toml`). RocksDB and the unused `libloading` dependency are fully removed — see Codex's `aaroneous-devtools/inventories/C68_NATIVE_DEPENDENCY_BOUNDARY_AUDIT_2026-09-25.md` for what's still native (allocator, SQLite, tokenizer, TLS crypto) and not yet isolated. |
| Structural/semantic invariants | `cargo run -p ast_auditor -- audit core/ crates/ dev/emulator_harness/` reports **752 files scanned, 0 violations** as of this writing. |
| Profile dependency direction | Tracked but not yet enforced; current baseline is a dated, cross-referenced 14-edge table in `docs/CRATIFY_SPEC.md` section 2.3. |
| Full verification gate | `cargo xtask gate` — all 11 gates plus Gate 3.5 — passes end-to-end on `main`, toolchain pinned via `rust-toolchain.toml` to match CI's `dtolnay/rust-toolchain@stable`. |

---

## Executables

1. **`studio_hud`** (`cargo run --release -p studio_hud`): native egui/wgpu desktop HUD.
2. **`hypervisor`** (`cargo run --release --bin hypervisor`, `core/hypervisor/bin/hypervisor.rs`): headless runtime, the workspace composition root.
3. **`profile_compiler`**, **`shm_dump`**, **`spatial_kinetic`**: supporting binaries in `core/hypervisor/bin/`.

---

## What this file deliberately does not claim

No per-suite test-count breakdown by module name is listed here, because that's exactly the kind of prose that rotted last time (module names that get renamed or removed silently break a hand-written breakdown, and nobody notices until someone greps for it). If you need current test counts for a specific crate, run `cargo test -p <crate>` — it takes seconds and it's always right.
