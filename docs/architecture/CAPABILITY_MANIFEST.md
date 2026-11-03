# Aaroneous Workspace Capability Manifest Specification (C79)

> **STATUS**: STABLE CONTRACT (Aaroneous External Interface)  
> **APPLIES TO**: External Controllers, Tooling, Devtools Foundry, Orchestration Planes  
> **WORKSPACE**: `aaroneous` (Type-Safe Rust Component Framework)  
> **SCHEMA VERSION**: `aaroneous-capability-manifest-v1`  
> **COMMAND**: `cargo xtask workspace-metadata`  
> **GOVERNANCE REF**: Finding G-2026-01 / Lane C79

---

## 1. Overview & Purpose

External controllers (such as the `aaroneous-devtools` Foundry control transformer, automated evaluation harnesses, or deployment orchestrators) require a deterministic, schema-versioned, and tamper-evident view of Aaroneous capabilities without relying on ambient environment discovery or inferring gate semantics from unstructured text.

The `workspace-metadata` capability manifest export fulfills this requirement by providing:
1. **Explicit Root & Revision Binding**: Requires explicit `--workspace-root <PATH>` and `--git-revision <HEX>` inputs, strictly verifying that the target checkout matches the expected Git commit SHA before exporting facts.
2. **Cratify Profile Map**: Explicitly exposes the compliance profile (`kernel`, `control`, `presentation`, or `tooling`) for every workspace member declared under `[package.metadata.cratify]`.
3. **Allowed Native Gate Catalog**: Explicitly enumerates allowed verification gate names matching the 12-stage sequential verification protocol in `xtask/src/gate.rs`, enabling external controllers to validate gate invocations and reject undeclared gates.
4. **Declared Artifact Contract**: Specifies the expected artifact filename, serialization format, and hashing algorithm.
5. **Deterministic SHA-256 Fingerprint**: Computes a canonical cryptographic fingerprint over all declared manifest facts.
6. **Zero Ambient Authority**: Eliminates ambient discovery functions (`std::env::current_dir`) in favor of constructor/CLI injection.
7. **One-Way Dependency Isolation**: The manifest export is implemented purely within the product `xtask` crate without acquiring any build, CI, test, or runtime dependency on `aaroneous-devtools`.

---

## 2. CLI Interface & Usage

### 2.1 Generating the Capability Manifest

```bash
cargo xtask workspace-metadata --workspace-root <PATH> --git-revision <HEX_SHA> [--output <PATH>]
```

- `--workspace-root <PATH>`: Mandatory path to the Aaroneous workspace root directory containing the top-level `Cargo.toml`.
- `--git-revision <HEX_SHA>`: Mandatory 40-character hexadecimal Git commit SHA. The tool verifies that the Git repository at `--workspace-root` resolves to this exact revision; any mismatch causes immediate failure with exit code 1.
- `--output, -o <PATH>`: Optional path to write the formatted JSON manifest. If omitted, the manifest is emitted to standard output.

### 2.2 Validating a Gate Name

```bash
cargo xtask workspace-metadata --check-gate <GATE_NAME>
```

- Returns exit code 0 if `<GATE_NAME>` is declared in the allowed native gate catalog.
- Returns exit code 1 with an informative error message listing all declared gates if an unknown or undeclared gate name is supplied.

---

## 3. Schema Structure (`aaroneous-capability-manifest-v1`)

