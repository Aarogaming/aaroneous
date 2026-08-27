# 01: Project Taxonomy & Subsystem Roles

This document defines the official taxonomy, roles, boundaries, and communication responsibilities of every major program and subagentic system within the Aaroneous platform.

---

## 🏛️ Program & Subagent Taxonomy

| Name | Class | Primary Purpose | Primary Input/Output |
| :--- | :--- | :--- | :--- |
| **Aaroneous** | Overhead Platform | Master supervisor, user-facing control plane, and inter-program linker. | Human Intent (In) ➔ System Topology / Linked Coordination (Out) |
| **Marionette** | Standalone Program | Frontend user emulation system with backend probing and high-frequency datalogging. | Reflex Tensors / Action Vectors (In) ➔ HID Emulation / Perceptual Datalog (Out) |
| **Chimera** | Standalone Program | "Smart" software adaptation system (decompilation, AST parsing, reading/writing/copying target software). | Target Binaries / Source Code (In) ➔ AST Modifications / Bytecode Patches (Out) |
| **Orchestrator** | Sovereign Specialist | User-side task management, high-level intent breakdown, DAG generation, and goal tracking. | User Objectives (In) ➔ Structured Task DAGs / Execution Tokens (Out) |
| **Synthesizer** | Sovereign Specialist | Intelligence gathering, research, external knowledge indexing, and deep technical data retrieval. | Research Prompts / Query Filters (In) ➔ Semantic Embeddings / Knowledge Graph (Out) |
| **Presenter** | Sovereign Specialist | UI management, HUD rendering, telemetry visualizer, and user presentation layer. | Telemetry Streams / System State (In) ➔ Real-Time GUI Display / Human Language Summaries (Out) |

---

## 🔍 Detailed Role Profiles

### 1. Aaroneous (Overhead Platform & Master Linker)
- **Scope**: The root operating environment and user-facing entry point.
- **Responsibilities**:
  - Initializes and monitors the lifecycle of all subordinate programs (Marionette, Chimera, etc.).
  - Manages the **Machine-Native Linking Protocol** bus and shared memory memory-mapped regions.
  - Translates high-level user directives into binary intent vectors.
  - Provides system-wide governance, metabolic load balancing, and thermal watchdog enforcement.

### 2. Marionette (User Emulation & Observability Engine)
- **Scope**: Frontend interaction, perceptual intake, and low-level probing.
- **Responsibilities**:
  - **Frontend User Emulation**: Simulates precise keyboard, mouse, and peripheral actions based on motor intent tensors (with strict safety toggles).
  - **Visual & State Intake**: Captures display frames, UI element coordinates, and spatial layouts.
  - **Backend Probing & Datalogging**: Hooks into target processes to log events, capture execution traces, inspect memory values, and monitor runtime behavior.

### 3. Chimera (Software Adaptation & Mutation Engine)
- **Scope**: Codebase manipulation, binary deconstruction, and dynamic software adaptation.
- **Responsibilities**:
  - Ingests and disassembles target binaries (PE/ELF, DLLs, game binaries) and source code.
  - Generates Abstract Syntax Trees (AST) using Tree-Sitter or native parser backends.
  - Identifies bugs, bottlenecks, or unoptimized routines and synthesizes code/bytecode patches.
  - Manages safe, sandboxed test execution of mutated code before deployment.

### 4. Orchestrator (User-Side Task Orchestrator)
- **Scope**: Executive function, planning, and task dependency resolution.
- **Responsibilities**:
  - Breaks down complex, multi-stage goals into directed acyclic graphs (DAGs).
  - Assigns sub-tasks to the appropriate specialized subsystem (e.g. routing research to Synthesizer, emulation to Marionette, patching to Chimera).
  - Tracks task completion, verifies outcomes, and recalibrates plans on failure.

### 5. Synthesizer (Intelligence & Knowledge Harvester)
- **Scope**: Information retrieval, deep technical indexing, and semantic memory.
- **Responsibilities**:
  - Scrapes, crawls, and indexes documentation, code repositories, specifications, and external data sources.
  - Maintains the semantic vector index and knowledge database.
  - Answers deep technical queries from other specialists using pre-computed vector embeddings.

### 6. Presenter (UI & Presentation Manager)
- **Scope**: Visual communication, dashboard management, and human-facing interface.
- **Responsibilities**:
  - Drives the machine-native Desktop Studio (`a_hud` / Rust egui) and terminal dashboards (legacy MaelstromUI Tauri/React deprecated).
  - Visualizes real-time DAG execution, node health, metabolic token burn, and GPU telemetry.
  - Acts as the primary human-language synthesizer, converting raw binary machine metrics into intuitive visual and textual feedback.
