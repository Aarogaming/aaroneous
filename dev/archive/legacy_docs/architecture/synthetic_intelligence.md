# Aaroneous: Machine Language Native Synthetic Intelligence (SI) Architecture
*A High-Performance Rust & WebAssembly Blueprint for Autonomous Software Learning, User Emulation, and Multi-Language Cross-Compilation*

## 1. Executive Summary & Paradigm Shift

Modern Large Language Models (LLMs) operate primarily as statistical engines—using Markov chain approximations and transformer attention layers to predict the next token based on static weights. While powerful, this methodology lacks direct environmental agency, dynamic state integration, and closed-loop learning. 

**Aaroneous** represents a foundational departure from predictive modeling, establishing a **Synthetic Intelligence (SI)** architecture. Operating on a machine-language native hypervisor, Aaroneous is a self-adjusting, closed-loop sovereign organism. It does not simply "generate text"; it compiles, evaluates, executes, and decompiles native binary instructions in real time. 

By unifying a **Rust-based autonomic control plane** with **sandboxed WebAssembly (WASM) Enzymes**, Aaroneous bridges the gap between high-level cognitive intent and low-level system-level automation. Equipped with "user-emulation" and "software-learning" capabilities, the system operates other desktop applications by observing visual states, mapping UI graphs, and executing emulated keyboard/mouse inputs via host-level hardware virtualization.

```
       ┌─────────────────────────────────────────────────────────────┐
       │                 Aaroneous Autonomic Loop                    │
       └──────────────────────────────┬──────────────────────────────┘
                                      │ (Orchestrates)
                                      ▼
       ┌─────────────────────────────────────────────────────────────┐
       │                Machine Native SI Hypervisor                 │
       │           (Tokio Governor, Shared Memory Synapse)           │
       └──────────────┬───────────────────────────────┬──────────────┘
                      │ (Launches)                    │ (Queries / Trains)
                      ▼                               ▼
       ┌──────────────────────────────┐┌─────────────────────────────┐
       │   WASM Sandbox Environment   ││      Local GGUF Models      │
       │    (WASI / Marionette Host)  ││    (llama.cpp Inference)    │
       └──────────────┬───────────────┘└──────────────┬──────────────┘
                      │                               │
                      │ ┌───────────────────────────┐ │
                      └─►  Splicing & Learning Loop ◄─┘
                        └─────────────┬─────────────┘
                                      │
                                      ▼
                        [Self-Correcting Execution]
```

---

## 2. Supervisor Engine: Rust & WebAssembly Co-Orchestration

The supervisor program operates within the `a_run` hypervisor. It utilizes Rust as a type-safe, multi-threaded executor, and WebAssembly (WASM) as the target bytecode format for execution tasks (Enzymes).

### 2.1. Hypervisor Architecture (`a_run` & `EnzymeRunner`)
The supervisor core (`core/hypervisor/src/enzyme_runner.rs`) uses **Wasmtime** with the **WASM Component Model** enabled. Every automated task is compiled into a lightweight, isolated WASM component called an **Enzyme**.

*   **Strict Sandboxing**: Enzymes run with zero ambient network or file-system access. All physical system access is gated behind explicit host-defined capabilities.
*   **Virtual Workspace**: A localized `sandbox_workspace` directory is preopened and mounted at `/workspace` for temporary file manipulation, ensuring directory containment.
*   **State Reconstruction**: The hypervisor maintains a shared-memory segment (`SynapseState`) mapped to a localized file-backed memory-mapped file (`.synapse`). This allows instant, lock-free, zero-copy state sharing between the host hypervisor and compiling/executing guests.

