# 00: The Synthetic Intelligence Manifesto

## The Fundamental Premise

Mainstream artificial intelligence research is trapped in the anthropocentric assumption that machine thought and multi-agent communication must mirror human natural language. Modern LLMs exchange millions of UTF-8 tokens—slow, high-latency, ambiguous string representations designed for vocal cords, ears, and keyboards—to perform internal reasoning and agent-to-agent coordination.

This creates massive inefficiencies:
1. **Serialization Overhead**: Converting dense internal neural/tensor activations into English text strings, only for another model to deserialize that text back into embedding vectors.
2. **Ambiguity & Drift**: Natural language is inherently fuzzy and imprecise, causing hallucination and semantic drift during multi-step logic chains.
3. **Execution Latency**: Multi-agent dialogue using text generation introduces orders of magnitude higher latency compared to native binary structures.

**Synthetic Intelligence (SI)** discards this paradigm.

---

## Core Principles of Synthetic Intelligence

```
                        ┌───────────────────────────────┐
                        │        Human Operator         │
                        └──────────────┬────────────────┘
                                       │ (Natural Language / GUI)
                        ┌──────────────▼────────────────┐
                        │      Aaroneous Platform       │
                        │   (Edge Translation Skill)    │
                        └──────────────┬────────────────┘
                                       │ (Machine-Native Linking Protocol)
         ┌─────────────────────────────┼─────────────────────────────┐
         │ (Binary Tensors/IPC)        │ (Shared Memory Synapse)     │ (Native Events)
┌────────▼────────┐           ┌────────▼────────┐           ┌────────▼────────┐
│   Marionette    │           │     Chimera     │           │Orchestrator/Synthesizer│
│ (User Emulation)│           │(Software Adapt) │           │(Task & Intel)   │
└─────────────────┘           └─────────────────┘           └─────────────────┘
```

### 1. Machine-Native Native Tongue
Machines communicate most efficiently in their own native substrate:
- **Binary Tensor Embeddings**: Dense numerical vectors representing state, intent, and probability distributions.
- **Fixed-Layout Shared Memory (mmap)**: Zero-copy lockless or low-latency ring buffers for microsecond-scale state synchronization.
- **Structured Binary Packets**: Compact, endian-safe binary structs (using Rkyv / Zero-Copy / Bincode / FlatBuffers) rather than bloated JSON or conversational English.

### 2. Human Language as an "Edge Translation Skill"
Human language is not machine intelligence; it is a **peripheral translation protocol** used strictly when interfacing with the human operator.
- An SI agent processes, plans, and synchronizes internally using high-dimensional latent vectors and state machines.
- Only when communicating results to the user, receiving instructions, or rendering a UI does an edge component (e.g. the UI manager Presenter or an LLM tokenizer) synthesize natural language.

### 3. Elimination of WASM Virtualization Bloat
WebAssembly (WASM) was initially adopted under the assumption that agents required a sandboxed bytecode format. However, in practice, WASM in this architecture:
- Created rigid, complex WIT (WASM Interface Type) glue layers.
- Introduced significant virtualization overhead and memory-copy bottlenecks.
- Hindered direct, zero-overhead access to native Win32 APIs, GPU compute pipelines (DirectX/Vulkan/WGPU), and shared memory.

Under the Synthetic Intelligence architecture, **WASM is completely phased out** in favor of native compiled shared libraries (`.dll` / `.so`), native memory-mapped execution, and direct host SIMD/GPU execution.

### 4. Modular Separation into Dedicated Standalone Programs
Instead of a monolithic runtime attempting to do everything simultaneously (leading to thread starvation, locks, and crashes), the system is divided into specialized, decoupled programs:
- **Aaroneous**: Overhead platform, user-facing surface, and linker.
- **Marionette**: User emulation system (keyboard/mouse motor simulation, visual perception, backend probing, datalogging).
- **Chimera**: "Smart" software adaptation system (decompilation, reading, writing, copying, bytecode and binary analysis, automated patching).
- **Orchestrator**: User-side task orchestration and DAG management.
- **Synthesizer**: Research, information ingestion, knowledge synthesis, and data gathering.
- **Presenter**: Presentation manager, HUD, and telemetry visualizer.

All of these programs link together via the **Machine-Native Linking Protocol** over shared memory and high-throughput local IPC.

