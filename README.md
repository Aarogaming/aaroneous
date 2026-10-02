# Aaroneous

A type-safe Rust component framework for building zero-allocation execution blocks and plugins. Not a monolithic application — a collection of independent, plug-and-play component crates with strict trait boundaries and zero-copy contracts.

## Status

- **2,239+ tests passing** across 22 crates
- **11-gate verification** (`cargo xtask gate`) — encoding, fmt, clippy, build, tests, AST audit, stub check, emulator harness, release check, feature flags
- **Zero `todo!()` or `unimplemented!()`** in committed code
- **Zero prefix stutter** — no `aaroneous_` prefixes on crates, types, or modules

## Build

```bash
cargo xtask gate        # Full verification (run this first)
cargo run --release -p studio_hud --bin aaroneous   # Desktop HUD
cargo run --release -p hypervisor --bin hypervisor -- --help  # CLI
```

## Architecture

Five protection rings, lower = more privileged:

| Ring | Crates | Purpose |
|------|--------|---------|
| 0 | `core/hypervisor` | Microkernel host, execution loop |
| 1 | `ipc_bus`, `compute`, `core-contracts` | Real-time interconnect, SSM engine, zero-copy contracts |
| 2 | `orchestrator`, `governance` | Task scheduling, interference checking, resource governors |
| 3 | `capabilities`, `llm_gateway`, `platform_bridge` | MCP tools, LLM transport, OS abstractions |
| 4 | `api`, `studio_hud` | Desktop GUI (egui/eframe) |

Library crates in lower rings never import higher-ring crates; the `core/hypervisor` binaries are the composition root. See [CONTRIBUTING.md](CONTRIBUTING.md) for full rules.

## .si Format

Solid-state neural model cartridges. Three-block architecture:

| Block | Contents | Mutability |
|-------|----------|-----------|
| 1 | Frozen SSM weights (immutable base model) | Read-only |
| 2 | Dynamic adaptation matrix (LoRA delta) | Mutable at runtime |
| 3 | Episodic skill stack (mined habits) | Read-only |

64-byte aligned header, CRC32 integrity, zero-copy `memmap2` loading. See [docs/SI_FORMAT.md](docs/SI_FORMAT.md) for the binary specification.

## Quick Commands

```bash
cargo xtask gate                          # Verify workspace
cargo run -p ast_auditor -- audit core/ crates/ dev/  # AST audit
cargo test --workspace                    # Run all tests
cargo bench -p benchmarks                 # Run benchmarks
```

### Hypervisor

```bash
cargo run --release -p hypervisor --bin hypervisor -- start --tick 1000
cargo run --release -p hypervisor --bin hypervisor -- boot --profile isolated
cargo run --release -p hypervisor --bin hypervisor -- mesh --nodes 4 --live
cargo run --release -p hypervisor --bin hypervisor -- mcp --host 127.0.0.1 --port 8766
```

## Documentation

- [CONTRIBUTING.md](CONTRIBUTING.md) — Rules, verification gate, naming conventions
- [AGENTS.md](AGENTS.md) — Full operating directives and agent constitution
- [docs/SI_FORMAT.md](docs/SI_FORMAT.md) — Binary format specification
- [docs/architecture.md](docs/architecture.md) — Master architecture
- `governance/CROSS_AGENT_COORDINATION_PROTOCOL.md` in the private `aaroneous-devtools` companion repo — how multiple agent sessions (Claude, Codex, Antigravity/Gemini, local Qwen) coordinate work; not linked here since that repo is private

## Reviewer Checklist

Before approving any change:

- [ ] `cargo xtask gate` passes
- [ ] No `todo!()` or `unimplemented!()` in new code
- [ ] Every new crate declares `[package.metadata.cratify] profile` ([docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md))
- [ ] No `.unwrap()`, `.expect()`, or `panic!` on runtime input outside tests/bootstrap
- [ ] New types use canonical names from `crates/governance`
- [ ] Tests use `tempfile::tempdir()`, not ambient filesystem
- [ ] No `std::env::var`, `.canonicalize()`, clock reads, or self-spawned threads outside bootstrap
- [ ] `kernel`-profile crates: scan-loop code marked `#[hot_path]`, no heap allocation
- [ ] New dependencies carry a compliance-distance score and admission verdict
- [ ] New crates follow zero prefix stutter convention

## License

MIT
