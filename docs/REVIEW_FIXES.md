# Repository review fixes

The review's correctness and verification findings are addressed by targeted changes:

- Governance no longer caches authorization by node count and rounded energy. Each graph is checked in full, including direct callers. Invalid policy bounds fail closed. Single-graph certificates no longer claim pairwise non-interference. The reported prover backend reflects the Rust structural checks actually performed.
- Snapshot transport uses atomic payload words and validates sequence markers after copying. A shared nonblocking claim coordinates publishers. Readers refresh missing mappings explicitly. Version 3 uses a separate default endpoint. See [the transport contract](SNAPSHOT_TRANSPORT.md).
- WAL replay tracks the last complete record boundary and trims an incomplete suffix before accepting new writes. Record lengths are bounded before allocation. Complete corruption and non-EOF I/O errors fail without truncating the file. Regression tests append after recovery and reopen again.
- The auditor propagates parse, read and traversal failures and rejects missing or empty targets. Generated build output is excluded during traversal, but explicitly targeted files are checked. Evasion tests now invoke the auditor. Syntax rules recognize real hot-path documentation markers and check executable stubs and manual Pod implementations without treating literals as code.
- Supervision takes monotonic milliseconds from its caller, honors backoff and restart windows, resets consecutive failures after success, and rejects unsupported affinity. This is a control-plane task-step retry adapter, not an allocation-free OS process supervisor.
- Native Rust verification and CI invoke the canonical gate. The gate runs all-target compilation, full workspace tests, source auditing, the constitution's text inspection and the emulator harness. The command is platform-neutral.
- The obsolete WASM-artifact existence assertion is replaced by a test of the current executor's explicit unsupported-operation result. The WASM runtime had already been removed; this change does not reintroduce it.

Compatibility changes:

1. `SmtProofCache`, `SmtProofKey`, and `SmtActionInterlock::proof_cache` are removed. Call the evaluator for each graph. `smt_non_interference_verified` is false on single-graph certificates; use the pairwise report when evaluating interference.
2. `Supervisor::register_task` returns `Result`, and `tick(now_ms)` takes injected time and returns `Result`. Affinity must be implemented by the supplied task adapter. A zero restart budget prevents retries; exhausted window budgets defer retries until the next window.
3. Custom snapshot paths must migrate to a new version 3 segment after stopping older peers. Default endpoints are already separated. `SwmrSnapshotReader::refresh` belongs in acquisition/control-plane code, not the hot read path. Publication returns a typed error on contention or exhausted sequence numbers.
4. WAL records are limited to 16 MiB of serialized data. A complete existing record larger than this limit causes an error without modifying the file.

Architectural scope remains explicit: the allocation audit checks annotated syntax, not the transitive call graph; performance figures need reproducible measurements; the Rust plugin trait is an in-process contract, not a stable dynamic-library ABI. Broad kernel decomposition and a production dynamic-plugin ABI remain separate architecture work, rather than guarantees implied by this repair.

Validation completed on Windows on September 14, 2026:

- All six steps in the repository's sequential verification protocol passed, including the native Rust verifier.
- The final workspace run reported 2,118 passed tests and 4 ignored tests, with no failures. The separately required emulator harness also passed both tests.
- The architectural audit and required forbidden-pattern inspection passed. `git diff --check` reported no whitespace errors.
- Compiler warnings remain. Linux execution and the separate CI formatting/Clippy steps were not validated by this Windows run.

The final command output is retained in `review-fixes-final-gate.log` at the workspace root. Existing modified model-state files were preserved; no commit or push was made.
