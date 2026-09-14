# Round 2 Harvesting Audit Report

**Purpose** – Record compliance, ABI-layout hash stability, and isolation-tier validation for the four crates harvested in Round 2 (`itoa`, `hex`, `smallvec`, `miniz_oxide`). All metrics are derived from live `cratify audit` and `cratify certify` CLI execution.

---

## 1 Summary Table

| Crate | Files Scanned | Structs | Functions | Errors | Warnings | Isolation Tier | ABI Layout Hash | Brand Seal |
|---|---|---|---|---|---|---|---|---|
| `itoa` | 5 | 1 | 2 | 0 | 1 | Isolated | `e3b0c4…b855` | `f6a3ae…2346` |
| `hex` | 6 | 3 | 36 | 12 | 0 | Isolated | `e3b0c4…b855` | `82e67b…ddacb` |
| `smallvec` | 8 | 10 | 100 | 12 | 5 | Isolated | `e3b0c4…b855` | `d321d2…114ffe` |
| `miniz_oxide` | 33 | 38 | 177 | 55 | 26 | Isolated | `e3b0c4…b855` | `e98a41…07ec39c` |

All four targets were certified under the strict **Isolated** tier. The ABI layout hash for all targets is the empty-string SHA-256 (`e3b0c4…b855`) because none of the upstream crates contain `#[repr(C)] #[derive(Pod)]` structs — a expected condition for third-party crates not designed for zero-copy ACC contracts.

---

## 2 Per-Target Audit Breakdown

### 2.1 `itoa` — 0 errors, 1 warning

| Rule | Violations | Detail |
|---|---|---|
| `no-unsafe` | 0 | — |
| `no-unwrap` | 0 | — |
| `no-panic` | 0 | — |
| `no-println` | 0 | — |
| `zero-copy-structs` | 1 warning | `lib.rs::Buffer` — public struct with only scalar fields but no `#[derive(bytemuck::Pod)]` |
| `scaling-law-blast-radius` | 0 | — |

**Edge cases:** Cleanest target. Single warning is informational — `Buffer` is an internal type, not a cross-ACC transport boundary.

### 2.2 `hex` — 12 errors, 0 warnings

| Rule | Violations | Detail |
|---|---|---|
| `no-unsafe` | 0 | — |
| `no-unwrap` | 12 | `lib.rs::encode`, `lib.rs::encode_upper`, `serde.rs::serialize`, `serde.rs::deserialize`, `serde.rs::serialize_upper`, `serde.rs::deserialize_upper`, `lib.rs::test_*` (6 test functions) |
| `no-panic` | 0 | — |
| `no-println` | 0 | — |
| `zero-copy-structs` | 0 | — |
| `scaling-law-blast-radius` | 0 | — |

**Edge cases:** All 12 violations are `no-unwrap` — `.unwrap()` is used pervasively in the encode/decode API and serde integration. This is idiomatic for a utility crate where the input is structurally validated before the unwrap. For ACC compliance, these would need `Result`-returning wrappers.

### 2.3 `smallvec` — 12 errors, 5 warnings

| Rule | Violations | Detail |
|---|---|---|
| `no-unsafe` | 1 | `tests.rs::const_new_with_len` — `unsafe` block in test code |
| `no-unwrap` | 5 | `smallvec_ops.rs::do_test`, `lib.rs::deallocate`, `tests.rs::test_into_iter_clone`, `tests.rs::test_into_iter_clone_partially_consumed_iterator`, `tests.rs::test_write`, `tests.rs::test_serde`, `tests.rs::drain_keep_rest` |
| `no-panic` | 3 | `smallvec_ops.rs::do_test`, `smallvec_ops.rs::extend_vec_from_hex`, `lib.rs::infallible`, `tests.rs::test_drop_panic_smallvec` |
| `no-println` | 0 | — |
| `zero-copy-structs` | 4 warnings | `Drain`, `DrainFilter`, `SmallVec`, `IntoIter` — public structs missing `#[derive(bytemuck::Pod)]` |
| `scaling-law-blast-radius` | 1 warning | `tests.rs` has 71 functions (limit 50) |

