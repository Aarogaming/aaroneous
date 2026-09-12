# Forensic Ingestion Protocol (RFC-0005)

> **TIER 2 PROTOCOL SPECIFICATION**  
> **SCOPE**: Forensic Ingestion, Quarantine Isolation, Kernel Extraction, and Regression Codification.  
> **APPLIES TO**: All external code staged under `dev/legacy_staging/`.

---

## 1. Overview & Objective

RFC-0005 defines the invariant-preserving ingestion methodology for external code bases, historical prototypes, and third-party modules. Legacy systems staged under `dev/legacy_staging/` must undergo rigorous isolation, trace-driven state extraction, and negative contract codification before any code can be grafted into production workspace crates.

---

## 2. Six-Step Ingestion Workflow

When processing legacy modules staged in `dev/legacy_staging/`:

```
Legacy Code -> Stage (dev/legacy_staging/)
    |
Emulator Harness Trace (dev/emulator_harness)
    |
Forensic Triage (Kernel Extraction / Pathology Analysis)
    |
Synthesis (crates/<target> via zero-copy fixed buffers)
    |
Codification (docs/forensics/ + Cratify rules + negative tests)
```

### Step 1: Containment
- **DO NOT** refactor, modernize, or patch legacy code in place.
- Keep the legacy codebase strictly isolated within `dev/legacy_staging/<artifact_id>/`.
- Never import or link legacy staging artifacts directly into production workspace dependencies.

### Step 2: Trace & Isolate
- Route legacy execution through `dev/emulator_harness` to extract 40-byte `TraceEvent` streams:
  ```rust
  use emulator_harness::{TraceEvent, extract_state_delta};
  
  let events = /* capture from emulator */;
  let delta = extract_state_delta(&events, target_addr)?;
  ```

### Step 3: Kernel Extraction
- Use `extract_state_delta` or AST signature extraction (`orchestration_plane::domain_classifier`) to isolate the minimal deterministic state transition needle.
- Strip all non-deterministic logic, ambient environment lookups, dynamic heap allocations, and panic hooks.

### Step 4: Failure Analysis
- Document anti-patterns and pathology findings in `docs/forensics/<id>_<name>.md`.
- Detail violated workspace invariants (e.g., ambient authority, unbounded heap usage, prefix stutter).

### Step 5: Synthesis
- Rebase the clean, certified kernel into `crates/<target>/` using zero-copy fixed buffers and stack-allocated data structures.
- Ensure boundary types implement `#[repr(C)]` and derive `Pod` / `Zeroable`.

### Step 6: Codification
- Record a post-mortem autopsy report.
- Add an automated Cratify rule in `crates/cratify/src/rules/` to prevent recurrence.
- Write an executable negative contract test in `tests/negative_contracts/` verifying that the anti-pattern is actively rejected.

---

## 3. Quarantine Isolation & Ingestion Gates

- **Quarantine Buffer**: Modules with unhandled ambient risks (`std::env::*`, direct file system queries) are routed strictly to `staging/quarantine/<module>.rs`.
- **Grafting Prohibition**: Live grafting to domain crates is prohibited if unhandled ambient risks > 0.
- **Verification Gate**: The ingested module must pass `cargo run -p cratify -- audit core/ crates/ dev/` with exit code 0.
