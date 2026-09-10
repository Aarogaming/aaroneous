# 03: Codebase Census & Forensic Audit

This document is the authoritative, file-by-file and folder-by-folder audit of the entire Aaroneous repository as of August 2026. Every folder and major crate has been directly inspected and evaluated.

---

## 🗺️ Top-Level Directory Census

| Directory / File | Type | True Function & Contents | Status & Reality |
| :--- | :--- | :--- | :--- |
| **`.cargo/`** | Config | Cargo configuration (linker flags, build profiles). | Active |
| **`.github/`** | CI/CD | GitHub workflows and CI configurations. | Active |
| **`.opencode/`** / **`opencode/`** | Metadata | Internal IDE and devops state files (`DEVOPS_STATE.json`). | Metadata |
| **`.synapse/`** | Temp Data | Local runtime synapse state files. | Ephemeral |
| **`AAS/`** | Python | Aaroneous Agent System bridge (`bridge.py`) and shard (`shards/merlin.py`). | Experimental Prototype |
| **`MaelstromUI/`** | Frontend | Tauri 2.0 + React + Vite user interface. | **DELETED** (Replaced by native `a_hud`) |
| **`agents/`** | Multi-Crate | 10 subdirectories with specialized agent definitions (`marionette_host`, `qa_engineer`, etc.). | Tangled / Partially Duplicate |
| **`bin/`** | Binary Output | Contains pre-compiled `a-run.exe` binary. | Build Artifact |
| **`cache/`** | Cache | Ephemeral compiler and model cache files. | Ephemeral |
| **`chromosomes/`** | Binaries/DLLs | Pre-compiled DLLs (`sensor_node.dll`, `tensor_forge.dll`), WASM enzymes, test genomes. | Mixed Native / WASM (To Be Purged) |
| **`components/`** | 20 Crates | The primary domain component crates (see Components Audit below). | Core Building Blocks (Need Decoupling) |
| **`config/`** | Config | 25 JSON and TOML configuration files defining NATS topics, habitats, and links. | Active |
| **`core/`** | 4 Crates | The central runtime crates (`hypervisor`, `nervous_system`, `chimera_vm`, `intelligence`). | Core (Contains Host-Disrupting Hooks) |
| **`data/`** | Data / DB | SQLite database (`styles.db`), `federation_memory.json`, and fabrication folders. | Active |
| **`deploy/`** | Infra | Helm charts, Terraform configurations, systemd service unit. | Deployment Config |
| **`docs/`** | Docs | 29 markdown files + 21 subdirectories containing conflicting legacy migration reports. | Unreliable / Superceded by `dev/docs/` |
| **`examples/`** | Rust | Example scripts (`discover_models.rs`, `run_federation.rs`). | Reference Code |
| **`exports/`** | Output | Exported models and build artifacts. | Ephemeral |
| **`extensions/`** | Modules | Extension points for dynamic runtime loading. | Under Construction |
| **`genesis_architect.sab/`** | Directory | Bundle directory containing `.sovereign` binary. | Disguised Folder Structure |
| **`genetics/`** | Models/Data | GGUF sources, Q6_K models, registry definitions. | Active |
| **`hox.db/`** | DB Storage | Database folder storing HOX genetic and trait persistence. | Active DB |
| **`include/`** | Headers | Native C/C++ header files for external bindings. | Interface |
| **`logs/`** | Logs | Runtime logs and stderr outputs (`chimera.err`). | Ephemeral |
| **`monitoring/`** | Monitoring | Prometheus YAML configs and Grafana dashboards. | Active |
| **`nats/`** | Binaries | Bundled NATS server executable (`nats-server.exe`). | Active Dependency |
| **`registry/`** | Metadata | System registry snapshots and crate mappings. | Active |
| **`resource_governor.sab/`** | Directory | Bundle directory containing sovereign resource governor. | Disguised Folder Structure |
| **`scripts/`** | Scripts | 38 Python/PowerShell scripts (transpilers, extractors, cheatsheets). | Ad-hoc / Cluttered |
| **`sdk/`** | SDKs | Client SDKs for `rust` and `python`. | Active |
| **`security_sentinel.sab/`** | Directory | Bundle directory containing sovereign security sentinel. | Disguised Folder Structure |
| **`shaders/`** | GPU Shaders | WGSL compute shaders (`spatial_delta_gate.wgsl`, `reflex_kernel.wgsl`). | Active GPU Kernels |
| **`telemetry_aggregator.sab/`**| Directory | Bundle directory containing sovereign telemetry aggregator. | Disguised Folder Structure |
| **`templates/`** | Templates | Starter templates (`universal_native`, `universal_sab`). | Active |
| **`tools/`** | Tools | Rust tools for metabolic monitoring and GUI dashboards. | Active |
| **`zstd/`** | Lib | Zstandard compression artifacts. | Ephemeral |

