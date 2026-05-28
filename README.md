# Aaroneous

**Machine-Native Stem Cell Engine** — Rust AI agent system with a federation of collaborating specialists and WASM-based enzyme execution.

## Workspace

```
Aaroneous/
├── core/                  # Core crates
│   ├── hypervisor/        # Orchestrator, specialists, WASM runtime, federation
│   └── nervous_system/    # Zero-copy shared memory (synapse)
├── components/            # Reusable components (17 sub-crates)
│   ├── intelligence/      # LLM routing, task analysis
│   ├── storage/           # SQLite persistence
│   ├── sabs/              # Skill Action Block system
│   ├── skills/            # Skill definitions and fusion
│   ├── genetics/          # Specialist genome, breeding
│   ├── digestion/         # Personality/soul digestion
│   ├── biology/           # Metabolic governance, thermodynamics
│   ├── agents/            # Agent definitions
│   ├── constellation/     # Constellation graph topology
│   ├── control/           # Control plane messages
│   ├── hive/              # Hive runtime orchestration
│   ├── compute/           # Mathematical compute engine
│   ├── scientific_analyzer/ # Pipeline analysis
│   └── paths/             # Workspace path discovery
├── MaelstromUI/           # Tauri + React desktop UI
├── federation/            # Federation HTTP, NATS, forge, specialists
├── docs/                  # Documentation
├── data/                  # Runtime data (SABs, routines)
├── config/                # Configuration files
├── registry/              # HOX maps, SAB manifests
└── agents.md              # OpenCode agent instructions
```

## Build & Run

```bash
cargo build --release
cargo run --release --bin spatial_kinetic
```

### Binaries

| Binary | Description |
|--------|-------------|
| `a_run` | CLI entry point for the Autonomic Nervous System |
| `spatial_kinetic` | Spatial-kinetic reflex loop — genome-driven HID output + wgpu reflex pipeline |
| `test_enigo` | Mouse automation test (enigo) |

## Current State

- **Compilation**: `cargo check -p a_run` — clean compilation.
- **Tests**: `cargo test -p a_run --lib` — 766 passed, 0 failed, 3 ignored.

### Active Subsystems

| Subsystem | Status | Description |
|-----------|--------|-------------|
...
| Autonomic Loop | Live | Fully integrated, managing agent runtime via shared-memory synapse |

## Tech Stack

- **Language**: Rust 2024 edition
- **Async**: Tokio (multi-threaded)
- **IPC**: Zero-copy shared memory (`#[repr(C)]` synapse), rkyv serialization
- **WASM**: Wasmtime (Component Model)
- **ML**: candle-core (tensor operations)
- **Database**: SQLite via rusqlite
- **Messaging**: NATS
- **UI**: Ratatui (TUI), egui/wgpu (native), Tauri + React (MaelstromUI)
- **Serialization**: rkyv (zero-copy), serde (JSON)

## Documentation

See [docs/INDEX.md](docs/INDEX.md) for the full index.

## License

MIT
