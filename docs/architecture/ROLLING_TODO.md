# Rolling Architecture Worklist

This is the single working backlog for gradual hardening of Aaroneous. Update it from
repository evidence, CI results, and completed verification; do not add speculative work.

## Operating rules

- Complete one bounded item at a time. Preserve a passing, reviewable commit after each item.
- Treat the canonical verification gate and cross-platform CI as release evidence.
- Move an item to **Done** only when its acceptance criteria and required checks are recorded.
- Add newly discovered issues with source paths, a concrete failure mode, and a priority.
- Do not treat a green syntax audit as allocator, timing, race, or ABI proof.

## Active

### A1 — Merge the text-encoding and boundary-audit branch

**Status:** Waiting for GitHub CI result  
**Branch:** `codex/text-encoding-integration`  
**Evidence:** strict encoding check, formatting, structural audit, and emulator harness pass
locally. The rebased cross-platform CI run is [35047740597](https://github.com/Aarogaming/aaroneous/actions/runs/35047740597).

**Done when:**

- Linux and Windows CI pass for the rebased branch.
- A pull request is reviewed with the mechanical text changes and the small Rust repair
  identified separately.
- The branch is merged to `main`.

### A2 — Establish a golden bounded reducer path

**Status:** Ready  
**Scope:** Start from `dev/emulator_harness`; extract or define the smallest reusable
execution-host contract without moving unrelated subsystems.

**Done when:**

- The reducer takes caller-provided input and storage, produces typed output, and has no
  ambient reads.
- Fixed trace replay proves deterministic output.
- An allocator-instrumented test measures the reduction call.
- The path has a documented execution budget and Linux/Windows CI coverage.

## Planned

### B1 — Enforce execution-host dependency policy

**Depends on:** A2  
Create a package-level policy that prevents the execution host from gaining HTTP, browser,
GUI, database, model, filesystem-watch, GPU, network, or asynchronous-runtime dependencies.

### B2 — Narrow the hypervisor public surface

**Depends on:** A2, B1  
Keep hypervisor binaries as composition roots. Replace broad library re-exports with narrow
traits and typed frames owned by lower layers.

### B3 — Extract bounded primitives from mixed crates

**Depends on:** B1  
Move only proven scan-path primitives from `compute` and `ipc_bus`. Keep persistence,
model loading, OS integration, and scheduling outside the execution-host dependency closure.

### B4 — Define the plugin lifecycle contract

Write an RFC that distinguishes the current in-process Rust trait from a future dynamic ABI
or process boundary. Cover compatibility, ownership, startup, shutdown, and failure isolation.

### B5 — Publish measured assurance claims

Replace unmeasured latency and verification claims with reproducible benchmark reports:
machine, profile, input dimensions, warmup, sample count, allocation observation, and latency
distribution.

## Done

- Correctness and verification repairs recorded in [REVIEW_FIXES.md](../REVIEW_FIXES.md).
- Strict UTF-8 text contract and safe normalizer are implemented on
  `codex/text-encoding-integration`.
- Kernel boundary evidence and staged extraction plan are recorded in
  [KERNEL_BOUNDARY_AUDIT.md](KERNEL_BOUNDARY_AUDIT.md).

## Scheduled maintenance checklist

On each scheduled check:

1. Read this file and the latest CI outcome for the active branch.
2. If CI fails, add the failing check, root cause, and next bounded repair under **Active**.
3. If CI passes and a task is ready, move only the next actionable item to **Active**.
4. Commit and push a list update only when its status or evidence materially changed.
5. Notify only for a CI completion, failure, blocked item, or completed work item.

