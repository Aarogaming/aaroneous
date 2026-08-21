# 04: Duplicate & Conflicting Implementation Analysis

This document provides a forensic breakdown of the duplicated, fragmented, and conflicting implementations found across the repository. It details why each implementation was created, what it actually does, and the definitive path to reconcile them.

---

## 🎭 The Marionette Implementations

Marionette was envisioned as a **frontend user emulation system with backend probing and datalogging**. Over multiple iterations, at least 5 competing implementations emerged:

```
                          ┌─────────────────────────────────────────┐
                          │         Marionette (True Vision)        │
                          │   Frontend Emulation + Backend Probing  │
                          └────────────────────┬────────────────────┘
                                               │
       ┌────────────────────────┬──────────────┴──────────┬────────────────────────┐
       │                        │                         │                        │
┌──────▼──────────┐   ┌─────────▼─────────┐     ┌─────────▼─────────┐    ┌─────────▼─────────┐
│ agents/         │   │ components/       │     │ components/       │    │ core/hypervisor/  │
│ marionette_host │   │ marionette_host   │     │ chimera_          │    │ win32_intercept/  │
│                 │   │                   │     │ marionette_loop   │    │ hid_bridge.rs     │
│ Enigo/Scrap     │   │ Mocked Enigo      │     │ Async Trait       │    │ Win32 SendInput   │
│ Live unit test  │   │ with TODOs        │     │ Enigo/Scrap Loop  │    │ 30-60 FPS Hijack  │
└─────────────────┘   └───────────────────┘     └───────────────────┘    └───────────────────┘
```

### Detailed Matrix of Marionette Implementations:

| Location | Mechanism | Code Reality | Verdict & Action |
| :--- | :--- | :--- | :--- |
| `agents/marionette_host` | `enigo` + `scrap` | Unit tests execute live mouse moves (`host.pull_string_mouse(100, 200)`). | **Deprecate / Merge**. Unsafe test structure. |
| `components/marionette_host` | `enigo` + `scrap` | Has `// TODO: Fix enigo API usage` and returns `Ok("Mouse moved (mocked)")`. | **Deprecate / Merge**. Incomplete duplicate. |
| `components/chimera_marionette_loop/src/marionette.rs` | `async trait MarionetteHost` | Clean async interface with `VisualObservation` and `HidCommand` structs. | **Adopt as Core Trait Definition**. |
| `core/hypervisor/src/win32_intercept/hid_bridge.rs` | Win32 `SendInput` API | Low-level C-FFI direct Win32 hardware event dispatcher. | **Move to Marionette Native Backend** (behind strict safety switch). |
| `core/hypervisor/src/wasm_ebus_bridge/marionette_bridge.rs` | WASM WIT interface | Virtualized WASM interface binding. | **Phase Out with WASM Removal**. |

### Reconciliation Plan for Marionette:
1. Establish a single standalone crate/program: **`crates/marionette`**.
2. Adopt the clean `MarionetteHost` async trait from `components/chimera_marionette_loop/src/marionette.rs`.
3. Provide two swappable backend implementations:
   - `MockMarionette`: Returns synthetic screen frames and records simulated mouse/keyboard events into a datalog buffer (default for CI/CD, testing, and dev).
   - `NativeWin32Marionette`: Uses `win32_intercept/capture.rs` and `hid_bridge.rs` only when explicitly activated with `--enable-live-emulation`.
4. Delete the duplicated folders in `agents/marionette_host` and `components/marionette_host`.

---

## 🧬 The Chimera Implementations

Chimera was envisioned as a **"smart" software adaptation system** capable of decompiling, reading, writing, copying, and patching target software. Four conflicting implementations were created:

| Location | Claimed Role | Code Reality | Verdict & Action |
| :--- | :--- | :--- | :--- |
| `core/chimera_vm` | Universal `#![no_std]` VM | Reads 16-byte raw chunks and maps byte prefixes to fake C-IR opcodes (`0x71-0x75`). | **Archive / Refactor**. Conceptually interesting byte-slicer, but incomplete. |
| `components/chimera_marionette_loop/src/chimera.rs` | AST synthesis engine | Tree-sitter query finding `panic!` macros and generating commented patch strings. | **Adopt as AST Patching Core**. |
| `core/hypervisor/src/lib.rs` (`run_health_checks`) | "DNA Bank (Chimera)" | Calls `persistence::PersistenceManager::new(":memory:")` (SQLite trait storage). | **Rename to SQLite DNA Bank** to remove namespace confusion. |
| `MaelstromUI` & REST API (`server.rs`) | Macro recorder | Endpoints `/chimera/record` and `/chimera/routines` for UI event recording. | **Rebrand to Routine Datalogger** and move under Marionette. |

### Reconciliation Plan for Chimera:
1. Establish a single standalone crate/program: **`crates/chimera`**.
2. Consolidate AST parsing and patch synthesis (using Tree-Sitter) as the primary software adaptation engine.
3. Integrate binary disassembly and bytecode inspection as Chimera's low-level analysis module.
4. Move macro recording and peripheral playback entirely to Marionette where it logically belongs.

---

## 🧠 The Synapse Implementations

| Location | Implementation Type | Reality |
| :--- | :--- | :--- |
| `core/hypervisor/src/autonomic_loop.rs` | `LegacySharedMemorySynapse` | Memory-mapped file in `AppData\Local\Temp\primary.synapse` using raw pointer copies. |
| `core/nervous_system/src/lib.rs` | `Synapse` struct | Typed struct definitions for memory pressure and state signals. |
| `config/constellation_nats_topics.json` | NATS pub/sub topics | Subject routing over NATS message broker. |

### Reconciliation Plan for Synapse:
- Unify Synapse into the **Machine-Native Linking Protocol**: a single, well-documented shared memory ring buffer backed by a clean Rust wrapper with endianness and alignment guarantees, accompanied by NATS subject mapping.
