# Aaroneous repository review

Aaroneous has a useful architectural direction: isolate state reduction, make component inputs explicit, and represent execution boundaries with bounded data. The implementation contains credible building blocks, but the repository does not yet substantiate its framework-wide safety and determinism claims. The next development milestone should establish trustworthy contracts and verification around a small execution path before expanding the capability surface.

This assessment covers architecture, selected correctness-critical implementations, and the prescribed verification sequence. It is a targeted repository review, not an exhaustive security or formal verification audit. Initial local HEAD was `4c78cfe1861b55c893450f2e8b093ff7c6c4bf0b`, matching GitHub's latest returned commit. HEAD advanced during review to `4bc2f45ce37e2c43405801574525054c72f68427`. There were pre-existing edits, and tests ran against a changing workspace. Findings below identify source locations; test results must not be interpreted as certification of a frozen commit. No implementation fixes were made.

## Architecture and strengths

The workspace manifest lists 29 component packages. Separating `si_ir`, `si_format`, `core-contracts`, `paths`, and `wire` provides places to define stable semantics and binary contracts independently of presentation. The documented acquisition/reduction/output split is a sound organizing principle for a framework that must support deterministic execution. Constructor injection can make replay, simulation, and deployment use the same domain logic. [1][2]

The emulator reducer is a particularly clear reference implementation. It consumes a caller-provided trace slice, uses bounded local state, returns a typed error, and derives Pod/Zeroable for its explicitly laid-out result. This is a concrete example of the intended component style. Its two tests passed. Those tests demonstrate layout and functional reduction; the test named `test_zero_allocation_reduction` does not itself instrument allocator activity. [3]

The WAL crash tests cover truncated writes, invalid magic, tombstones, and compaction. These are valuable failure-oriented tests, although they stop short of the crucial append-after-recovery sequence discussed below. Cross-platform CI and a dedicated AST auditor are also useful foundations. [4][8]

The main architectural issue is incomplete separation. The hypervisor manifest includes compute, orchestration, governance, browser automation, network clients, graphics, persistence, adaptation, and tooling dependencies. A composition root can legitimately know about many adapters, but the execution kernel and its dependency closure should be independently buildable and reviewable. Crate count alone does not establish interchangeability. [2]

## Prioritized correctness findings

### P1: Proof-cache collisions can authorize a different graph

`SmtProofKey` contains only node count and energy rounded down to thousandths. `evaluate_action_graph` returns authorization on a cache hit before running the lattice verifier. Two graphs with different opcodes, allocation sizes, types, or dependencies can therefore share an approval. [5]

A concrete source-derived case is to authorize a one-node allocation of 1,024 bytes at energy 0.02, then evaluate a one-node allocation larger than the 64 MiB arena at the same energy. The second call takes the cached authorization instead of reaching the allocation-size check. This is a deterministic control-flow finding, not an executed reproduction. The stronger `evaluate_action_gate` wrapper adds checks, but direct callers exist in adaptation code and the public graph evaluator remains unsafe as an authorization boundary. [5][6]

Remove this cache until its identity includes the complete immutable graph and all policy inputs. A future cache should bind the result to graph content, verifier version, and limits, and protect against mutation after verification. Add adversarial tests where equal-size graphs differ in safety-relevant fields. Also validate non-finite numeric inputs explicitly.

### P1: Snapshot consistency is not established by the current synchronization

The publisher modifies mapped sequence markers and payload through ordinary non-atomic fields. The reader uses a separately created mapping and a separate lock, so the writer's lock does not protect reader accesses. Acquire/release fences do not by themselves synchronize non-atomic accesses; the Rust documentation explicitly requires an atomic communication operation. [7][12]

There is also an independent ordering defect: `read_from_mmap` checks markers and header at lines 922–928, then copies `slot.snapshot` at line 939. A writer that wraps after validation can modify the slot during that copy without a subsequent reader check. This is a source-level correctness finding; no dynamic race detector was run. [7]

Use a reviewed shared-memory ownership protocol with defined publication, reader lifetime, slot reuse, and platform guarantees. Merely changing marker types or adding fences is not a complete solution to concurrent payload access. Add forced-interleaving tests around slot reuse and validate in optimized builds. The HUD uses this reader, so the concern applies to a real telemetry path. [7]

