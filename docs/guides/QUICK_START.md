# Aaroneous Quick Start

## Prerequisites

- Windows 10/11 (Linux/macOS supported)
- Rust toolchain (`cargo`, `rustc`)
- 500MB+ free disk space

## Build & Run

```bash
cargo build --release
cargo run --release -- start
```

**Output:** `target/release/a-run.exe`

## CLI Commands

```bash
# Start the hive with TUI dashboard
a-run start --dashboard tui

# Create a specialist
a-run specialist create --name "MySpecialist" --archetype "Scholar"

# List specialists
a-run specialist list --detailed

# Check system health
a-run status health

# View stats
a-run query stats --detailed

# Refresh SAB matrix
a-run status sab-matrix --refresh

# Help
a-run --help
```

## Workspace Resolution

Aaroneous discovers its workspace root via:
1. `AARONEOUS_WORKSPACE` environment variable
2. Current directory (if it contains `Cargo.toml` + `core/`)
3. Fallback to `D:\Aaroneous`

**No hardcoded paths** — all filesystem access goes through `WorkspacePaths::discover()`.

## Key Directories

```
Aaroneous/
├── core/              # Core crates (hypervisor, nervous_system)
├── components/        # Reusable components (agents, biology, sabs, etc.)
├── extensions/        # WASM and Python extensions
├── shards/            # Sovereign agent packages
├── sdk/               # Rust SDK
├── MaelstromUI/       # Tauri + React desktop UI
├── docs/              # Documentation
├── config/            # Configuration files
├── models/            # GGUF model files (gitignored)
├── data/              # Runtime data (SABs, routines, fabrication)
├── inbox/             # Drop files for ingestion
└── registry/          # Orchestration patterns, SAB observability
```

## Architecture Overview

Aaroneous is a **machine-native stem cell engine** with a federation of AI specialists:

- **Core Hypervisor** — Orchestrates specialists, manages WASM enzymes, handles IPC via shared memory
- **Nervous System** — Zero-copy synapse state (`#[repr(C)]`) for real-time communication
- **SABs (Skill Action Blocks)** — Reusable capability modules (WASM or native DLLs)
- **Federation** — Specialists (Visionary, Omnipresent, Symbiotic, Phygital, Archivist) collaborate via proposals and consensus
- **MaelstromUI** — Tauri + React desktop interface

## Configuration

Primary config: `config.toml` at workspace root.

```bash
a-run config show
a-run config validate
```

## Next Steps

1. **[Self-Hosting Guide](SELF_HOSTING_GUIDE.md)** — Deploy on your infrastructure
2. **[SDK Guide](SDK_CUSTOM_SPECIALIST_GUIDE.md)** — Build custom specialists
3. **[Architecture Overview](../architecture/FEDERATION_README.md)** — Understand the design
4. **[Current Status](../reports/CURRENT_STATUS.md)** — See what's implemented
5. **[Operational Runbook](../operations/OPERATIONAL_RUNBOOK.md)** — Day-to-day operations
