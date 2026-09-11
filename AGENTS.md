# Repository Operating Directives (Aaroneous Workspace)


## Foundational Mandates

### 1. Toolchain
- **Rust Edition**: Rust 2024. All crates MUST inherit `edition.workspace = true`.
- **Compiler**: nightly (stable for production releases).

### 2. Zero-Heap Hot Paths
Hot paths in `hypervisor`, `ipc_bus`, `compute`, and `dev/emulator_harness` must NEVER use:
- `Vec`, `Box`, `String`, or any dynamic heap allocation
- Unbounded collections (`HashMap`, `BTreeMap`)
- Allocation in loops or tight iteration

### 3. Memory Geometry
All boundary types MUST:
- Use `#[repr(C)]` for fixed layout and ABI compatibility
- Derive `bytemuck::Pod` and `bytemuck::Zeroable` (no manual unsafe impls)
- Include explicit padding fields (e.g., `pub reserved: u16`) to guarantee alignment

### 4. Banned Anti-Patterns
The following are **explicitly forbidden** in all source trees:

| Pattern | Reason | Enforcement |
|---------|--------|-------------|
| Manual `unsafe impl Pod/Zeroable` | Violates soundness contract | Cratify rule |
| `std::sync::Mutex`, `RwLock` on hot paths | Serialization bottleneck, deadlocks | Hot path lint |
| `OnceLock`, `lazy_static` for shared state | Deferred initialization UB | Static init only |
| `unsafe transmute` on statics | Instant undefined behavior | Compile-time check |
| `todo!()`, `unimplemented!()` in production | Silent failure gates | Negative test |
| `.unwrap()`, `.expect()` in hot paths | Panic = crash, no recovery | Result propagation |

### 5. Concurrency Model
- **Single-writer**: Static buffers with atomic indices (`SwrnRingBuffer`)
- **Multi-reader**: Immutable references only
- **No mutexes** in telemetry or state extraction hot paths


## Forensic Ingestion Protocol (RFC-0005)

When processing legacy modules staged in `dev/legacy_staging/`:

1. **Containment**: Do NOT refactor or fix legacy code directly in place. Stage in `dev/legacy_staging/<artifact_id>/`.

2. **Trace & Isolate**: Route execution through `dev/emulator_harness` to extract 40-byte `TraceEvent` streams:
   ```rust
   use emulator_harness::{TraceEvent, extract_state_delta};
   
   let events = /* capture from emulator */;
   let delta = extract_state_delta(&events, target_addr)?;
   ```

3. **Kernel Extraction**: Use `extract_state_delta` to identify the minimal state transition needle (The "Do").

4. **Failure Analysis**: Document anti-patterns in `docs/forensics/<id>_<name>.md` (The "Don't"):
   - Dynamic allocation in hot loops
   - Pointer aliasing violations
   - Unbounded concurrency patterns

5. **Synthesis**: Rebase the clean, certified kernel into `crates/<target>/`:
   - Zero-copy stack-allocated arrays
   - Lock-free SWMR patterns where applicable
   - SMT invariant fences for numerical bounds

6. **Codification**: 
   - Record autopsy in `docs/forensics/`
   - Add Cratify rule to `crates/cratify/src/rules/`
   - Add regression test to `tests/negative_contracts/`


## Enforcement Gate

Before reporting completion, you MUST run and pass:

```bash
# 1. Workspace compilation
cargo check --workspace --all-targets

# 2. Invariant audit
cargo run -p cratify -- audit core/ crates/ dev/

# 3. Zero-stub verification
! git grep -n -E "(\btodo!\(|\bunimplemented!\(|unsafe impl.*Pod)" -- "crates/" "core/" "dev/"

# 4. Emulator harness tests
cargo test -p emulator_harness

# 5. Full suite
bash scripts/agent_check.sh
```

## Golden References

- **Zero-Allocation State Processing**: `dev/emulator_harness/src/reducer.rs`
- **Forensic Methodology**: `docs/rfc/RFC-0005-FORENSIC-INGESTION.md`
- **Case Study Template**: `docs/forensics/0001_aas_omni_galaxy_view.md`
- **Negative Contracts**: `tests/negative_contracts/test_omni_anti_patterns.rs`


## Agent Self-Verification

Always run `scripts/agent_check.sh` after major changes. This script:
1. Validates workspace compilation
2. Runs Cratify audit for anti-pattern violations
3. Scans for manual Pod impls, stubs, and unwrap usage
4. Ensures all dogfooding harnesses pass

If any check fails, fix the issue before continuing.


## Version

**Phase 38 Immune Ledger Active** - Last updated: 2026-09-11