### P1: WAL recovery leaves a damaged tail in the append stream

Replay stops when a record is incomplete, but `open` then opens the same file in append mode without truncating to the last valid record boundary. A later successful `put` appends after the damaged record. On the next restart, the old record length can consume bytes from the new record, causing parsing failure and loss of subsequent recoverable entries. [4]

Record the last valid offset during recovery and repair or quarantine the invalid tail before accepting writes. Add the full sequence: write records, truncate the last record, reopen, append another record, close, reopen, and verify the new record. Bound record lengths against an explicit maximum and remaining file size before allocating the replay buffer. Existing tests verify the first reopen only.

## Verification currently overstates coverage

The allocation visitor checks only functions/files with `hot_path` attributes. A search across `core`, `crates`, and `dev` found the attribute spelling only in the rule's documentation, not in production annotations. Consequently, zero reported allocation violations is not evidence that execution paths allocate nothing. This auditor also operates on syntax, not transitive calls or allocator behavior. [8]

Audit failure handling is permissive. A Rust parse error prints a diagnostic and returns success; traversal discards file audit errors and directory traversal errors. Missing targets do not cause failure. This behavior was observed directly: the full audit exited zero after reporting a parse error in `crates/adaptation_engine/target/scientific_test.rs`. Generated files should have explicit exclusions; unexpected parse/read failures should fail the gate. [8]

The agent-evasion tests are especially misleading: the inspected cases assign bad code to `_code` and call `assert!(true)` without invoking the auditor. They cannot detect regressions in the claimed rules. Replace these with actual auditor calls and assertions about violation type and location. [8]

The three gate definitions disagree. The constitution requires all-target compilation, full workspace tests, a broad audit, a textual stub scan, the harness, and the shell script. The shell script audits only four paths and runs library tests. CI runs workspace checks and tests but omits the architectural audit and explicit stub scan. A green shell gate can therefore miss integration failures that the full workspace command catches. [1][9]

## Claims that need narrower wording or stronger evidence

The examined `Z3Prover::verify_non_interference` implements Rust set intersections over register footprints. Its backend label switches to `Z3-SMT-v4.12` when a feature is enabled, but the inspected method does not invoke Z3. This is useful static analysis, but it should be described by the proof actually performed. In particular, a single-graph certificate marks non-interference verified without proving a relationship to a second graph. [5][10]

The plugin interface defines a Rust trait and descriptors, while `hotload` returns a loaded library. This does not yet define a versioned dynamic plugin ABI, exported entry point, ownership contract, unloading lifecycle, or compatibility negotiation. Keep descriptors and allocation-heavy discovery in the control plane; establish a concrete binary or process boundary before claiming independently interchangeable dynamic plugins. [11]

README latency claims include sub-microsecond determinism and several microsecond execution targets. The review did not establish those numbers with benchmarks. Publish the target machine, build profile, input dimensions, warmup, sample count, allocation observations, and latency distribution alongside each measured claim. Keep design targets explicitly separate from observed measurements. [1]

The new supervision component landed during this review and received only a brief inspection. Its public budget fields include restart windows and affinity, while the inspected tick loop does not enforce those fields. It reads `Instant::now`, uses a HashMap, and allocates status results. Those choices can be acceptable for a control-plane supervisor, but not as evidence of a pure, non-allocating reducer. Treat this as a follow-up review area, not a completed assessment of the new commit. [13]

## Verification results

| Required check | Observed result | Interpretation |
|---|---|---|
| `cargo check --workspace --all-targets` | Passed, with warnings | Compilation only; workspace changed during review |
| `cargo test --workspace` | Failed, exit 101 | `test_wasm_enzyme_exists` requires a missing compute WASM artifact; remaining suites were not all reached |
| `cargo run -p ast_auditor -- audit core/ crates/` | Passed, 724 files, zero reported violations | Also emitted a parse error; coverage limitations above apply |
| Required negated `git grep` | Failed | Matches included source-analysis strings, comments, and negative test fixtures; not proof of executable stubs |
| `cargo test -p emulator_harness` | Passed | Two unit tests; zero doc tests |
| Retired shell wrapper | Could not start, exit 1 | Historical WSL `BasePath` configuration error; replaced by `ast_auditor verify`. |

