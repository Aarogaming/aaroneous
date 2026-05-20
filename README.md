# Aaroneous

**Machine-Native Stem Cell Engine** — A high-performance Rust AI agent system with a federation of collaborating specialists.

## Quick Start

```bash
cargo build --release
cargo run --release -- start
```

## What Is Aaroneous?

Aaroneous is a synthetic intelligence engine built on three pillars:

1. **Core Hypervisor** — Orchestrates AI specialists, manages WASM enzymes, handles zero-copy IPC
2. **Federation of Specialists** — Visionary, Omnipresent, Symbiotic, Phygital, and Archivist collaborate via proposals and consensus
3. **SABs (Skill Action Blocks)** — Reusable capability modules in WASM or native form

## Architecture

```
Aaroneous/
├── core/                  # Core crates
│   ├── hypervisor/        # Main orchestrator, specialists, HTTP API
│   └── nervous_system/    # Zero-copy shared memory (synapse)
├── components/            # Reusable components (agents, biology, sabs, skills)
├── extensions/            # WASM and Python extensions
├── shards/                # Sovereign agent packages
├── sdk/                   # Rust SDK
├── MaelstromUI/           # Tauri + React desktop UI
├── config/                # Configuration
├── docs/                  # Documentation
└── data/                  # Runtime data (SABs, routines, fabrication)
```

## Key Commands

```bash
a-run start --dashboard tui              # Start with TUI
a-run specialist create --name X         # Create a specialist
a-run status health                      # Check system health
a-run query stats --detailed             # View statistics
a-run status sab-matrix --refresh        # Refresh SAB mappings
```

## Configuration

- **Primary config:** `config.toml` at workspace root
- **Workspace discovery:** `AARONEOUS_WORKSPACE` env var → current dir → `D:\Aaroneous`
- **Zero hardcoded paths** — all filesystem access uses `WorkspacePaths::discover()`

## Documentation

| Topic | Link |
|-------|------|
| Quick Start | [docs/guides/QUICK_START.md](docs/guides/QUICK_START.md) |
| Current Status | [docs/reports/CURRENT_STATUS.md](docs/reports/CURRENT_STATUS.md) |
| Component Reference | [docs/architecture/COMPONENT_REFERENCE.md](docs/architecture/COMPONENT_REFERENCE.md) |
| Mathematical Frameworks | [docs/architecture/MATHEMATICAL_FRAMEWORKS.md](docs/architecture/MATHEMATICAL_FRAMEWORKS.md) |
| Strategic Vision | [docs/reports/STRATEGIC_VISION.md](docs/reports/STRATEGIC_VISION.md) |
| Governance | [docs/operations/GOVERNANCE.md](docs/operations/GOVERNANCE.md) |
| Operational Runbook | [docs/operations/OPERATIONAL_RUNBOOK.md](docs/operations/OPERATIONAL_RUNBOOK.md) |
| Full Index | [docs/INDEX.md](docs/INDEX.md) |

## Tech Stack

- **Language:** Rust 2024
- **Async:** Tokio
- **IPC:** Shared memory (zero-copy `#[repr(C)]`), rkyv serialization
- **WASM:** Wasmtime
- **Database:** SQLite (rusqlite)
- **Messaging:** NATS
- **UI:** Ratatui (TUI), Tauri + React (MaelstromUI)
- **Consensus:** Raft

## License

MIT