---

## 📦 Components Directory Deep-Dive (`components/`)

All 20 crates in `components/` evaluated:

| Component | Crate Name | Purpose & Code Reality | Status |
| :--- | :--- | :--- | :--- |
| `components/agents` | `agents` | Defines `Agent` trait, `RelicAgent`, `SpecialistAgent`, `UserAgent`, cognitive biases. | Clean Foundation |
| `components/biology` | `biology` | Thermodynamic governor, metabolic token forecasting, thermal throttle state. | High Quality Core |
| `components/chimera_marionette_loop` | `chimera_marionette_loop` | Combines Tree-Sitter AST patcher (Chimera) and Enigo screen/mouse loop (Marionette). | **Dangerous / Conflicted** |
| `components/compute` | `compute` | Advanced math (automata, bayesian, graph theory, linalg, Markov decision processes). | High Quality Math Lib |
| `components/constellation` | `constellation` | Graph database / spatial memory map representing nodes and relationships. | Active |
| `components/control` | `control` | Control plane message parsing and specialist state tracking. | Active |
| `components/deconstruction` | `deconstruction`| Deconstructs source code and AST structures. | Prototype |
| `components/digestion` | `digestion` | Persona ingestion engine (Experience, Narrative, Personality, Relational, Specialist). | Conceptual / Active |
| `components/foundry` | `foundry` | Specialist synthesis and build automation. | Active |
| `components/genetics` | `genetics` | Genetic loci, breeding operations, and specialist genomes. | Active |
| `components/hive` | `hive` | Multi-agent execution runtime and statistics collector. | Active |
| `components/intelligence` | `intelligence` | Task routing engine, LLM client, and task analysis. | Active |
| `components/marionette_host` | `marionette_host` | Enigo/Scrap wrapper with mocked mouse moves and screen capture. | Incomplete Prototype |
| `components/paths` | `aaroneous_paths`| Centralized workspace path resolver (`D:\Aaroneous`). | Clean Utility |
| `components/sab` | `sab` | Core SAB definitions. | Core |
| `components/sab_matrix` | `sab_matrix` | Matrix builder for sovereign agent bundles. | Core |
| `components/sabs` | `sabs` | Manifest loader and surface definition for SABs. | Core |
| `components/scientific_analyzer` | `scientific_analyzer` | Hypotheses, experiment design, AST observation, and pipeline verification. | Active |
| `components/skills` | `skills` | Skill registries, soul ranks, and fused skill sets. | Active |
| `components/storage` | `storage` | Persistence abstractions and disk serialization. | Active |

---

## ⚡ Core Directory Deep-Dive (`core/`)

| Core Crate | Crate Name | Purpose & Code Reality | Status |
| :--- | :--- | :--- | :--- |
| `core/hypervisor` | `a_run` | The master monolithic crate containing ANS autonomic loop, Win32 intercept, WGPU reflex pipeline, and Federation HTTP server. | **High Risk (Contains OS Hijacking Code)** |
| `core/nervous_system`| `nervous_system` | Basic definitions for the Synapse and signal propagation. | Minimal Crate |
| `core/chimera_vm` | `chimera_vm` | `#![no_std]` experimental 16-byte raw byte chunk parser with custom C-IR opcodes. | Incomplete Experiment |
| `core/intelligence` | `intelligence` | Intelligence shard modules. | Minimal Stubs |


