# Aaroneous Rebuild: Comprehensive Roadmap

This document presents the full rebuild of the Aaroneous project roadmap, grounded in insights from the current directory structure and designed to avoid speculative planning. Every phase and subtask will directly correspond to existing modules and their contents, ensuring actionable guidance.

---

# Table of Contents

- [Project Principles](#project-principles)
- [Current Directory Analysis](#current-directory-analysis)
- [Development Phases](#development-phases)
  - [Phase 1: Modular Foundations](#phase-1-modular-foundations)
  - [Phase 2: Dynamic Ingestion Pipelines](#phase-2-dynamic-ingestion-pipelines)
  - [Phase 3: Testing Secure Execution](#phase-3-testing-secure-execution)
  - [Phase 4: Observability & Dual UI](#phase-4-observability--dual-ui)
---

## **Project Principles**
This roadmap aims to refactor and modularize the Aaroneous project into discrete, interoperable components while maintaining a high-performance Rust core. Key principles:

1. **Guided by Reality:** Rooted in the current directory structures and development assets across modules.
2. **Modular Refactoring:** Promote clean boundaries between critical units (e.g., Aaroneous, Guild, Library).
3. **End-to-End Integration:** Align orchestration and execution through Python <-> Rust/WASM interactions.

---

## **Current Directory Analysis**
### Module Insights

| Module                     | Purpose                                      | Key Subdirectories/Files                       |
|----------------------------|----------------------------------------------|-----------------------------------------------|
| **Aaroneous**              | Rust core + WASM engine                      | `src/`, `Cargo.toml`, `telemetry_aggregator.sab` |
| **AaroneousAutomationSuite** | Tactical automation with Python             | `artifacts/`, `test_data_machine.py`          |
| **Merlin**                 | Data intelligence (ingestion engine)         | `service.py`, `rotate_tokens.py`, `genome/`    |
| **Guild**                  | Task pipeline coordinator                    | `tests/`, `service.py`, `Guild/`               |
| **Library**                | SOP storage module                          | `Library/`, `Relic/`, `Dionysus/`              |
| **MyFortress**             | Sandbox/security module                     | `MyFortress/`, `runtime/`                      |
| **Maelstrom**              | Observability: TUI + GUI                    | `Maelstrom.bat`, `O3DE/`, `Gems/`             |

---

### **Phase 1: Modular Foundations**

#### Core Goals:
1. **Rust WASM Host:**
    - Secure execution by sandboxing nano-agent binaries.
    - Implement telemetry ingestion pathways using `telemetry_aggregator.sab`.
2. **Automation Alignment:**
    - Refactor execution layers in `AaroneousAutomationSuite` with WASM as core.

#### Subtasks:
1. **Rust Execution Scaffold**
   - Directory: `src/`
   - Actions:
     - Reorganize binaries under ergonomic Rust modules.
     - Configure **sandbox limits** for execution:
       ```rust
       let config = Config::default();
       config.static_memory_maximum_size(32 * 1024 * 1024); // Limit: 32MB
       ```

2. **Python Test Systems**
   - Directory: `AaroneousAutomationSuite/tests/`
   - Actions:
       - **Kernel Injection:** Adapt `repair_kernel.py` pipelines.
       - Deploy lightweight log gathering routines.
---

Each phase refined layer will expand.