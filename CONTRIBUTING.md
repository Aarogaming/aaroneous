# Contributing to Aaroneous

## Before you start

Run the verification gate. This is the single command that validates your workspace matches CI:

```bash
cargo xtask gate
```

If it passes, you're good. If it fails, fix the first error and re-run.

## Rules (quick reference)

| Rule | What it means |
|------|--------------|
| **No `todo!()` or `unimplemented!()`** | Banned in committed code. Propagate `Result` instead. |
| **No prefix stutter** | Don't prepend `aaroneous_` or `aaroneous-` to crates, types, or modules. |
| **No ambient reads** | No `std::env::var`, `.canonicalize()`, or `std::fs::read` outside bootstrap. |
| **No heap on hot paths** | In `core/hypervisor`, `crates/ipc_bus`, `crates/compute`: no `String`, `Vec`, `Box`, `format!`. |
| **No `unsafe impl Pod`** | Derive only. Manual `unsafe impl` is banned. |
| **No `.unwrap()` on hot paths** | Propagate errors via `Result`. |
| **Mandatory tempdir in tests** | Use `tempfile::tempdir()`, never touch ambient filesystem. |
| **Canonical names in governance** | `crates/governance` defines canonical type names. Legacy aliases are deprecated. |

## Verification gate (full list)

`cargo xtask gate` runs these in order:

1. UTF-8 encoding (no BOM, LF endings)
2. `cargo fmt` check
3. `cargo clippy --workspace -- -D warnings`
4. `cargo check --workspace --all-targets`
5. `cargo test --workspace`
6. AST auditor (0 violations)
7. Zero-stub inspection (no `todo!()`, no `unsafe impl Pod`)
8. Emulator harness tests
9. Release binary check
10. Optional feature compilation
11. Iroh compatibility check

## Architecture at a glance

- **Ring 0**: `core/hypervisor` - microkernel host, execution loop
- **Ring 1**: `crates/ipc_bus`, `crates/compute`, `crates/core-contracts` - real-time interconnect and compute
- **Ring 2**: `crates/orchestrator`, `crates/governance` - task scheduling, safety interlocks
- **Ring 3**: `crates/capabilities`, `crates/llm_gateway`, `crates/platform_bridge` - ingress and transducers
- **Ring 4**: `crates/api`, `crates/studio_hud` - presentation layer

Lower rings are more privileged. Ring 0/1 never import Ring 3/4 crates.

## .si format

See [docs/SI_FORMAT.md](docs/SI_FORMAT.md) for the binary container specification.

## Naming conventions

Use standard systems names: `hypervisor`, `paths`, `wire`, `hud`, `api`, `bridge`, `controller`, `ingestor`, `pipeline`. Avoid monikers, puns, or biological metaphors for technical components.

## Getting help

- Full architecture: [docs/architecture.md](docs/architecture.md)
- Governance rules: [AGENTS.md](AGENTS.md)
- Operating model: [governance/OPERATING_MODEL.md](governance/OPERATING_MODEL.md)
- .si format spec: [docs/SI_FORMAT.md](docs/SI_FORMAT.md)

## Reviewer Checklist

Before approving any change, verify:

- [ ] `cargo xtask gate` passes
- [ ] No `todo!()` or `unimplemented!()` in new code
- [ ] No `.unwrap()` or `.expect()` on hot paths (propagate `Result`)
- [ ] New types use canonical names from `crates/governance` (not legacy aliases)
- [ ] Tests use `tempfile::tempdir()`, never touch ambient filesystem
- [ ] No `std::env::var`, `.canonicalize()`, or ambient reads
- [ ] Hot-path crates (`hypervisor`, `ipc_bus`, `compute`) have no heap allocation
- [ ] New crates follow zero prefix stutter convention (no `aaroneous_` prefix)
- [ ] No `unsafe impl Pod` or `unsafe impl Zeroable` (derive only)
- [ ] If adding a new CI check, add matching gate in `xtask/src/gate.rs`
