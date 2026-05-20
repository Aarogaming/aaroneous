# Aaroneous Component Reference

This document covers all major code areas, including those without dedicated documentation.

## Core Hypervisor (`core/hypervisor/`)

### Binaries
| File | Purpose |
|------|---------|
| `bin/main.rs` | Entry point — CLI parsing, workspace discovery, specialist initialization |
| `bin/chimera.rs` | Bevy-based transparent overlay — Aaroneous's "eyes and hands" on the desktop |

### Core Modules (`src/`)
| Module | Purpose |
|--------|---------|
| `synapse.rs` | rkyv serialization wrapper for `SynapseState` — IPC/serialization boundary |
| `persistence.rs` | SQLite database layer — specialist records, learning state, event storage |
| `workspace.rs` | `WorkspacePaths` — central path resolver (env var → cwd → fallback) |
| `cli.rs` | CLI commands — start, specialist management, status queries, config, SAB matrix |
| `tui_framework.rs` | Terminal UI dashboard — system health, specialists, skill tree, event log |
| `inbox_system.rs` | Task inbox — receives and queues incoming tasks |
| `config_validator.rs` | Configuration validation — validates HOX maps, specialist configs |
| `chromosome_registry.rs` | Chromosome (binary module) registry — tracks available WASM/native enzymes |
| `model_registry.rs` | GGUF model registry — tracks available models and their metadata |
| `data_ingestion.rs` | File watcher and ingestion pipeline — watches inbox, routes to specialists |
| `self_digestion.rs` | Self-analysis — Aaroneous reads and understands its own codebase |
| `auto_fabricator.rs` | Automated crate fabrication — downloads, builds, and packages Rust crates |
| `metadata_ingestor.rs` | Watches files, git, system metrics — feeds compute engine |
| `unified_orchestration.rs` | Main daemon loop — observe, estimate, predict, route, act cycle |
| `decision_engine.rs` | Autonomous decision-making — evaluates tasks, routes to specialists |
| `action_executor.rs` | Executes actions — file operations, WASM enzyme calls, HID input |
| `orchestration_daemon.rs` | Daemon with metabolic throttling — runs unified cycles |

### Specialized Systems (`src/`)
| Module | Purpose |
|--------|---------|
| `tensor_router.rs` | Routes tensor operations to appropriate compute backends |
| `spectral_layout.rs` | Graph spectral layout for constellation visualization |
| `lora_adapter_vault.rs` | LoRA model management — loads and swaps adapters |
| `dopamine_system.rs` | Reward system — tracks and modulates reward signals |
| `chaos_monkey.rs` | Fault injection testing — tests system resilience |
| `diplomat_enzyme.rs` | Inter-agent communication protocol |
| `research_enzyme.rs` | Research capabilities — web search, paper analysis |
| `self_correction_enzyme.rs` | Self-healing — detects and corrects errors autonomously |
| `curiosity_enzyme.rs` | Intrinsic motivation — drives exploration and learning |
| `retina_module.rs` | Visual processing — screen capture and analysis |
| `sentinel_test.rs` | NLM sentinel and HITL handshake tests |
| `self_heal_test.rs` | Self-healing evolution loop tests |

### Federation (`federation/`)
| Module | Purpose |
|--------|---------|
| `specialists/visionary.rs` | Visionary specialist — inference, analysis, planning |
| `specialists/omnipresent.rs` | Omnipresent specialist — system-wide monitoring |
| `specialists/symbiotic.rs` | Symbiotic specialist — biometric/user state integration |
| `specialists/phygital.rs` | Phygital specialist — physical/digital world bridge (AR/XR) |
| `specialists/archivist.rs` | Archivist specialist — memory, knowledge, DNA bank |
| `specialists/generic.rs` | Generic specialist template for custom specialists |
| `graph/dag.rs` | Directed acyclic graph — model dependency and crystallization |
| `graph/analyzer.rs` | GGUF model analyzer — parses and reports model structure |
| `hive/mod.rs` | Hive management — specialist scheduling and coordination |
| `http/router.rs` | HTTP API — REST endpoints for specialists, models, forge, DNA |
| `tensor_vault/mod.rs` | Tensor vault — indexes and queries GGUF tensor metadata |
| `forge/mod.rs` | Forge — GGUF parsing, crystallization, hybrid model creation |
| `model_registry/mod.rs` | Model registry — tracks and manages GGUF models |