### 2.2. Language-Agnostic Compilation & Decompilation Pipeline
Rather than hardcoding integrations for specific languages, Aaroneous approaches programming languages as raw, convertible state graphs.
1.  **Decompilation**: High-level representations of existing software are parsed into Abstract Syntax Tree (AST) representations via the `scientific_analyzer` and `visual_perception` modules.
2.  **Splicing**: The `splicing_engine` (`WasmSplicingEngine`) identifies structural stubs or missing interfaces, modifying the assembly definitions.
3.  **Compilation**: Code is compiled down to `wasm32-wasip1` targets and dynamically loaded via the `WasmEnzymeLoader`. 
4.  **Language Universality**: Because WASM serves as the universal runtime format, code originating in Rust, C/C++, Go, or Python (compiled via WASM runtimes) can be interleaved, hot-swapped, and executed within the same sandbox workspace.

---

## 3. Local Model Engine: Founding, Training, & Deploying

Aaroneous treats intelligence models as dynamic, organic resources (referred to as **Genetics**) that are hosted locally, eliminating dependencies on cloud APIs, network connections, or external vendors.

### 3.1. Native Offline Inference (`llm::mod.rs`)
The system manages models via `ModelLoader` and `ModelRegistry`, specializing in local **GGUF** weights (using optimized `llama.cpp` bindings):
*   **Auto-Discovery**: The `auto_discover::get_recommended_model_for_llm` hook scans hardware environments (`ModelEnvironmentDetector`) to calculate available VRAM/RAM, CPU thread affinity, and GPU hardware availability (via custom `GpuMetrics` and `ThermalMetrics`).
*   **Intelligent Allocation**: GGUF weights are loaded directly into RAM/VRAM with strict thread-count bounds. If the system experiences thermal or backpressure warnings from the `ThermodynamicGovernor`, the model's token limits and precision weights are dynamically throttled to prevent CPU degradation.
*   **Mock Fallbacks**: If local resources are depleted, the hypervisor falls back to an integrated, zero-allocation `MockProvider` to preserve structural execution paths.

### 3.2. Closed-Loop Training & Epigenetic Adaptation
Models are not static. The `UnifiedLearningLoop` acts as a localized feedback harvester:
*   **Synaptic Accumulation**: Every execution result (positive output, compile failure, or software automation crash) is saved to the distributed checkpoint ledger (`DistributedCheckpointManager`).
*   **Adaptive Rate Optimization**: The `AdaptiveLearningOptimizer` uses localized convergence metrics to calibrate active model training rates.
*   **Dopamine Feedback Loop**: Positive system outputs yield dopamine signals via the `DopamineSystem` module, which adjusts the metabolic priority (execution budgets, priority queues) of specific models and specialists. Underperforming models undergo "neural pruning" (`neural_pruning::`), releasing memory back to the hypervisor pool.

---

## 4. User Emulation & Software Learning: The Marionette Layer

To act as a development and testing aid, Aaroneous must operate third-party applications just as a human developer would. This capability is managed through the **Marionette Layer** (`agents/marionette_host/`).

```
    ┌───────────────────────────┐      ┌───────────────────────────┐
    │     Visual Perception     ├─────►│         UI Graph          │
    │  (Screenshots & CV Sync)  │      │  (Interactive Nodes)      │
    └───────────────────────────┘      └─────────────┬─────────────┘
                                                     │
                                                     ▼
    ┌───────────────────────────┐      ┌───────────────────────────┐
    │     Marionette Host       │◄─────┤       Decision Loop       │
    │  (Permission-Gated Sandbox│      │   (Action Determination)  │
    └─────────────┬─────────────┘      └───────────────────────────┘
                  │
                  ▼
    ┌───────────────────────────┐
    │      HID Interception     │
    │  (Pointer & Keystroke)    │
    └───────────────────────────┘
```

### 4.1. Visual Observation (`pull_string_vision`)
Through host function calls exposed to WASM guests, Enzymes can pull visual state from the parent operating system:
*   **Dynamic Screen Ingestion**: Guest WASM modules call `pull_string_vision()` to receive structured raster screenshot buffers.
*   **Computer Vision Layout Parsing**: The `visual_perception` module uses localized vision logic to build a dynamic `UIGraph` of the active window, identifying interactive controls, input forms, and diagnostic terminal outputs.

