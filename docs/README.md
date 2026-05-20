# Aaroneous: Machine-Native Stem Cell Engine

Aaroneous is a high-performance, Rust-native synthetic intelligence engine with a federation of collaborating AI specialists.

## Architecture

- **Core Hypervisor** — Orchestrates specialists, manages WASM enzymes, handles IPC via shared memory (`core/hypervisor/`)
- **Nervous System** — Zero-copy synapse state (`#[repr(C)]`) for real-time communication (`core/nervous_system/`)
- **SABs (Skill Action Blocks)** — Reusable capability modules in WASM or native DLL form (`components/sabs/`)
- **Federation** — Specialists (Visionary, Omnipresent, Symbiotic, Phygital, Archivist) collaborate via proposals and consensus
- **MaelstromUI** — Tauri + React desktop interface (`MaelstromUI/`)

## Workspace Structure

```
Aaroneous/
├── core/              # Core crates
│   ├── hypervisor/    # Main orchestrator, specialists, HTTP API, WASM runtime
│   └── nervous_system/ # Zero-copy shared memory (synapse)
├── components/        # Reusable components
│   ├── agents/        # Agent definitions and workspace paths
│   ├── biology/       # Metabolic governance, system biology
│   ├── sabs/          # Skill Action Block system
│   ├── digestion/     # Soul/personality digestion
│   ├── control/       # Control plane
│   └── skills/        # Skill system
├── extensions/        # WASM and Python extensions
├── shards/            # Sovereign agent packages (AAS_Core)
├── sdk/               # Rust SDK
├── MaelstromUI/       # Tauri + React desktop UI
├── config/            # Configuration files (config.toml, specialist_registry.json)
├── models/            # GGUF model files (gitignored)
├── data/              # Runtime data (SABs, routines, fabrication)
├── inbox/             # Drop files for ingestion
├── registry/          # HOX maps, SAB manifests, orchestration patterns
└── docs/              # This documentation
```

## Quick Start

```bash
cargo build --release
cargo run --release -- start
```

## Key Commands

```bash
a-run start --dashboard tui        # Start with TUI
a-run specialist create --name X   # Create specialist
a-run status health                # Check system health
a-run query stats --detailed       # View statistics
```

## Configuration

Primary config: `config.toml` at workspace root.
Workspace discovery: `AARONEOUS_WORKSPACE` env var → current dir → `D:\Aaroneous`.

## Documentation

- **[Quick Start](guides/QUICK_START.md)** — Get running in 5 minutes
- **[Current Status](reports/CURRENT_STATUS.md)** — What's implemented
- **[Strategic Vision](reports/STRATEGIC_VISION.md)** — Long-term roadmap
- **[Governance](operations/GOVERNANCE.md)** — System governance rules
- **[Mathematical Frameworks](architecture/MATHEMATICAL_FRAMEWORKS.md)** — Compute engine theory
- **[Operational Runbook](operations/OPERATIONAL_RUNBOOK.md)** — Day-to-day operations
- **[History](history/)** — Archived session notes, superseded plans, launch materials
