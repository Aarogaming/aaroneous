# Round 2 Open‑Source Rust Target List & Harvest Plan

**Version:** v1.2.0 (Aaroneous Framework)

---

## Candidate Crates
| Crate | Category | Why it fits ACC / Cratify goals | Minimum Version |
|-------|----------|--------------------------------|-----------------|
| `tokio` | Async runtime | Production‑grade, `no_std` optional, widely audited; provides task scheduler needed for microkernel‑tier `isolated` execution. | `1.28` |
| `smol` | Lightweight async runtime | Small footprint (`<200 KB`), ideal for ACCs that need low memory limits (`memory_limit_mb` ≤ 64). | `1.3` |
| `bytes` | Zero‑copy buffer library | Offers `Bytes` and `BytesMut` types that implement `bytemuck::Pod`; matches the zero‑copy contract for IPC frames. | `1.5` |
| `serde` (with `derive`) | (De)serialization | Standard for manifest and message payload encoding; `serde` structs can derive `Pod` via `bytemuck` when using `#[repr(C)]`. | `1.0` |
| `bitflags` | Bitmask utilities | Provides compile‑time checked flags for `OperationalIntent` and `Capability` – already used in `core‑contracts`. | `2.4` |
| `anyhow` | Error handling | Preferred error type for ACC audit/translate stages; integrates with `Result` without `unwrap`. | `1.0` |
| `thiserror` | Typed error definitions | Complements `anyhow` for domain‑specific error enums in ACC crates. | `1.0` |

---

## Harvest Plan (per crate)
1. **Add Dependency** – Insert the crate into the ACC's `[dependencies]` section of `cratify.toml` (or `Cargo.toml` if the ACC is already scaffolded).
2. **Audit** – Run `cratify audit` to ensure the crate satisfies the zero‑copy contract (`Pod` derivation) and does not introduce `unsafe` unless explicitly allowed.
3. **Harvest** – Once audit passes, execute `cratify harvest` to package the compiled binary, manifest, and dependency metadata into a signed ACC artifact ready for deployment.

---

## Integration Checklist
- Verify that each added crate compiles with the `max_blast_radius = "isolated"` setting (i.e., no global mutable state).
- Run the unit‑test harness (`cratify verify`) after adding a new dependency to catch integration regressions.
- Document any optional feature flags used in the ACC README for downstream developers.

*This list will be revisited after each major Cratify release to incorporate newer, vetted crates.*
