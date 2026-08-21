# Hybrid Micro-VM & Universal Patch Engine Architecture

## 1. Executive Vision: The Universal Development & Patching Tool
Aaroneous shifts the software modification paradigm from centralized developer patching to decentralized, synthetic intelligence-driven local adaptation. Operating as a **Universal Patch Engine**, Aaroneous empowers developers, players, and modders to diagnose, modify, and automate software at a machine level without waiting for official developer updates.

By combining two foundational engines—**The Chimera Engine** (file synthesis, binary restructuring, and AST-level patching) and **The Marionette Host** (visual perception, UI graph mapping, and user emulation)—Aaroneous functions as an autonomous, self-correcting development and review assistant.

---

## 2. Architectural Topology: Host-Bound Rust & Isolated Micro-VM Execution
To avoid the brittleness of WASM sandbox linkers while maintaining absolute host security, Aaroneous utilizes a hybrid execution model:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Windows / Host OS                               │
│  ┌───────────────────────┐             ┌────────────────────────────┐  │
│  │   UI & Orchestration  │◄─── gRPC ──►│ Rust Orchestrator (`a_run`)│  │
│  │ (MaelstromUI / egui)  │  (Tonic)    │ - Epigenetic State Router  │  │
│  └───────────────────────┘             └─────────────┬──────────────┘  │
└──────────────────────────────────────────────────────┼─────────────────┘
                                                       │ Shared IPC / vhost-vsock
┌──────────────────────────────────────────────────────┼─────────────────┐
│                 Lightweight Micro-VM / Linux Sandbox │                 │
│  ┌───────────────────────────────────────────────────▼──────────────┐  │
│  │                     Chimera Synthesis Engine                      │  │
│  │   - Binary Deconstruction & AST Rewriting (tree-sitter / syn)     │  │
│  │   - Machine-Native Learning & LLM Inference (Candle / GGUF)       │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                     Marionette Emulation Loop                     │  │
│  │   - Visual Perception (scrap) & UI Graph Parsing                  │  │
│  │   - Trial-and-Error Exploration & Autonomous HID Injection        │  │
│  └───────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1. Host Orchestration (`a_run`)
* **Role**: Manages UI state, user intent routing, security gating, and resource allocation.
* **Technology**: Pure Rust async runtime (Tokio) communicating with internal workers via high-performance gRPC (`tonic`) or Unix domain sockets / named pipes.

### 2.2. Isolated Execution Sandbox (Micro-VM / Container)
* **Role**: Houses machine-learning inference engines, source decompilers, and the Marionette visual feedback loop.
* **Technology**: Embedded lightweight hypervisor wrapper or containerized Linux daemon providing strict resource capping (CPU/RAM/GPU) via cgroups and namespaces.

---

## 3. Core Subsystems

### 3.1. The Chimera Function (The Universal Patch Engine)
* **Capability**: Reads, writes, copies, and adapts any file structure, script, or binary asset.
* **Operation**:
  1. **Ingestion**: Scans target binaries or source trees using `tree-sitter` and AST parsers.
  2. **Diagnosis**: Local GGUF models analyze machine-level execution failures, stack traces, or inefficient logic paths.
  3. **Synthesis**: Generates surgical patches (source rewrites or binary modifications).
  4. **Verification**: Compiles and runs automated test suites to validate patch correctness before presenting a publishable fix.

### 3.2. The Marionette Function (User & Player Emulation)
* **Capability**: Operates target programs or games like a human expert, conducting rigorous quality assurance, exploration, and automated regression testing.
* **Operation**:
  1. **Perception**: Ingests sub-millisecond screen buffers (`scrap`) to construct a dynamic `UIGraph`.
  2. **Exploration**: Executes trial-and-error interaction strategies via virtualized HID input (`enigo`).
  3. **Experience Storage**: Encodes successful interaction sequences into persistent memory vectors (`federation_memory.json`), enabling swarm/hive intelligence sharing.
  4. **Escape from Loops**: Uses meta-cognitive watchdog daemons (`grim_reaper.rs`) to detect execution deadlocks and trigger swarm consensus resets.

---

## 4. Operational Protocols & Standards
* **Language Agnosticism**: Internal agents communicate via compressed binary nucleotide packets (`nucleotide_packet.rs`), minimizing token overhead and allowing non-human language models to execute hyper-efficient machine intelligence operations.
* **Safety & Gating**: All file modifications and input injections pass through the `security_hardener` and `permission_gate`, ensuring user intent is respected and unintended system access is blocked.