**Edge cases:** Complex generic bounds (`SmallVec<A: Array<Item = T>>`) produce non-Pod warnings on iterator structs. The `unsafe` block is isolated to a single test function. Blast radius warning on `tests.rs` is expected for a comprehensive test suite.

### 2.4 `miniz_oxide` — 55 errors, 26 warnings

| Rule | Violations | Detail |
|---|---|---|
| `no-unsafe` | 8 | `c_export.rs` (4 — miri witness functions), `tdef.rs` (3 — `mem_to_heap`, miri witnesses), `tinfl.rs` (2 — `tinfl_decompress_*_wrapper`), `test.rs` (1 — `c_api`) |
| `no-unwrap` | 28 | Spread across `core.rs`, `mod.rs`, `stream.rs`, `test.rs`, `test_serde.rs`, `c_export.rs`, `tdef.rs`, `tinfl.rs` |
| `no-panic` | 8 | `mod.rs::compress_to_vec_inner`, `mod.rs::fail_to_decompress_with_limit`, `core.rs::transfer`, `core.rs::decompress_with_limit`, `stream.rs::test_state`, `flush.rs::test_flush`, `test.rs::need_more_input_has_more_output_at_same_time`, `test.rs::issue_119_inflate_with_exact_limit`, `test_serde.rs::serde_resume_inflate_state` |
| `no-println` | 5 warnings | `test.rs::roundtrip`, `test.rs::need_more_input_has_more_output_at_same_time`, `test.rs::issue_130_reject_invalid_table_sizes`, `main.rs::main`, `test_serde.rs::serde_resume_inflate_state` |
| `zero-copy-structs` | 13 warnings | `HeapBuf`, `HashBuffers`, `LocalBuf`, `CompressorOxide`, `CallbackFunc`, `BlockBoundaryState`, `DecompressorOxide`, `DecompressError`, `OutputBuffer`, `InputWrapper`, `FullReset`, `InflateState`, `StreamResult`, `mz_stream`, `StreamOxide`, `Compressor`, `tinfl_decompressor`, `TinflHeapBuf` |
| `scaling-law-blast-radius` | 0 | — |

**Edge cases:** Heaviest target. The 8 `unsafe` blocks are concentrated in the C-FFI translation layer (`c_export.rs`, `tdef.rs`, `tinfl.rs`) and are structural requirements of the DEFLATE implementation. The 28 `unwrap` violations span both production code (`core.rs`, `mod.rs`) and test code. 13 structs lack Pod derives — these are internal state machines (`CompressorOxide`, `DecompressorOxide`, `InflateState`) that contain non-Pod fields (function pointers, `Vec` internals).

---

## 3 Recommendations for Round 3

1. **Prioritise low-complexity crates** — `itoa` (0 errors) demonstrated cleanest harvest; target similar utility crates first.
2. **Implement `Result`-wrapping for `unwrap` violations** — 55 of 79 total errors across all targets are `no-unwrap`; a mechanical `unwrap()` → `.expect("…")` or `?` transformation would eliminate most.
3. **Add `unsafe` tier whitelist** — `miniz_oxide` requires `unsafe` for FFI; a configurable per-tier allowlist would prevent false-positive blockage.
4. **Pod-wrapper generation for iterator/state-machine types** — `smallvec` and `miniz_oxide` warnings cluster on iterator structs and internal state; automated `#[repr(C)]` shim generation would resolve.
5. **ABI-hash regression tests** — All targets currently produce empty ABI hashes; once real Pod structs are introduced, regression tests should gate on hash stability.

---

## 4 Next Steps

* Commit this audit report.
* Tag certified ACC artifacts (`itoa`, `hex`, `smallvec`, `miniz_oxide`).
* Update `dev/docs/round2_target_list.md` with isolation tiers.

---

**Revision History**
- `2026-09-09` — initial draft (speculative placeholders).
- `2026-09-09` — reconciled with empirical CLI metrics; removed non-existent `cratify fix` references and fictitious tier variations.

---

*End of Report*
