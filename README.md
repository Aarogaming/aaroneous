# Aaroneous

**Machine-Native Stem Cell Engine** — Rust AI agent system with a federation of collaborating specialists and WASM-based enzyme execution.

## What It Is

A Rust workspace for autonomous specialist agents, WASM enzyme execution, shared-memory synapses, and federation tooling. The current focus is stabilizing `a_run` while advancing the WASM/Sentinel frontier.

## Current State

- `cargo check -p a_run --all-targets --jobs 2` passes.
- `cargo test -p a_run --lib --jobs 2` passes (`782 passed, 0 failed, 3 ignored`).
- `cargo build -p a_run --bin a_run --jobs 2` passes.
- Remaining compiler output is warning-only.

## Operating Rules

- Run tests per crate, not `cargo test --workspace`.
- Use `--jobs 2` for heavy crates when needed.
- Workspace constraints live in `docs/guides/WORKSPACE_RULES.md`.

## Near-Term Frontiers

- Zig HID driver for low-latency marionette control.
- Predictive policy engine for intent-to-action refinement.
- Curiosity learning loop for feedback-driven adaptation.
- GGUF splicing and agent synthesis/hot-load workflows.
- MaelstromUI / Glass Workshop integration.

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

## Active Subsystems

| Subsystem | Status | Description |
|-----------|--------|-------------|
| Enzyme Runner | Live | Wasmtime Component Model sandbox with WASI isolation |
| Hox Registry | Live | SQLite-backed capability registry with permission round-trips |
| Metadata Ingestor | Live | Recursive filesystem and metrics ingestion pipeline |
| Orchestration Daemon | Live | Converts metadata into tasks and executes actions |
| Cellular Automata | Live | Fixed-width VSA/FSM/superposition layouts for deterministic memory |
| Retina Module | Live | Browser-based ingestion with safe teardown |
| Autonomic Loop | Live | Runtime management via shared-memory synapse |

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

Key docs:

- [docs/README.md](docs/README.md)
- [docs/guides/QUICK_START.md](docs/guides/QUICK_START.md)
- [docs/guides/WORKSPACE_RULES.md](docs/guides/WORKSPACE_RULES.md)
- [docs/reports/CURRENT_STATUS.md](docs/reports/CURRENT_STATUS.md)

## License

MIT
