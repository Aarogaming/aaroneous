# Aaroneous Core Documentation Matrix

**Framework Version:** `v1.7.0`  
**Standard:** Sterile Execution Plane (SEP) / Zero-Allocation Substrate  
**Status:** Active Canonical Specification Manual  

---

## 1. The Evolutionary Lineage of Aaroneous

The development arc of Aaroneous represents a deliberate progression from rapid scripting prototypes toward a high-integrity, machine-native expansion substrate:

### Phase 1: Early Automation & Auxiliary Scripting (Legacy / AAS Era)
- **Origin:** Early automation scripts, helper utilities, and manual system interventions operating under legacy iterations (such as the *Aaroneous Automation Suite*).
- **Constraints:** Heavy reliance on Python-assisted auxiliary memory-mapping, dynamic scripting, and unconstrained execution loops. Highly functional for rapid experimentation, but prone to script fragmentation, brittle integration pathways, and technical debt.

### Phase 2: LLM Integration & Operator Harness
- **Pivot:** Ingestion of automated code generation tools and local language models via experimental prompt harnesses and agent execution loops.
- **Lessons Learned:** Exposed the fundamental flaws of treating LLMs as autonomous, unconstrained agents. Multi-intent batching and black-box background loops induced semantic drift and non-deterministic behavior, demonstrating the necessity of strict structural and operational boundaries.

### Phase 3: Early Rust Migration & Technical Debt Hardening
- **Transition:** Migration of core engine logic into pure Rust for thread safety, raw execution performance, and native OS integration.
- **Bottlenecks:** Initial Rust iterations carried over legacy metaphors and unstructured concurrency patterns, causing hidden heap allocations (`Vec`, `Box`, `String`) on hot paths and complex inter-module coupling.

### Phase 4: Formalization & The v1.7.0 Sterile Execution Plane (Current Era)
- **Modernization:** Radical restructuring eliminating biological and mythological metaphors in favor of rigorous systems-engineering principles.
- **Core Pillars:**
  - **Sterile Execution Plane (SEP):** Strict ban on dynamic heap allocations within hot telemetry, vision, and motor loops; memory traffic transits pre-allocated L1–L3 ring buffers and bounded stack buffers (`[u8; N]`) with 64-byte cacheline alignment (`align(64)`).
  - **Cratify Gatekeeper:** Automated compile-time AST and DAG static analysis pipeline ensuring all Accelerated Component Containers (ACCs) satisfy memory and safety invariants before compilation.
  - **Stateless Transducer Protocol:** Models bound to deterministic, single-instruction Text-in / Structured-Data-out transformations, decoupled via an isolated LLM Gateway and paced by priority-constrained backoff schedulers.
  - **Dense Specification Core:** Complete architectural transparency across a unified 6-document matrix.

---

## 2. The Prime Invariants (Zero Technical Drift)

All Aaroneous code, crates, and execution pipelines are governed by three non-negotiable operational invariants:

1. **Zero-Allocation Hot Paths:** No dynamic heap allocations (`Vec`, `Box`, `String`) on primary execution, perception, or telemetry paths. All data transit relies on fixed-capacity stack/slice buffers (`[u8; N]`, `[f32; N]`) and zero-copy plain-old-data contracts (`bytemuck::Pod` + `Zeroable`) with 64-byte cacheline alignment (`align(64)`) to eliminate micro-architectural jitter.
2. **Compile-Time Governance:** Structural integrity is never left to convention or heuristics. It is enforced deterministically via compile-time AST and DAG fingerprinting (`syn`) that rejects non-compliant modules prior to compilation.
3. **Reality-Grounded Engineering:** Rejection of ambiguous, anthropomorphic, or biological metaphors. Subsystems are defined strictly by physical and mathematical realities: cache-coherency, lock-free atomics, memory bounds, and explicit execution plane isolation.

---

## 3. Documentation Index

The `dev/docs` substrate is consolidated into 6 dense, canonical technical specification manuals:

| Specification | Target Scope | Core Subsystems |
|---|---|---|
| [`architecture.md`](./architecture.md) | Runtime Physics & Memory Topology | Sterile Execution Plane (SEP), Lock-Free Ring Buffers, `CapabilityBroker`, `EngineStatePublisher`, Priority Backoff Scheduler (RDTSC). |
| [`governance.md`](./governance.md) | Formal Verification & Safety Interlocks | Cratify Pipeline (`audit → scaffold → translate → verify → harvest`), AST/DAG verification gates, Zero-Panic error architecture, Windows Job containment (`max_blast_radius = "isolated"`). |
| [`cartridges.md`](./cartridges.md) | Machine-Native Container Format | `.si` v3.0 format, `rkyv` zero-copy archives, continuous HiPPO State-Space Model (SSM) tensor layout, `cubecl` GPU scan dispatch. |
| [`roadmap.md`](./roadmap.md) | Frontiers & Long-Term Milestones | 5 Architectural Pillars, 7-Horizon Frontier Matrix (H1–H7), Phase 38 deliverables, and historical phase index. |
| [`operations.md`](./operations.md) | Operational Playbook & Auxiliary Context | Legacy Unification & Assimilation Pipeline, Internal/External Dogfooding ($n!$ Expansion), Local Qwen Model / OpenCode integration, Android Head Unit diagnostics (`123456`, `7098HH`), and physical engineering boundaries. |
| `archive/` | Historical Audit & Transition Sprawl | Retained legacy documentation, Batch audits, historical benchmarks, and pre-v1.7.0 exploration documents. |

---

## 4. Workspace Status Snapshot

- **Rust Baseline:** Pure safe Rust (Strict ownership/borrowing, zero `unsafe` in core application logic).
- **Static Verification:** `cargo check --workspace` & `cargo clippy --all-targets` clean.
- **Contract Enforcement:** All public inter-ACC message structs enforce `#[repr(C, align(64))]`, derive `bytemuck::Pod` + `bytemuck::Zeroable`, and set `max_blast_radius = "isolated"`.
- **Runtime Model:** Standalone native binaries and dynamic libraries (`.dll` / `.so`) communicating via memory-mapped IPC and atomic CAS ring buffers; zero WebAssembly or conversational string bloat on the hot path.