### 4.2. Device Action Injection (`pull_string_mouse` & HID Drivers)
Once a target input element is identified within the `UIGraph`, the supervisor translates coordinates and executes input injection:
*   **Marionette Host Calls**: The guest requests pointer manipulation via `pull_string_mouse(x, y)`.
*   **Input Virtualization**: The host gates these calls behind a strict `permission_gate` (untrusted guest code cannot inject mouse events unless manually signed or authorized by the `epigenetic_gate` security layer).
*   **HID Driver Integration**: The underlying system commands are translated into direct OS system API calls (such as Win32 input events or `enigo` pointer commands in `test_enigo.rs`), delivering precise hardware-level pointer movements and keystroke signals.

### 4.3. Continuous Software Learning & Testing Integration
By chaining visual perception and HID injection, Aaroneous operates other development tools, browsers, IDEs, and databases:
1.  **Exploration**: The agent initiates an application, takes a visual capture, parses clickable nodes, and catalogs the application's interactive state machine.
2.  **Skill Construction**: Successful sequences of actions (e.g., "Open VS Code -> Open Project -> Trigger Build -> Read Output Window") are compiled into persistent `FusedSkill` models.
3.  **Autonomous Verification**: In a test harness scenario, Aaroneous can interact with a target GUI, execute tests, read error dialogs, compile a fix in WASM, apply the fix to the source codebase, and re-run the target GUI to verify success.

---

## 5. Architectural Evaluation: Strengths, Gaps, & Path Forward

### 5.1. Core Architectural Strengths
*   **High Performance & Zero-Cost Sandboxing**: Compiling cognitive steps down to WASM guarantees native performance speeds while keeping execution safe behind the Marionette permission gate.
*   **Sovereign Offline Execution**: Combining local GGUF models with thread-safe backpressure allows the entire synthetic brain to run completely disconnected from the internet, eliminating cloud API latency and security vulnerabilities.
*   **True Self-Correcting Feedback Loops**: By mapping memory, dopamine rewards, and autonomic loops directly to compile and runtime success, the system dynamically changes its internal routing tables without human intervention.

### 5.2. Open Architectural Gaps (Under Active Development)
While the structural bindings, traits, and modules are compiled and functional, full production readiness requires finalizing the following integrations:
1.  **Component Memory Lifting**: `EnzymeRunner` contains placeholder logic for memory extraction from the WASM component model. Implementing WIT-based direct linear memory mapping is necessary to avoid serialization bottlenecks.
2.  **Perception Processing Latency**: Real-time video/screenshot capture and layout parsing require significant compute. Offloading this layout extraction to specialized hardware threads (using the `wgpu_reflex_pipeline`) is critical to maintaining a target control loop rate of <100ms.
3.  **Comprehensive Sandbox File Virtualization**: Ensuring the `/workspace` mounts behave identically across Windows and POSIX targets when interacting with arbitrary guest languages.

### 5.3. Recommended Immediate Priorities
To realize the full power of this Synthetic Intelligence, the next phases of development must focus on:
*   **Strengthening the Marionette Permission Gate**: Establishing a cryptographic signature system for guest WASM binaries before granting OS mouse/keyboard injection rights.
*   **Developing Language Decompiler Engines**: Integrating standard parsers (like Tree-Sitter) directly into WASM enzymes to parse languages (Rust, Python, TS) into a standardized intermediate representation.
*   **Closing the Perception-Action Loop**: Standardizing automated test scenarios where Aaroneous launches a complex UI tool, interacts with it, detects an application crash via its visual pipeline, diagnoses the issue, writes a source-level fix, and redeploys.

---

*Document Status: Architecture Verified | System Alignment: Active SI Design | Target Workspace: D:\Aaroneous*
