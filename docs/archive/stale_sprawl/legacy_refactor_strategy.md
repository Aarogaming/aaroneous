# Legacy Refactor Strategy

**Objective** – Provide a repeatable pipeline for ingesting monolithic, pre‑framework Aaroneous modules and producing isolated, zero‑copy ACC crates that satisfy `bytemuck::Pod` constraints.

---

### 1. Candidate Identification
| Category | Typical Files | Selection Criteria |
|---|---|---|
| Core utilities | `src/utils/*.rs`, `src/common/*.rs` | Small, pure‑logic functions, no OS‑specific APIs, limited external crate usage. |
| CLI parsers | `src/cli/*.rs` | Uses `clap`/`structopt`; mostly argument handling and simple validation. |
| Data formatters | `src/format/*.rs`, `src/serde_helpers.rs` | Serialises/deserialises known data structures; no runtime allocation beyond buffers. |
| Graphics helpers | `src/gfx/*.rs` (if they only wrap DXGI calls) | Must be stateless wrappers; no per‑frame heap allocation. |

> **Rule** – Any module that directly invokes `std::fs`, `std::process`, or performs dynamic memory allocation beyond a fixed‑size buffer is **ineligible** for the `Isolated` tier and must be rewritten or split.

---

### 2. Refactoring Rules
1. **Dynamic allocation → Fixed‑size buffers**
   - Replace `Vec<T>` with `[T; N]` where `N` is a compile‑time bound derived from the maximum expected size.
   - For variable‑size data, introduce a `RingBuffer<T, const CAP: usize>` that implements `bytemuck::Pod` and provides zero‑copy reads/writes.
2. **`unwrap` / `expect` → Result propagation**
   - Every fallible call must be rewritten to return `Result<_, CratifyError>` and propagated with `?`.
   - Insert explicit error mapping where the original panic message is needed for diagnostics.
3. **Lifetime tightening**
   - All references become `'static` by moving data into owned POD structs.
   - Functions that previously accepted `&mut self` for temporary state must be refactored into pure functions that take input structs and return output structs.
4. **Unsafe & FFI**
   - Any `unsafe` block is prohibited in the `Isolated` tier. Wrap such code in a separate `core`‑tier crate and expose a safe POD façade.
5. **Forbidden APIs**
   - Disallow `std::thread::spawn`, `async`/`await`, and any global mutable state (`lazy_static`, `once_cell`). Replace with explicit message passing through lock‑free ring buffers.

---

### 3. Ingestion & Rebuilding Pipeline
```
cratify inspect   --path d:\Aaroneous\src\utils   --mode legacy
cratify audit     --strict                        # enforces Pod & no unwrap
cratify certify   --tier isolated                 # generates Brand Seal
cratify scaffold  --output d:\Aaroneous\crates\utils_acc
```
1. **Inspect** – Parses the module, extracts all public symbols, and produces an interim `cratify.toml` describing required buffer sizes.
2. **Audit** – Runs `cargo clippy -- -D warnings`, static `assert_impl_all!` checks for `Pod`, and a custom linter that flags `unwrap`/`panic`.
3. **Certify** – Computes an ABI hash, creates the brand‑seal, and validates tier compliance.
4. **Scaffold** – Generates a new ACC crate with a clean `Cargo.toml`, `src/lib.rs` containing only POD structs and pure functions.

---

### 4. Acceptance Criteria
| Criterion | Pass Condition |
|---|---|
| **Pod compliance** | All public structs/enums implement `bytemuck::Pod`. |
| **No panics** | `cargo test` runs with `RUST_BACKTRACE=0` and exits cleanly; lint report shows zero `unwrap`/`expect`/`panic!`. |
| **Isolation tier** | `cratify certify --tier isolated` succeeds; brand‑seal matches expected hash. |
| **Zero‑copy API** | Public functions accept/return only POD types; no `Box`, `Rc`, `Arc`. |
| **Performance baseline** | Benchmark (via `criterion`) shows ≤ 5 % overhead compared to original implementation for identical workloads. |

---

### 5. Governance
- Each migrated crate must have an accompanying **verification checklist** (`dev/docs/verification_checklist_<crate>.md`).
- Changes to the refactoring rules require a **policy bump** and a new version tag (`vX.Y+refactor`).
- All ACC crates are merged to `main` only after automatic CI passes both the **code‑gen audit** and the **performance benchmark** stages.

*Document version*: `v0.1‑legacy‑refactor‑strategy` (2026‑09‑09)
