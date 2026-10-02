---
name: local-agent-delegate
description: >-
  Delegate code generation, method synthesis, refactoring, and test fixture tasks to a local GPU-hosted model (vLLM OpenAI-compatible server) for token optimization, zero latency, and private offloading. Includes prompt staging, reasoning runaway suppression, modular task decomposition, and invariant verification loops.
---

# Local Agent Delegation (`local-agent-delegate`)

## Overview

This skill defines the operational protocol for delegating computation, code generation, refactoring, and test synthesis tasks to locally hosted models running on the user's workstation, served by **vLLM**'s OpenAI-compatible API (`http://localhost:8000/v1`, e.g. `Qwen/Qwen2.5-Coder-14B-Instruct-AWQ`).

This pattern drastically reduces cloud token consumption, leverages local GPU compute, and ensures tight iteration loops for high-volume systems code generation.

> **Migration note**: this workflow previously targeted Ollama (`http://localhost:11434`, `/api/chat`). vLLM's continuous batching and PagedAttention give materially better throughput under the repeated single-request delegation pattern this skill uses, especially across decomposed multi-phase jobs. Start the server with e.g. `python -m vllm.entrypoints.openai.api_server --model Qwen/Qwen2.5-Coder-14B-Instruct-AWQ --port 8000`.

---

## Architecture & Subsystems

```
┌─────────────────────────────────────────────────────────────┐
│                      Antigravity Agent                      │
│            (Decomposes task, drafts specifications)         │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│              scripts/local_agent_delegate.ps1               │
│  • -PromptFile / -PromptFiles (single-shot or -Decompose)   │
│  • -SystemPrompt (suppresses reasoning/thinking runaway)    │
│  • -OutputFile / -OutputFiles (streams directly to disk)    │
│  • -MaxTokens 4096 (allocates generous generation headroom) │
└──────────────────────────────┬──────────────────────────────┘
                               │ POST /v1/chat/completions
                               ▼
┌─────────────────────────────────────────────────────────────┐
│               Local vLLM Server (OpenAI-compatible, GPU)    │
│          Model: Qwen2.5-Coder-14B-Instruct (or similar)     │
└──────────────────────────────┬──────────────────────────────┘
                               │ Writes output to disk
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                  Invariant Audit & Compile                  │
│  • cargo check & cargo test                                 │
│  • cargo run -p ast_auditor -- review                       │
│  • AGENTS.md Conformance Gates (Zero Heap, #[repr(C)])      │
└─────────────────────────────────────────────────────────────┘
```

---

## Operating Protocol

### 1. Verification & Model Health
Always verify the local server is reachable and inspect the loaded model before dispatching large jobs:
```powershell
# Check vLLM's OpenAI-compatible health/model-list endpoint
Invoke-RestMethod -Uri "http://localhost:8000/v1/models" | Select-Object -ExpandProperty data | Select-Object id
```
`local_agent_delegate.ps1` also runs this check itself before every dispatch (single-shot or decomposed) and fails fast with a clear error if the server isn't up.

Primary model configured on this workstation:
- **`Qwen/Qwen2.5-Coder-14B-Instruct-AWQ`**: default coder model served by vLLM (swap via `-Model` for a different checkpoint vLLM has loaded).

---

### 2. Reasoning Runaway Suppression
Reasoning-tuned checkpoints can emit a long internal chain-of-thought before the actual answer. Without calibration, complex prompts can spend thousands of tokens purely in internal reflection and get truncated before producing usable code.

**Mandatory Countermeasures**:
1. **Calibrated System Prompt**:
   ```text
   You are a senior Rust systems programmer. Do NOT output any lengthy thinking trace or reasoning. Output only pure, complete Rust code.
   ```
   *(Keeps generation focused on code, not reflection.)*
2. **Headroom Budget**: Set `-MaxTokens 4096` (or 8192) so the model never runs out of tokens.
3. **Zero Temperature**: Set `-Temperature 0.0` for deterministic, reproducible code synthesis.

---

