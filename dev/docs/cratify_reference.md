# Cratify `cratify.toml` Reference Manual & Schema Specification

**Version:** v1.2.0 (Aaroneous Framework)

---

## Overview
The `cratify.toml` file describes an **Accelerated Component Container (ACC)** – a self‑contained Rust crate that can be scaffolded, audited, and harvested by the Cratify pipeline. All fields are strictly typed and validated at scaffold time.

---

## Top‑Level Table: `[acc]`
| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | yes | Human‑readable identifier of the ACC (e.g. `my_acc`). Must be a valid Rust crate name. |
| `version` | string (semver) | yes | ACC version, e.g. `0.1.0`. Follows Cargo semantics. |
| `description` | string | no | Free‑form summary shown in generated documentation. |
| `license` | string | no | SPDX identifier (e.g. `MIT`, `Apache-2.0`). |
| `authors` | array of strings | no | List of author email addresses. |
| `max_blast_radius` | string (`"isolated"` \| `"global"`) | yes | Zero‑copy contract enforcement. `"isolated"` forces the ACC to run in a sandboxed microkernel tier. |

---

## Workspace Metadata – `[workspace]`
```toml
[workspace]
# Optional root directory for the ACC workspace. If omitted, Cratify uses the crate's parent.
root = "./"
# List of additional paths to include in the workspace (e.g., examples, benches).
include = ["examples/*", "benches/*"]
```
* `root` must be a relative path inside the repository.
* `include` entries support glob patterns.

---

## Dependency Mapping – `[dependencies]`
```toml
[dependencies]
# Key is the Crate name, value is a table with version & optional features.
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```
* All dependencies are **audited** during the `audit` stage; crates without a `no_std` guarantee are rejected unless `allow_std = true` is explicitly set.
* Feature flags are enumerated to guarantee reproducible builds.

---

## Execution Tier – `[tier]`
```toml
[tier]
# Determines which microkernel tier the ACC runs in.
# `user` – runs in the same process as the hypervisor (fastest, least isolation).
# `isolated` – runs in a separate sandbox (Windows Job/Object or Linux namespace).
level = "isolated"
```
* The tier must be compatible with `max_blast_radius`. `isolated` tier **requires** `max_blast_radius = "isolated"`.

---

## Isolation Settings – `[isolation]`
```toml
[isolation]
# Optional fine‑grained sandbox settings.
memory_limit_mb = 64
cpu_quota_percent = 25
```
* These settings are applied by the `CapabilityBroker` at runtime.

---

## Example Manifest
```toml
[acc]
name = "example_acc"
version = "0.1.0"
description = "Demo ACC for Cratify"
max_blast_radius = "isolated"

[workspace]
root = "./"
include = ["examples/*"]

[dependencies]
serde = { version = "1", features = ["derive"] }

[tier]
level = "isolated"

[isolation]
memory_limit_mb = 128
cpu_quota_percent = 20
```

---

## Validation Rules (enforced by `cratify` binary)
1. **Schema Compliance** – All required tables/fields must exist.
2. **Semantic Versioning** – `version` must parse as a valid semver.
3. **Zero‑Copy Contract** – If `max_blast_radius = "isolated"`, every public struct inside the crate must derive `bytemuck::Pod`.
4. **No `unsafe`** – Crates flagged with `unsafe` are rejected unless `allow_unsafe = true` is set (highly discouraged).
5. **Feature Whitelisting** – Only dependencies listed under `[dependencies]` may be compiled.

---

## Lifecycle Commands (ACC‑Cratify‑Pipeline)
| Stage | Command | Description |
|-------|----------|-------------|
| **Audit** | `cratify audit` | Runs static analysis, checks `bytemuck::Pod` derivations, validates manifest. |
| **Scaffold** | `cratify scaffold` | Generates ACC skeleton (`src/lib.rs`, `Cargo.toml`, `cratify.toml`). |
| **Translate** | `cratify translate` | Converts the crate into a microkernel‑compatible binary with zero‑copy ABI. |
| **Verify** | `cratify verify` | Executes unit‑test harness inside the isolated tier. |
| **Harvest** | `cratify harvest` | Packages the binary, manifest, and metadata into a signed ACC artifact. |

---

*The above reference is the canonical source for all `cratify.toml` manifests used across the Aaroneous ecosystem.*
