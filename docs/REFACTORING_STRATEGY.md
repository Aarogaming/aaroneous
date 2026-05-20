# Aaroneous System Refactoring & Repurposing Strategy

To achieve the "Target State" of a high-performance, Rust-native Multi-Agent System (MAS), the existing Python-based ecosystem (`Merlin`, `Guild`, `Library`, `AAS`) must undergo a structured refactoring process. This document outlines the technical transition from "Monolithic Python Services" to "Federated Rust/WASM Enzymes."

---

## 1. The Refactoring Mental Model

The refactoring is guided by a three-layer shift:
1.  **Orchestration (Stay in Python):** Keep high-level task management, web scraping, and workflow logic in the Python `Kernel` where flexibility is key.
2.  **Execution (Move to WASM):** Port performance-critical "Business Logic" (e.g., GGUF parsing, data distillation, security scanning) into Rust-compiled WASM Enzymes.
3.  **Communication (Standardize on NATS/Shared Memory):** Replace local imports and side-car scripts with the **Aaroneous Federation Protocol** (NATS) and the **Shared Memory Synapse**.

---

## 2. Repository-Specific Repurposing Paths

### **A. Merlin (Intelligence Engine)**
- **CURRENT:** Large-scale Python scraping and inference orchestration.
- **REFACTOR:** 
    - Convert `scripts/inference/skill_distiller.py` and `thought_distiller.py` into **Rust Crates**.
    - Compile these into WASM modules (`enzymes/merlin_distiller.wasm`).
    - The Python service becomes a **WASM Host Controller** that feeds scraped data into the Rust distiller.

### **B. Guild (Task Execution)**
- **CURRENT:** Service bus coordination and metabolic monitoring.
- **REFACTOR:** 
    - The `scripts/core/workflow.py` logic is ported to the **Aaroneous Rust Alpha Node**.
    - `Guild` is repurposed as the **Federation Load Balancer**, managing "Worker Shards" via NATS JetStream.

### **C. Library (Knowledge Vault)**
- **CURRENT:** SOP storage and knowledge record management.
- **REFACTOR:** 
    - Transition from `.json` and `sqlite` files to a **RocksDB-backed DNA Bank** managed by the Rust core.
    - Repurpose the Python layer to handle **Natural Language Querying** (via LLM) that indexes the Rust-managed DNA Bank.

### **D. MyFortress (Security Gatekeeper)**
- **CURRENT:** Local sandbox validation for scripts.
- **REFACTOR:** 
    - Implement **WASM Sandboxing** (via `wasmtime` fuel/memory limits) as the primary security gate.
    - Ports the `scripts/utils/capability_audit.py` into a Rust-native **Sovereign Auditor** that runs inside the execution pipeline.

---

## 3. The "Shard-to-Alpha" Promotion Protocol

Existing Python services will join the federation using a standardized handshake:

1.  **Handshake:** The Python Service (`service.py`) connects to the NATS bus and broadcasts its `mcp-manifest.json` capabilities.
2.  **Lease:** The Aaroneous Rust node assigns a **Metabolic Lease** (VRAM/CPU caps) to the service.
3.  **Encapsulation:** The Rust node identifies Python logic that can be "Enzymatized" (converted to WASM) and flags it for the `Workbench` factory.
4.  **Supersession:** When a Rust-native version of a Python capability is ready, the Rust node transitions that task from the "Python Shard" to the "WASM Engine," effectively depreciating the Python code while maintaining the service interface.

---

## 4. Immediate Refactoring Checklist

- [ ] **ABI Standardization:** Finalize the `.wit` file in `D:\Aaroneous\templates\universal_sab` to ensure all Ported logic has a common interface.
- [ ] **Shared Memory Bridge:** Update `D:\AaroneousAutomationSuite\scripts\core\bus.py` to support `SharedMemorySynapse` reads from the Rust host.
- [ ] **Kernel Cleanup:** Remove `D:\Merlin\service.py`'s hardcoded path dependency on `D:\AaroneousAutomationSuite` to allow independent containerization/hosting.

---
**Document Status:** Grounded & Operational.
**Target Milestone:** First "Enzyme" port of `Merlin`'s skill distiller.
