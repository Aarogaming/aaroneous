# Hardening Audit Matrix

This matrix turns repository hardening into bounded, evidence-based audits. It is not a
claim that every perspective has been reviewed. A lens is **Complete** only when its
evidence, findings, remediation, and verification are recorded.

## How to use this matrix

- Audit one lens or a small related group at a time.
- Record source paths, commands, tool versions, and immutable commit identifiers.
- Classify findings as Critical, High, Medium, Low, or Informational.
- Add confirmed work to [WORKLIST.md](WORKLIST.md); do not create a second backlog here.
- Re-audit after dependency, boundary, or deployment changes.

## Priority 1 — Trustworthy execution

| ID | Lens | Current evidence | Next acceptance criterion |
|---|---|---|---|
| H01 | Build, tests, and CI reproducibility | Canonical gates exist; current branch awaits cross-platform CI evidence. | A frozen commit passes the same required gate locally and on Linux and Windows. |
| H02 | Dependency and supply-chain risk | GitHub reports dependency advisories; no repository-owned advisory report exists. | Produce a lockfile-aware report with dependency paths, advisory IDs, and remediation decisions. |
| H03 | Unsafe Rust and FFI boundaries | The review source identifies a trait-object FFI boundary; current location differs from the historical claim. | Inventory all unsafe blocks and exported ABI symbols; prove ownership and free-function symmetry. |
| H04 | Panic, error, and recovery | Review fixes cover WAL recovery and selected error paths. | Exercise malformed input, failed I/O, poisoned synchronization, and restart paths without process aborts. |
| H05 | Concurrency and atomic publication | Snapshot transport has a documented protocol and focused repairs. | Forced-interleaving tests cover publication, reuse, reader lifetime, and optimized builds. |
| H06 | Allocation and bounded-memory hot paths | Syntax auditor covers annotated code; emulator test is not allocator-instrumented. | Measure allocations for the golden reducer and enforce a bounded-storage contract. |
| H07 | Determinism and ambient authority | Constructor injection and path policies exist. | Inventory clocks, randomness, environment reads, and filesystem access in selected execution paths. |
| H08 | IPC, WAL, and persistence recovery | WAL tail recovery and snapshot transport fixes are recorded. | Property and crash-recovery tests cover append, corruption, version migration, and capacity limits. |

## Priority 2 — Boundary protection

| ID | Lens | Current evidence | Next acceptance criterion |
|---|---|---|---|
| H09 | Input validation and serialization | Typed wire and container crates exist. | Fuzz or property-test parsers and boundary messages with size, version, and malformed-data limits. |
| H10 | Filesystem containment | The review addressed a path-handling finding; legacy sources remain to verify. | Test path normalization and sandbox boundaries against traversal, symlinks, and Windows-specific forms. |
| H11 | Authentication, authorization, and secrets | Historical audit names a timing-risk claim that needs reproduction. | Inventory secret comparison and authorization decisions; verify constant-time or otherwise appropriate handling. |
| H12 | Plugin lifecycle and ABI | Current plugin model is an in-process Rust trait, not a stable dynamic ABI. | Publish lifecycle contract and test load, failure, ownership, unload, and compatibility boundaries. |
| H13 | Network protocol and remote input | HTTP, MCP, P2P, and adapter dependencies are present. | Define threat model, authentication boundary, rate limits, and malformed-frame test coverage per exposed protocol. |
| H14 | OS and platform abstraction | Linux and Windows CI exist; local native toolchain differs from hosted CI. | Test platform adapters behind explicit capability/configuration boundaries on each supported OS. |
| H15 | Resource limits and denial of service | Some record and restart limits exist. | Establish size, queue, retry, timeout, and concurrency budgets with overload tests. |

## Priority 3 — Operational assurance

| ID | Lens | Current evidence | Next acceptance criterion |
|---|---|---|---|
| H16 | Observability, audit trails, and recovery | Flight-recording and telemetry concepts exist. | Verify incident events can be correlated, bounded, redacted, retained, and replayed. |
| H17 | Public API compatibility | Crates expose broad surfaces and compatibility is not centrally tracked. | Inventory public APIs and define semver, feature, message-version, and deprecation policy. |
| H18 | Architecture and dependency direction | Kernel boundary audit documents direct Ring 0 dependency violations. | Introduce a minimal execution-host reference path and a dependency-policy check. |
| H19 | Documentation and operational accuracy | Worklists and root documentation are consolidated. | Link every active operational claim to current evidence and remove or qualify unmeasured assertions. |
| H20 | Performance evidence and regressions | Architecture documents contain targets; reproducible measurements are not yet standardized. | Publish benchmark harness, machine profile, allocation observation, and latency distribution for the golden path. |

## Audit record template

```text
Lens: Hxx
Commit:
Scope:
Tools and versions:
Evidence:
Findings:
Risk classification:
Remediation:
Verification:
Follow-up:
```