### Infrastructure
| Module | Purpose |
|--------|---------|
| `mcp_service/` | MCP (Model Context Protocol) service — auth, capabilities, HTTP API, transport |
| `mcp_bridge/` | MCP bridge — client/server communication protocol |
| `event_log/` | Event log — append-only store, compaction, replication |
| `raft_consensus/` | Raft consensus — election, log replication, snapshots, mutations |
| `wasm_ebus_bridge/` | WASM-EBus bridge — zero-copy communication between host and WASM enzymes |
| `hid_driver/` | HID driver — keyboard/mouse input control (rdev + enigo) |
| `spatial/` | Spatial engine — 3D/AR scene management |
| `wit/` | WIT (WebAssembly Interface Types) — WASM interface definitions |
| `agentic_players/` | Agentic players — shadow agents, intent analysis, policy enforcement |

## Nervous System (`core/nervous_system/`)

| Module | Purpose |
|--------|---------|
| `shared_memory.rs` | Canonical `SynapseState` — `#[repr(C)]` zero-copy layout for direct memory mapping |

## Components

### Agents (`components/agents/`)
| Module | Purpose |
|--------|---------|
| `agents.rs` | Agent definitions and specialist types |
| `workspace.rs` | Component-level `WorkspacePaths` resolver |

### Biology (`components/biology/`)
| Module | Purpose |
|--------|---------|
| `biology.rs` | `SystemBiology` — metabolic state, tokens, expression rate |
| `metabolic_governor.rs` | `PredictiveMetabolicGovernor` — load prediction, throttling decisions |

### SABs (`components/sabs/`)
| Module | Purpose |
|--------|---------|
| `sab_tensor.rs` | SAB tensor analysis — scientific computing on SAB data |

### Digestion (`components/digestion/`)
| Module | Purpose |
|--------|---------|
| `workspace.rs` | Digestion-level `WorkspacePaths` resolver |
| *(soul/personality digestion)* | Processes and internalizes specialist "soul" data |

### Control (`components/control/`)
| Module | Purpose |
|--------|---------|
| *(control plane)* | System control and orchestration |

### Skills (`components/skills/`)
| Module | Purpose |
|--------|---------|
| *(skill system)* | Skill definitions, evolution, and fusion |

### Scientific Analyzer (`components/scientific_analyzer/`)
| Module | Purpose |
|--------|---------|
| `batch_tensor.rs` | Batch tensor analysis — scientific computing pipelines |

## Extensions

### WASM (`extensions/wasm/`)
WASM enzyme implementations for compute tasks. Each enzyme is a standalone WASM module loaded by the hypervisor.

### Python (`extensions/python/`)
Python extension modules for tasks better suited to Python ecosystems.

## SDK (`sdk/`)

Rust SDK for building custom specialists and integrating with Aaroneous.

## MaelstromUI (`MaelstromUI/`)

Tauri + React desktop application with tabs for:
- Agent Factory — create and configure specialists
- Agentic Ops — agentic operations
- Command Center — system control
- SAB Arsenal — skill action block management

## Shards (`shards/`)

Sovereign agent packages. `shards/AAS_Core/` contains the core AAS (Aaron Autonomous System) implementation.

## Config (`config/`)

| File | Purpose |
|------|---------|
| `config.toml` | Primary configuration — LLM, database, federation settings |
| `specialist_registry.json` | Dynamic specialist registry — enabled specialists and their configs |
| `hox_map.json` | HOX gene map — positional roles and capability expression |
| `links.json` | Link configuration — external service connections |

## Registry (`registry/`)

| File | Purpose |
|------|---------|
| `sab_matrix.generated.json` | Auto-generated SAB-to-capability mapping |
| `sab_observability.json` | SAB observability configuration |
| `orchestration_patterns.json` | Orchestration pattern definitions |
| `target_crates.txt` | Target crate list for fabrication |
| `hox_*.json` | HOX presets for specific sovereigns |

## Data (`data/`)

| Directory | Purpose |
|-----------|---------|
| `sabs/` | Compiled SAB binaries (WASM/DLL) |
| `routines/` | Playback routines for Chimera |
| `fabrication/` | Fabricated crate workspaces |
| `training_data/` | Specialist training datasets |
| `federation_memory.json` | Federation-wide memory state |

## Inbox (`inbox/`)

Drop files here for automatic ingestion and processing by specialists.

## Models (`models/`)

GGUF model files (gitignored). Discovered via `WorkspacePaths::discover().models()`.

