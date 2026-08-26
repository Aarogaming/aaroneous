# Contributing to Aaroneous

Thank you for your interest in contributing to Aaroneous — a sovereign machine-native synthetic intelligence runtime.

## Getting Started

1. **Fork and clone** the repository
2. **Install Rust** (see `rust-toolchain.toml` for required version)
3. **Run the test suite**: `cargo test --workspace`
4. **Run clippy**: `cargo clippy --workspace -- -D warnings`

## Development Workflow

### Branch Naming

- `feat/short-description` — new features
- `fix/short-description` — bug fixes
- `refactor/short-description` — code restructuring
- `docs/short-description` — documentation changes

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(compute): add SiForge builder API
fix(orchestrator): resolve MDP routing skill matching
docs(readme): update architecture diagram
refactor(chimera): extract AST mutation into separate module
```

### Code Style

- **Rust Edition 2021** (except `core/hypervisor` which uses 2024)
- **Zero clippy warnings**: `cargo clippy --workspace -- -D warnings`
- **No `.unwrap()` in production code** — use `Result`/`Option` with `?` or `ok_or_else()`
- **No `unsafe`** unless absolutely necessary with `// SAFETY:` comment explaining why
- **Doc comments** on all public items: `///`, `//!`
- **`#[cfg(test)]`** modules for unit tests

### Testing

- All new features must include tests
- Run `cargo test --workspace` before submitting
- Integration tests go in `core/hypervisor/tests/`
- Unit tests go in the module they test

### Architecture

See `dev/docs/` for detailed architecture specifications. Key principles:

- **SI over AI**: Machine-native execution, not LLM wrappers
- **Tiered dispatch**: Thinkers (Tier 1) → Organizers (Tier 2) → Workers (Tier 3)
- **Solid-state**: `.si` binary containers with zero-copy mmap
- **Lock-free**: SPMC synapse bus for inter-agent communication

## Pull Request Process

1. Update documentation if changing public API
2. Add entries to `CHANGELOG.md` under `[Unreleased]`
3. Ensure all CI checks pass
4. Request review from a maintainer

## Security

Report security vulnerabilities privately via `SECURITY.md`. Do not open public issues for security bugs.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