### 3. File-Staged Delegation Pattern
To prevent shell quote-escaping corruption of multiline prompts:
1. **Write the prompt to a staging file**: `scripts/prompt_<task>.txt`.
2. **Execute delegation via the PowerShell runner**:
   ```powershell
   pwsh -File scripts/local_agent_delegate.ps1 `
       -PromptFile scripts/prompt_<task>.txt `
       -OutputFile scripts/output_<task>.rs `
       -Model "Qwen/Qwen2.5-Coder-14B-Instruct-AWQ" `
       -MaxTokens 4096 `
       -Temperature 0.0
   ```
3. **Inspect and integrate** the generated code from `scripts/output_<task>.rs`.

---

### 4. Modular Task Decomposition (`-Decompose`)
Do not ask the local model to write an entire multi-struct crate in a single prompt. Split the task into focused, single-responsibility phase files and run them through `-Decompose`, which dispatches each phase in order and feeds every prior phase's generated output back in as context for the next one — so later phases see the exact code earlier phases produced instead of re-deriving it blind:

```powershell
pwsh -File scripts/local_agent_delegate.ps1 `
    -Decompose `
    -PromptFiles @(
        "scripts/prompt_phaseA_struct.txt",
        "scripts/prompt_phaseB_accessors.txt",
        "scripts/prompt_phaseC_conversions.txt",
        "scripts/prompt_phaseD_tests.txt"
    ) `
    -OutputFiles @(
        "scripts/output_phaseA.rs",
        "scripts/output_phaseB.rs",
        "scripts/output_phaseC.rs",
        "scripts/output_phaseD.rs"
    ) `
    -Model "Qwen/Qwen2.5-Coder-14B-Instruct-AWQ" `
    -MaxTokens 4096 `
    -Temperature 0.0
```

Typical phase split:
- **Phase A**: Struct definition, alignment attributes (`#[repr(C, align(64))]`), and compile-time `const _: () = assert!(...);` geometry assertions.
- **Phase B**: Core accessor and modification methods (`empty`, `is_valid`, `set_channel`, `get_channel`).
- **Phase C**: Serialization/conversion traits (`From`, `TryFrom`) and `#![no_std]` compatibility.
- **Phase D**: Unit tests (`bytemuck::bytes_of`, roundtrip serialization, edge cases).

`-OutputFiles` is optional per call — omit it (or pass `$null` entries) to print a phase's output to stdout instead of a file while still carrying it forward as context for the next phase.

---

### 5. Architectural Verification Gate
All code synthesized by the local model MUST pass the repository's machine contract:
1. **Compile & Unit Test**:
   ```powershell
   cargo test -p <crate_name>
   ```
2. **Pattern Conformance Review**:
   ```powershell
   cargo run -p ast_auditor -- review crates/<crate_name>
   ```
3. **Workspace Invariant Audit**:
   ```powershell
   cargo run -p ast_auditor -- audit core/ crates/
   ```

This gate applies per-phase under `-Decompose` just as it does for a single-shot call — integrate and verify each phase's output before the next phase builds on it, rather than batching verification to the end.

---

## Common Pitfalls & Solutions

| Pitfall | Symptom | Remediation |
| :--- | :--- | :--- |
| **Thinking Saturation** | Output file is 0 bytes; response truncated mid-thought | Add strict system prompt suppressing chain-of-thought; increase `-MaxTokens` to 4096+. |
| **Shell Quote Truncation** | PowerShell errors or truncated prompt | Always use `-PromptFile`/`-PromptFiles` instead of passing multiline string arguments via CLI. |
| **Server Unreachable** | `vLLM server unreachable at http://localhost:8000/v1` | Start the server: `python -m vllm.entrypoints.openai.api_server --model <model> --port 8000`; confirm the port matches `-Endpoint`. |
| **Model ID Mismatch** | `404` or `model not found` from vLLM | `-Model` must exactly match the model id vLLM was launched with (check `GET /v1/models`), not an Ollama-style tag like `qwen3.5:9b-q6`. |
| **Serde Array Limit (>32)** | `the trait bound [u8; N]: Serialize is not satisfied` | Serde only supports arrays $\le 32$ without helper crates. Split padding larger than 32 into chunks (e.g. `_pad1: [u8; 32], _pad2: [u8; 12]`). |
| **Implicit Compiler Padding** | `bytemuck` derive fails with unaligned struct | Add explicit padding bytes (`_pad0: [u8; 2]`) to naturally align 4-byte and 8-byte members. |