The missing fixture was `data/extensions/wasm/compute_worker/target/wasm32-unknown-unknown/release/compute_enzyme.wasm`. Make its build/setup an explicit dependency of the integration workflow, or classify the integration test with a documented prerequisite. Do not silently skip the failure while reporting complete coverage. [14]

The working tree contained changes before review and gained a tracked model-manifest change during testing/concurrent activity. No automatic reset was performed because authorship cannot be safely inferred in a shared, changing workspace. Verification logs are retained beside this report.

## Recommended next milestone

1. Fix approval-cache identity, snapshot publication/slot reuse, and WAL tail recovery. Each fix should have a focused failure reproduction and regression test.
2. Make one canonical verification command authoritative in local workflows and CI. Fail on incomplete auditing, use actual negative test cases, and make integration fixtures reproducible.
3. Select one small end-to-end component path: injected input, pure reducer, bounded output, replay test, allocation measurement, and a measured execution budget. Use it as the standard for subsequent components.
4. Separate kernel contracts from control-plane adapters in manifests and public APIs. Define lifecycle and compatibility rules for plugins and supervision.
5. Rewrite assurance and performance claims around what the gates and measurements actually establish.

The framework's strongest asset is its explicit architectural intent. The highest-value work now is making a small set of guarantees true, testable, and difficult to bypass.

## Sources

Repository sources below were inspected locally on September 14, 2026. Line references identify the reviewed implementation; concurrent changes may move them.

1. [README](../../README.md), [constitution](../../AGENTS.md), and [master architecture](../architecture/MASTER_ARCHITECTURE.md).
2. [Workspace manifest](../../Cargo.toml), [hypervisor manifest](../../core/hypervisor/Cargo.toml), and [hypervisor library](../../core/hypervisor/src/lib.rs).
3. [Emulator reducer](../../dev/emulator_harness/src/reducer.rs) and [tests](../../dev/emulator_harness/src/lib.rs).
4. [WAL implementation](../../crates/ipc_bus/src/persistent_wal.rs), especially replay and append opening; [crash recovery tests](../../crates/ipc_bus/tests/wal_crash_recovery.rs).
5. [Action interlock](../../crates/governance/src/smt_action_interlock.rs), key at lines 48–52, cache lookup around 182–207.
6. [Lattice verifier](../../crates/governance/src/lattice_verifier.rs), allocation bounds around 169; [mutation caller](../../crates/adaptation_engine/src/mutation.rs), around 152.
7. [Snapshot ring](../../crates/ipc_bus/src/swmr_shm.rs), publisher 792–823 and reader 893–941; [HUD adapter](../../crates/studio_hud/src/state_snapshot.rs).
8. [Auditor](../../crates/ast_auditor/src/lib.rs), [allocation visitor](../../crates/ast_auditor/src/rules/zero_alloc_hot_path.rs), and [evasion tests](../../crates/ast_auditor/tests/agent_evasion_suite.rs).
9. [Native verifier](../../crates/ast_auditor/src/verification.rs) and [CI](../../.github/workflows/ci.yml).
10. [Prover implementation](../../crates/governance/src/z3_prover.rs) and [feature configuration](../../crates/governance/Cargo.toml).
11. [Plugin API](../../crates/plugin_api/src/lib.rs) and [dynamic loader](../../crates/hotload/src/lib.rs).
12. Rust standard library, [atomic fence documentation](https://doc.rust-lang.org/std/sync/atomic/fn.fence.html), especially “Mandatory Atomic,” accessed September 14, 2026.
13. [Supervision component](../../crates/orchestrator/src/supervision.rs), introduced during review.
14. [WASM integration assertion](../../core/hypervisor/tests/test_e2e_pipeline.rs), line 235; the workspace test, audit, and harness logs were retained in the original review workspace and are not versioned in this repository.
15. GitHub, [initial reviewed commit](https://github.com/Aarogaming/aaroneous/commit/4c78cfe1861b55c893450f2e8b093ff7c6c4bf0b), September 14, 2026.
