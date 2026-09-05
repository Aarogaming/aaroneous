# 08: Modular Restructuring & Stabilization Roadmap

## Executive Roadmap Overview

This roadmap details the concrete, phased engineering steps required to transition Aaroneous from an un-vetted, tangled monolith into a suite of clean, decoupled, machine-native programs running under the Synthetic Intelligence architecture.

---

## 🗺️ The Five Stabilization Phases

```
Phase 1: Developer Tooling & Forensic Ledger (COMPLETED in dev/)
  ├── Establish dev/ headquarters, authoritative docs, and diagnostic scripts
  ├── Document true file census, duplicate matrix, and host safety protocols
  └── Author Machine-Native Linking Protocol (MNLP) specification

Phase 2: Host Safety Isolation & Sandboxing
   ├── Modify SpatialKineticConfig & Desktop Emulator to default enable_hid_output = false
  ├── Implement MockMarionette for zero-risk testing and development
  ├── Add AARONEOUS_ALLOW_HOST_INPUT guard for live peripheral execution
  └── Create automated test harness with mock display buffers

Phase 3: WASM Elimination & Dependency Pruning
  ├── Remove wasmtime, wasmtime-wasi, and wit-bindgen from Cargo.toml files
  ├── Convert wasm_loader and wasm_splicer to native dynamic library loaders
  ├── Delete / archive obsolete .wasm files and templates/universal_sab
  └── Clean up orphaned transpiler and python extraction scripts in scripts/

Phase 4: Modular Program Separation
  ├── Extract Marionette into standalone program (crates/marionette)
  ├── Extract Chimera into standalone program (crates/chimera)
   ├── Isolate Orchestrator (task management) & Synthesizer (intelligence) into specialist crates
  ├── Unify Synapse into the Machine-Native Linking Protocol shared memory library
  └── Update root Cargo.toml workspace members to include all active crates cleanly

Phase 5: Native Desktop Studio (`a_hud`) Stabilization
  ├── Fix dangling HTML/JSX syntax errors in MaelstromUI/src/App.tsx
  ├── Connect MaelstromUI SSE streams to the new linking protocol daemon
  ├── Test real-time DAG rendering, telemetry charts, and model import workflows
  └── Verify end-to-end multi-program linking without host disruption
```

---

## 📋 Immediate Action Items for Phase 2 & 3

1. **Safety Switch Integration**:
   - Update `core/hypervisor/src/spatial_kinetic_engine.rs` to set `enable_hid_output: false` by default.
   - Guard `HIDOutputBridge::execute_intent` behind an environment variable check (`AARONEOUS_ALLOW_HOST_INPUT == "1"`).

2. **Workspace `Cargo.toml` Unification**:
   - Add all legitimate component crates (`components/agents`, `components/biology`, `components/compute`, `components/constellation`, `components/control`, `components/digestion`, `components/genetics`, `components/hive`, `components/intelligence`, `components/paths`, `components/scientific_analyzer`, `components/skills`, `components/storage`) to the root `[workspace] members` array.

3. **Frontend Syntax Patch**:
   - Verify deletion of legacy React files.


