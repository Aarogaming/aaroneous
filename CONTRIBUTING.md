# Contributing to Aaroneous

## Before you start

Run the verification gate. This is the single command that validates your workspace matches CI:

```bash
cargo xtask gate
```

If it passes, you're good. If it fails, fix the first error and re-run.

## Rules (quick reference)

The full policy is [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md) (v2). Every crate meets a **universal floor** and declares a **compliance profile** that adds stricter rules.

### Universal floor (every crate)

| Rule | What it means |
|------|--------------|
| **No `todo!()` or `unimplemented!()`** | Banned in committed code. Propagate `Result` instead. |
| **No prefix stutter** | Don't prepend `aaroneous_` or `aaroneous-` to crates, types, or modules. |
| **No ambient reads** | No `std::env::var`, `.canonicalize()`, clock reads (`SystemTime::now`/`Instant::now`), or ambient filesystem access outside bootstrap entrypoints. Inject config and time. |
| **No self-started threads/tasks** | Spawn through an injected executor or `orchestrator::Supervisor`. |
| **No panics on runtime input** | No `.unwrap()`/`.expect()`/`panic!` on I/O, config, or model-derived values. Mark provably infallible cases `// INFALLIBLE: <reason>`. |
| **No `unsafe impl Pod`** | Derive only. Manual `unsafe impl` is banned. |
| **Mandatory tempdir in tests** | Use `tempfile::tempdir()`, never touch ambient filesystem or env vars. |
| **Canonical names in governance** | `crates/governance` defines canonical type names. Legacy aliases are deprecated. |

### Profiles

Declare in the crate's `Cargo.toml`:

```toml
[package.metadata.cratify]
profile = "control"   # kernel | control | presentation | tooling
```

| Profile | Examples | Adds |
|---|---|---|
| `kernel` | `hypervisor`, `ipc_bus`, `compute`, `wire` | No heap on `#[hot_path]` code; `#[repr(C)]` + derived `Pod` boundary types; no locks on hot paths; `#![warn(unsafe_code)]` + `// SAFETY:` |
| `control` | `orchestrator`, `llm_gateway`, `governance` | Pure reducers, I/O in adapters, degraded paths at external calls; `#![deny(unsafe_code)]` |
| `presentation` | `api`, `studio_hud` | `#![deny(unsafe_code)]` |
| `tooling` | `ast_auditor`, `xtask` | `#![deny(unsafe_code)]` |

### Adding a dependency

Score it on compliance distance (`alloc`, `ambient`, `abi`, `safety`) and record the verdict in your PR: Admit, Admit with conditions, Extract pattern, or Reject. See CRATIFY_SPEC section 5.

### Bringing in outside code

Code from companion tooling, external projects, or generated drafts follows the graduation gate (CRATIFY_SPEC section 6): proven, classified, behind a workspace trait, landed inert or in shadow mode, origin copy deleted.

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

Lower rings are more privileged. Library crates in lower rings never import crates from higher rings (verified 2026-09-23 at `6321a63`). The `core/hypervisor` binaries are the composition root and are exempt: they wire every ring together. Rings govern dependency direction; compliance profiles govern which rules apply (see [docs/CRATIFY_SPEC.md](docs/CRATIFY_SPEC.md)).

## .si format

See [docs/SI_FORMAT.md](docs/SI_FORMAT.md) for the binary container specification.

## Naming conventions

Use standard systems names: `hypervisor`, `paths`, `wire`, `hud`, `api`, `bridge`, `controller`, `ingestor`, `pipeline`. Avoid monikers, puns, or biological metaphors for technical components.

## Getting help

- Full architecture: [docs/architecture.md](docs/architecture.md)
- Governance rules: [AGENTS.md](AGENTS.md)
- .si format spec: [docs/SI_FORMAT.md](docs/SI_FORMAT.md)

## Reviewer Checklist

Before approving any change, verify:

- [ ] `cargo xtask gate` passes
- [ ] Every new crate declares `[package.metadata.cratify] profile`
- [ ] No `todo!()` or `unimplemented!()` in new code
- [ ] No `.unwrap()`, `.expect()`, or `panic!` on runtime input outside tests/bootstrap (or marked `// INFALLIBLE:`)
- [ ] New types use canonical names from `crates/governance` (not legacy aliases)
- [ ] Tests use `tempfile::tempdir()`, never touch ambient filesystem or env vars
- [ ] No `std::env::var`, `.canonicalize()`, clock reads, or self-spawned threads/tasks outside bootstrap
- [ ] `kernel` crates: scan-loop code is marked `#[hot_path]` and has no heap allocation
- [ ] New crates follow zero prefix stutter convention (no `aaroneous_` prefix)
- [ ] No `unsafe impl Pod` or `unsafe impl Zeroable` (derive only)
- [ ] New dependencies carry a compliance-distance score and admission verdict in the PR
- [ ] Code imported from outside the workspace followed the graduation gate, and the origin copy is scheduled for deletion
- [ ] No profile loosened without recorded owner sign-off
- [ ] If adding a new CI check, add matching gate in `xtask/src/gate.rs`
