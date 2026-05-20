# Aaroneous: The Unified Sovereign Engine

This document outlines the final architectural vision for Aaroneous as the **Gold Standard** of our ecosystem. We are transitioning from a collection of "Sovereign Siblings" to a unified, multi-layered engine where Aaroneous acts as the high-performance host and orchestrator, while our existing repositories are refactored into modular components ("Enzymes") that fill in specific gaps.

---

## 1. The Unified Architectural Model

Aaroneous is no longer just a node in the mesh; it is the **Synthetic Fabric** itself.

| Layer | Technology | Role |
| :--- | :--- | :--- |
| **Execution (Core)** | **Rust / WASM** | High-performance, resource-governed "Enzymes." Handles HID, LLM Inference, and raw data processing. |
| **Orchestration (Middle)** | **Aaroneous HiveRuntime** | The Rust-native event loop (`hive_runtime.rs`) that coordinates all specialists, skills, and data flows. |
| **Intelligence (Top)** | **Python / Merlin** | High-level research, web scraping, and advanced reasoning. Acts as a "Cognitive Layer" calling into the Rust core. |
| **Observability (Face)** | **egui_ratatui** | A dual-mode dashboard providing both low-latency terminal monitoring and rich graphical visualization. |

---

## 2. Ingesting the Siblings: The Module Realignment

Each sibling repository is being refactored into a native module or a "Cognitive Shard" controlled by Aaroneous.

### **A. Merlin (The Intelligence Ingestor)**
- **Role:** Research & Ingestion.
- **Realignment:** Merlin's scrapers and distillation logic remain in Python but are invoked via the **Aaroneous mcp_service**. 
- **The Gap:** Merlin provides the *raw material* for the `a_run::data_ingestion` module.

### **B. Guild (The Task Dispatcher)**
- **Role:** Workflow Management.
- **Realignment:** The `Guild` service bus and workflow logic are absorbed by the `a_run::autonomous_coordinator`.
- **The Gap:** Guild's prior specialized "relic" agents become Era 2 specialists inside the Aaroneous Federation.

### **C. Library (The Knowledge Vault)**
- **Role:** Persistent Memory.
- **Realignment:** Library's SOPs and knowledge records are migrated to the **RocksDB-backed DNA Bank** within `a_run::persistence`.
- **The Gap:** Library provides the *epigenetic memory* that informs the `a_run::genetics` system.

### **D. MyFortress (The Security Gatekeeper)**
- **Role:** Sandbox & Validation.
- **Realignment:** MyFortress is integrated as the **WASM Validation Layer** within the `a_run::wasm_ebus_bridge`.
- **The Gap:** MyFortress provides the *resource governance* policies enforced by `a_run::biology`.

---

## 3. The Python Integration Layer

Python is no longer the "OS" of our agents; it is now a **Language Layer** that ties the ecosystem together.

- **The AAS Bridge:** The Aaroneous Automation Suite (AAS) is repurposed as the **Python SDK** for the Aaroneous engine.
- **IPC Protocol:** Python components communicate with the Aaroneous core via:
    1.  **NATS:** For asynchronous, cross-process task delegation.
    2.  **Shared Memory Synapse:** For high-speed telemetry reads (zero-copy).
    3.  **MCP (HTTP/SSE):** For structured capability invocation.

---

## 4. Implementation Roadmap (The Unified Path)

### **Phase 1: The Core Consolidation**
- **Action:** Finalize `hive_runtime.rs` as the single entry point for all system activity.
- **Action:** Implement `a_run::mcp_service` to allow external Python scripts to register capabilities as if they were native modules.

### **Phase 2: The "Enzymatization" of Logic**
- **Action:** Port performance-critical logic from `Merlin` (Skill Distiller) and `Fabricator` (WasmRuntime) into native Rust modules within Aaroneous.
- **Action:** Compile repo-specific "Business Logic" into WASM Enzymes that can be hot-swapped by the engine.

### **Phase 3: The egui_ratatui Dashboard**
- **Action:** Develop the `egui` dashboard in `a_run::tui_framework` (extending it to GUI).
- **Action:** Ensure the dashboard visualizes the "Constellation" of active modules, regardless of whether they are native Rust or Python-based Shards.

---
**Status:** Unified. 
**Next Checkpoint:** Integration of Merlin's Ingestion logic into the `HiveRuntime` update loop.
