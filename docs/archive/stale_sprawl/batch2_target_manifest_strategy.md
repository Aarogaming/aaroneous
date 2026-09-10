# Batch 2 Target Manifest Strategy

**Objective** – Define the selection criteria, risk‑tier rating, and pre‑conditions for the next wave of pre‑framework Aaroneous modules that will be ingested by Cratify.

---

### 1. Candidate Categories
| Category | Example Modules | Typical responsibilities |
|---|---|---|
| **State Management** | `state_machine.rs`, `session_store.rs` | Holds runtime state, transitions, and persists session data. |
| **Configuration Parsers** | `config_loader.rs`, `toml_parser.rs` | Reads configuration files (TOML, JSON, YAML) and produces strongly‑typed structs. |
| **Mathematical Utilities** | `matrix.rs`, `fft.rs`, `numeric_ops.rs` | Pure‑math algorithms, linear algebra, signal processing, without side‑effects. |
| **IPC Handlers** | `ipc_channel.rs`, `msg_router.rs` | Wraps OS pipes, sockets, or Windows named pipes for inter‑process communication. |
| **Resource Managers** | `texture_cache.rs`, `audio_buffer.rs` | Manages GPU/Audio resources, often uses reference counting. |
| **Logging/Telemetry** | `log_sink.rs`, `telemetry_collector.rs` | Emits diagnostics; may allocate buffers for log aggregation. |

---

### 2. Risk‑Tier Rating System
| Tier | Definition | Typical deal‑breakers |
|---|---|---|
| **Low** | ‑ Minimal external crate dependencies.<br>‑ No dynamic allocations beyond fixed‑size buffers.<br>‑ Pure functions, no global mutable state.<br>‑ No `unsafe` or FFI. | None – ready for immediate ACC conversion. |
| **Medium** | ‑ Uses a small set of well‑vetted crates (e.g., `serde`, `log`).<br>‑ Contains limited `Vec<T>` or `String` that can be bounded with a compile‑time capacity.<br>‑ May have isolated `unsafe` blocks that can be moved to a core‑tier façade. | Requires modest refactor to replace dynamic buffers with capped ring buffers. |
| **High** | ‑ Heavy coupling to OS APIs (file I/O, process spawning).<br>‑ Extensive use of `async/await`, threads, or `lazy_static` globals.<br>‑ Unbounded dynamic allocations or complex generic lifetimes.<br>‑ Relies on external C libraries via FFI. | Must be split into a core‑tier crate plus a thin ACC façade; not a priority for Batch 2. |

> **Guideline** – Prioritize **Low** tier first, then **Medium** where effort is proportional to expected value. **High** tier modules are deferred to later phases.

---

### 3. Pre‑conditions for Refactor Branch Creation
1. **Static Analysis Pass** – `cargo check` and `cargo clippy -- -D warnings` must succeed with **zero** warnings.
2. **Pod‑Readiness Scan** – Run the custom Cratify lint: `cargo run -p cratify-lint -- check-pod <module_path>`; all public types must already implement `bytemuck::Pod` **or** be trivially convertible to a POD wrapper.
3. **Dependency Audit** – No dependency on crates that themselves contain `unsafe` blocks unless they are wrapped by a verified core‑tier crate.
4. **Allocation Bound Declaration** – Any use of `Vec<T>`/`String` must be annotated with a maximum capacity (e.g., `Vec<T, const N: usize>` via the `arrayvec` pattern) or replaced with a `RingBuffer<T, CAP>`.
5. **Test Coverage Baseline** – At least one unit test exercising each public function, ensuring the module builds under `--no-default-features` (to guarantee isolation).
6. **Documentation Review** – The module must have up‑to‑date inline docs and a corresponding entry in `dev/docs/legacy_refactor_strategy.md` indicating its intended ACC tier.

If **all** of the above are satisfied, a new branch `feature/cratify-batch2-<module_name>` may be opened for the refactor.

---

### 4. Prioritization Workflow
1. **Inventory Scan** – Run a repository‑wide grep for files matching the candidate categories above.
2. **Risk Rating Assignment** – Apply the tier matrix to each candidate based on the criteria in Section 2.
3. **Score Aggregation** – Compute a simple score: `Score = (TierWeight) × (LinesOfCode / 1000)` where `TierWeight` is `1` for Low, `2` for Medium, `3` for High.
4. **Select Top‑N** – Choose the highest‑scoring Low‑tier modules first, then fill remaining capacity with Medium‑tier modules until the sprint budget (≈ 30 man‑days) is reached.
5. **Create Refactor Branches** – Follow the pre‑conditions in Section 3 and submit a PR for each selected module.

---

*Document version*: `v0.1‑batch2‑target‑manifest` (2026‑09‑09)
