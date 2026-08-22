# Canonical Architecture & Crate Status Index

**Current Release:** v0.3.0  
**Workspace Structure:** 12 Sovereign Crates in Pure Rust  
**Compiler Baseline:** `cargo check --workspace` (0 Errors, 0 Warnings)  
**Test Suite:** 1,356 / 1,356 Passing (100%)

---

## 🏛️ Crate Census & Lifecycle Status

| Crate | Path | Lifecycle State | Production Boundaries | Test Coverage |
|---|---|---|---|---|
| **`a_run` / `aaroneous`** | `core/hypervisor` | 🟢 Production | Hypervisor CLI daemon, egui/wgpu Studio HUD, MCP service, Win32 HID bridge | 1,086 tests |
| **`compute`** | `crates/compute` | 🟢 Production | Solid-State `.si` containers (v2), CKA+InfoNCE distillation, Rosetta Stone | 75 tests |
| **`specialists`** | `crates/specialists` | 🟢 Production | 9 Olympian Sovereign Specialists, P2P gossip pulse, swarm capability sync | 17 tests |
| **`nervous_system`** | `crates/nervous_system` | 🟢 Production | 128-byte aligned SPMC Synapse Bus, zero CAS ring buffers, slab allocator | 17 tests |
| **`evolution`** | `crates/evolution` | 🟢 Production | 4-channel neurochemistry, LoRA weight burning, pairwise synergy mining | 24 tests |
| **`chimera`** | `crates/chimera` | 🟢 Production | The Dream Engine (Alice vs Bob self-play), AST auto-wrapping, mutation sandboxes | 27 tests |
| **`orchestrator`** | `crates/orchestrator` | 🟢 Production | MDP-based specialist task routing, swarm metabolic load balancing | 22 tests |
| **`omni`** | `crates/omni` | 🟢 Production | 3D Knowledge Galaxy Map, N-body gravitational clustering | 18 tests |
| **`transpiler`** | `crates/transpiler` | 🟢 Production | Machine-native discrete SI thought distillation, prompt serialization | 9 tests |
| **`biology`** | `crates/biology` | 🟢 Production | Cellular automata metabolic token budgeting & homeostasis | 14 tests |
| **`paths`** | `crates/paths` | 🟢 Production | Zero-hardcoded dynamic workspace & data directory resolver | 8 tests |
| **`marionette`** | `crates/marionette` | 🟢 Production | Machine-native Win32 hardware intercept & sandboxed motor intents | 41 tests |

---

## 🔒 Production vs. Experimental Boundaries

- **Supported Production Path**:
  - Desktop Studio UI (`aaroneous.exe`)
  - Sovereign Hypervisor Daemon & MCP Server (`a_run.exe` / `a_run mcp`)
  - Local Single-Node & P2P Swarm Clustering over Caduceus mesh
  - SQLite persistence (`hive.db`)
- **Reference / Cloud Design Blueprints**:
  - `deploy/helm/` and `deploy/terraform/` (Reference Kubernetes & cloud configurations)
  - `MaelstromUI/` (Historical Tauri fascia, deprecated in favor of native pure Rust Desktop Studio)
