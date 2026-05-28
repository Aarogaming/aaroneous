# Aaroneous Documentation

Machine-Native Stem Cell Engine — Rust AI agent system.

## Workspace

```
Aaroneous/
├── core/hypervisor/        # Main orchestrator, WASM runtime, federation (37+ submodules)
├── core/nervous_system/    # Zero-copy shared memory (synapse), rkyv
├── components/             # 17 reusable crates (intelligence, sabs, genetics, biology, etc.)
├── MaelstromUI/            # Tauri + React desktop interface
├── docs/                   # This documentation
├── config/                 # config.toml, specialist_registry.json
├── data/                   # Runtime data
└── registry/               # HOX maps, SAB manifests, orchestration patterns
```

## Build

```bash
cargo build --release
cargo run --release --bin spatial_kinetic [--fps 60] [--genome path/to/genome]
```

## Documentation Index

- **[INDEX.md](INDEX.md)** — Full document index
- **[Quick Start](guides/QUICK_START.md)** — Getting started
- **[Current Status](reports/CURRENT_STATUS.md)** — Implementation snapshot
- **[Architecture](architecture/)** — Federation design, mathematical frameworks, MCP
- **[Operations](operations/)** — Deployment, runbook, monitoring, governance
- **[Implementation](implementation/)** — Crate analysis, tier checklist
- **[Phases](phases/)** — Phase planning documents
- **[Reports](reports/)** — Status, vision, audits
- **[History](history/)** — Archived session notes, superseded roadmaps, launch materials

## Current State

| Check | Status |
|-------|--------|
| `cargo check -p a_run` | 0 errors, 0 warnings |
| `cargo check --workspace` | 0 errors |
| `cargo test -p a_run` | Test-only compilation errors (hive_db `self_digestion` imports) |
| Workspace crates | 17, all compile cleanly |

### Active Subsystems

- **Decision Engine** — `AutonomousDecisionEngine` evaluates tasks, produces actions with deterministic confidence-threshold execution
- **Enzyme Runner** — Wasmtime Component Model sandbox with WASI isolation and `process-task` export calling
- **Wasm Splicer** — Valid Component Model header (0x0d) synthesis with `Component::new()` validation
- **Semantic Indexing** — `embed_text()` produces deterministic 384-dim unit vectors via candle-core tensor projection
- **Hive DB** — SQLite persistence for specialists, skills, constellations, event history, semantic embeddings
- **Prefrontal Cortex** — Plan generation from context + intents
- **Executive Plan** — Step-by-step plan execution with status tracking
- **Self-Correction Enzyme** — Fault diagnosis and WASM re-splicing pipeline
- **Diplomat Enzyme** — reqwest-based Agent Protocol communication with external agents
- **Federation** — 6 specialists (Sentinel, Visionary, Omnipresent, Symbiotic, Phygital, Archivist); HTTP status server; enterprise audit/compliance; forge (GGUF crystallization); NATS P2P; multi-hive clusters; sovereign packages
- **Spatial-Kinetic Engine** — HID reflex loop with genome-driven input, wgpu shader pipeline, configurable FPS
- **Autonomic Loop** — Staged (commented out in lib.rs pending HardenedEnvironment)

## Tech Stack

Rust 2024 · Tokio · Wasmtime (Component Model) · candle-core · rkyv · rusqlite · NATS · ratatui · egui/wgpu · Tauri + React

## License

MIT
