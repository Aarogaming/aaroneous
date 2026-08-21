# 10: Auto-Wrapping & Software Adaptation Specification

## Objective

To enable Aaroneous to ingest, analyze, wrap, and adapt **any external target program** (CLI utilities, native DLLs/C-libraries, GUI applications, network services) and seamlessly integrate it into the cooperative Aaroneous utility mesh.

```
┌────────────────────────┐
│ Target Program Ingest  │ ──► [1. Chimera Inspection] ──► [2. Marionette Probing]
│ (CLI / DLL / App / API)│
└────────────────────────┘
                                                                   │
                                                                   ▼
┌────────────────────────┐                               ┌───────────────────────┐
│ Cooperative Utility    │ ◄── [4. NATS / Synapse Reg]  ◄─┤ 3. Auto-Wrapper      │
│ (First-Class Organ)    │                                │    Harness Generation │
└────────────────────────┘                                └───────────────────────┘
```

---

## ⚙️ The Four-Stage Auto-Wrapping Pipeline

### Stage 1: Structural Dissection (Chimera)
- **Target Analysis**:
  - **CLI Tools**: Analyzes `--help`, usage strings, subcommands, stdin/stdout behaviors, exit codes, and environment variables.
  - **Native DLLs / C-Libraries**: Parses PE/ELF export tables, symbol names, function signatures, and ABI calling conventions.
  - **Source Repositories**: Uses Tree-Sitter AST parsers to map functions, structs, and dependency graphs.
- **Output**: A structured `TargetCapabilityManifest` JSON/TOML definition.

### Stage 2: Interface Probing & Validation (Marionette)
- **Dynamic Probing**:
  - Executes dry-run test invocations of CLI parameters.
  - Hooks stdout/stderr streams to analyze output formatting (plain text, tables, JSON, binary streams).
  - For GUI targets: Locates window handles, inspects control IDs, and captures bounding boxes without firing destructive clicks.
- **Output**: An empirical `ProbeValidationReport` confirming working invocation patterns and expected latencies.

### Stage 3: Harness Code Synthesis (Stem Cell Auto-Wrapper)
- **Harness Generation**:
  - The Stem Cell code generator emits a lightweight, high-performance **Rust / C++ / Python adapter wrapper**.
  - Bridges the target's inputs/outputs to the **Machine-Native Linking Protocol (MNLP)**:
    - Inbound NATS commands (e.g. `aaroneous.v1.utility.<name>.invoke`) are unpacked and fed into the target via pipes, FFI calls, or process arguments.
    - Outbound target output is converted into binary tensor packets or structured binary responses and broadcast onto the NATS bus / shared memory.
- **Compilation**: The generated harness is compiled into a native binary or dynamic library.

### Stage 4: Mesh Registration & Cooperative Discovery
- The newly generated wrapper is started under the Process Supervisor.
- Sends a `HANDSHAKE_ANNOUNCE` packet (`0x0001`) to the Aaroneous Master.
- Odin adds the new capability to its Task Routing Table; Merlin indexes its parameters into the semantic knowledge graph.
- The external program is now a fully functional, cooperative organ within Aaroneous!

---

## 📦 Supported Wrapper Patterns

| Pattern | Target Type | Bridging Mechanism | Latency |
| :--- | :--- | :--- | :--- |
| **Piped Stream Adapter** | CLI Tools (grep, git, ffmpeg, curl) | Asynchronous non-blocking stdin/stdout Tokio process pipes. | ~1–5 ms |
| **Zero-Copy FFI Adapter** | Native DLLs / Shared Libs (`.dll`, `.so`) | Direct C-ABI dynamic linking (`libloading`), passing raw memory pointers. | < 5 µs |
| **Shared Memory Bridge** | High-throughput data engines (engines, databases) | Maps target output directly into `.synapse` ring buffer. | < 1 µs |
| **Virtual Emulation Adapter** | Closed-source GUI applications | Marionette visual capture + virtual input injection (sandboxed). | ~16–33 ms |