```json
{
  "schema_version": "aaroneous-capability-manifest-v1",
  "workspace_root": "D:/Aaroneous",
  "git_revision": "1f8c4ed59ee580c7e5ac94df6ceed96029e8818f",
  "crates": {
    "scan_core": {
      "name": "scan_core",
      "version": "0.4.0",
      "path": "crates/scan_core",
      "cratify_profile": "kernel",
      "ring": null,
      "profile": [],
      "description": "Dependency-free no_std deterministic scan kernel for Aaroneous components",
      "dependencies": []
    }
  },
  "tools": [],
  "profiles": {},
  "cratify_profiles": {
    "scan_core": "kernel",
    "orchestrator": "control",
    "api": "presentation",
    "xtask": "tooling"
  },
  "allowed_gates": [
    {
      "name": "check-encoding",
      "stage": "1",
      "description": "Text encoding contract (UTF-8 without BOM, LF line endings)"
    },
    {
      "name": "fmt",
      "stage": "2",
      "description": "Rust code formatting check (cargo fmt --all -- --check)"
    },
    {
      "name": "gate",
      "stage": "all",
      "description": "Full sequential 12-stage gate verification suite"
    }
  ],
  "artifact_contract": {
    "manifest_file": "capability-manifest.json",
    "format": "json",
    "hashing_algorithm": "sha256"
  },
  "fingerprint": "a3cc37daf71068e29941652dd787897248902d40f96e9649d48cbbc63f52c5a2"
}
```

---

## 4. Allowed Native Gate Catalog

External controllers must only invoke declared native verification gates. The catalog mirrors `xtask/src/gate.rs`:

| Gate Name | Stage | Target Check |
|---|---|---|
| `check-encoding` | 1 | UTF-8 without BOM, LF line endings across all tracked files |
| `fmt` | 2 | Canonical Rust code formatting (`cargo fmt --all -- --check`) |
| `clippy` | 3 | Strict workspace Clippy check (`-D warnings` across all targets) |
| `check-native` | 3.5 | Native dependency boundary audit and provenance check |
| `check-deps-policy` | 3.6 | Dependency & license policy check (`cargo deny --all-features check`) |
| `check-audit` | 3.7 | Security advisory audit (`cargo audit`) |
| `check-semver` | 3.8 | Semver compatibility check (`cargo semver-checks check-release`) |
| `check-workspace` | 4 | Full workspace compilation (`cargo check --workspace --all-targets`) |
| `check-portable-core` | 4a | Host portable core compilation (`scan_core` no-default-features) |
| `check-arm-core` | 4b | Bare-metal ARM compilation (`scan_core` thumbv7em) |
| `check-wasm-core` | 4c | WebAssembly compilation (`scan_core` wasm32) |
| `check-portable-contracts` | 4d | Host binary contracts compilation (`core-contracts` no-default-features) |
| `check-arm-contracts` | 4e | Bare-metal ARM binary contracts compilation (`core-contracts` thumbv7em) |
| `check-wasm-contracts` | 4f | WebAssembly binary contracts compilation (`core-contracts` wasm32) |
| `test-workspace` | 5 | Workspace deterministic test suite execution |
| `ast-audit` | 6 | Structural and semantic invariant AST audit |
| `check-stubs` | 7 | Zero-stub inspection (no `todo!`, `unimplemented!`, or `unsafe impl Pod`) |
| `test-emulator` | 8 | Golden dogfooding emulator harness verification |
| `check-unwraps` | 9 | Unwrap/panic ratchet check against baseline |
| `check-deps` | 9.5 | Cratify profile dependency-direction ratchet check |
| `check-release` | 10 | Release binary compilation (`aaroneous`, `hypervisor`) |
| `check-optional-features` | 11 | Optional runtime features compilation |
| `check-iroh-feature` | 12 | Iroh P2P compatibility feature compilation |
| `gate` | all | Full sequential execution of all 12 stages in order |

---

## 5. Security & Verification Guarantees

1. **Rejection of Ambient Discovery**: Omitting either `--workspace-root` or `--git-revision` fails immediately with an error. No attempt is made to guess the workspace root or read ambient environment variables.
2. **Tamper-Evident Binding**: If a caller claims revision `X` but the repository checkout at the supplied root is at revision `Y`, the tool aborts with `Workspace root and git revision mismatch`.
3. **Cryptographic Fingerprint**: The SHA-256 fingerprint is computed over canonical JSON serialization of the manifest facts (excluding the fingerprint itself). Any manual alteration of crate versions, dependency lists, profiles, or gates invalidates the fingerprint.
