# Aaroneous: Brutal Reality & Refactoring Roadmap

This document replaces all previous aspirational roadmaps with a strategy grounded in the actual technical state of the `D:\` DevOps root as of May 17, 2026.

---

## 1. The Brutal Reality Report

Our audit of the sibling repositories reveals a fractured ecosystem split across three "Eras":

| Era | Repositories | Kernel Version | Technical State | Role in Roadmap |
| :--- | :--- | :--- | :--- | :--- |
| **Sovereign** | `AndroidNode` | `D5DB...` | **Gold Standard.** Clean CLI, standalone logic, renamed `Kernel` class. | **The Template.** All other repos must be refactored to this standard. |
| **Metabolic** | `Fabricator` | `4226...` | **Experimental.** Contains advanced `WasmRuntime` and `USSManager` logic. | **The Feature Lab.** Port these metabolic features into the Sovereign Kernel. |
| **Template** | `Merlin`, `Guild`, `Library`, `MyFortress`, `Maelstrom`, `AAS` | `E84D...` | **Legacy/Stagnant.** Hardcoded paths to `D:\AaroneousAutomationSuite`. Identical clones with different names. | **The Refactor Targets.** Must be decoupled from AAS and promoted to Sovereign status. |

### **Major Blockers Identified:**
*   **Hardcoded Dependency:** `Merlin`, `Guild`, etc., cannot run without `D:\AaroneousAutomationSuite` being present and populated.
*   **O3DE Overhead:** `Maelstrom` is tied to a heavy C++ Gem architecture that we are now depreciating in favor of `egui_ratatui`.
*   **Capability Fragmentation:** MCP manifests in the template repos are generic "TemplateRepo" stubs, not real capabilities.

---

## 2. The Refactoring Process: "The Sovereign Promotion"

We will refactor every sibling repository to work with the `Aaroneous` Rust core by following this standardized promotion path:

### **Step 1: Decoupling & Kernel Standardization**
- **Action:** Replace the `E84D` kernel in `Merlin`, `Guild`, etc., with the `AndroidNode` Sovereign Kernel (`D5DB`).
- **Action:** Rewrite `service.py` to remove hardcoded paths. Use `os.path.dirname(__file__)` to ensure every repo is self-contained.
- **Action:** Rename the `FederationHeartbeat` class to `Kernel` to match the new standard.

### **Step 2: Porting Metabolic Features**
- **Action:** Extract the `WasmRuntime`, `TensionEngine`, and `MetabolicResourceMonitor` from `Fabricator`.
- **Action:** Integrate these into the Sovereign Kernel so that *every* node in the federation can host WASM Enzymes natively.

### **Step 3: ABI & IPC Alignment**
- **Action:** Implement the **Aaroneous Universal ABI** in the Python Kernel's `WasmRuntime`.
- **Action:** Switch IPC from named pipes to the **Shared Memory Synapse** (for local high-speed telemetry) and **NATS** (for cross-repo orchestration).

---

## 3. Updated UI/UX Strategy: egui_ratatui

We are moving away from O3DE towards a unified Rust-native interface.

*   **Execution Monitor (Ratatui):** The "Dev Mode" remains in the `Aaroneous` Rust core (`src/tui_framework.rs`).
*   **Tactical Dashboard (egui):** The "Primary Interface" will be a graphical dashboard that embeds the Ratatui view using `egui_ratatui`.
*   **Data Pipeline:** The UI will read the **Shared Memory Synapse** populated by the Sovereign Kernels, providing a zero-copy visual canvas of the entire synthetic fabric.

---

## 4. Immediate Refactoring Priorities

1.  **Refactor `Merlin`:** Use it as the first "Sovereign Promotion" test case. Decouple from AAS and port the `AndroidNode` kernel.
2.  **Unify `WasmRuntime`:** Ensure the `WasmRuntime` in Python can communicate with the `wasm_ebus_bridge` in the `Aaroneous` Rust core.
3.  **Standardize MCP:** Update `mcp-manifest.json` in each repo to reflect its *actual* unique role (e.g., Merlin as Scraper, Library as Vault).

---
**Status:** Brutally Grounded. 
**Next Checkpoint:** Promotion of `Merlin` to Sovereign status.
