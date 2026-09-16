# Agent Integration Guide: 3-Tier Governance Pipeline

## Overview

This repository uses a **three-tier agent delivery pipeline** to ensure local coding models (OpenCode, Cursor, Claude, etc.) maintain full context of architectural invariants, failure modes, and enforcement mechanisms.

## The Three Tiers

```
┌─────────────────────────────────────────────────────────────────┐
│                   TIER 1: CONTEXT ANCHOR                         │
│   File: AGENTS.md                                                │
│   Purpose: Core invariants, banned patterns, mandatory rules    │
│   How Agent Uses It: Reads first on every session start        │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│              TIER 2: PROCEDURAL REFERENCE                        │
│   File: docs/rfc/RFC-0005-FORENSIC-INGESTION.md                 │
│   Purpose: Exact execution lifecycle for legacy ingestion       │
│   How Agent Uses It: Consults when tasked with forensic work   │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│              TIER 3: MECHANICAL BOUNCER                          │
│   File: scripts/ast_auditor verify                                   │
│   Purpose: Instant local feedback loop (compile, audit, test)   │
│   How Agent Uses It: Runs after every major change              │
└─────────────────────────────────────────────────────────────────┘
```

## Tier 1: Context Anchor (AGENTS.md)

**Location**: Repository root (`AGENTS.md`)

**What Agents See First**: Every local coding model is tuned to search for and ingest root-level system rules before executing tools. AGENTS.md contains:

- **Foundational Mandates**: Toolchain requirements, zero-heap hot paths, memory geometry
- **Banned Anti-Patterns**: Manual Pod impls, static mutexes, unwrap usage, todo!() stubs
- **Forensic Protocol**: RFC-0005 ingestion lifecycle (containment → trace → kernel → synthesis)
- **Enforcement Gate**: Mandatory verification steps before reporting completion

**Agent Behavior**: Models will refuse to generate code that violates these explicit bans.

## Tier 2: Procedural Reference (RFC-0005)

**Location**: `docs/rfc/RFC-0005-FORENSIC-INGESTION.md`

**When Agents Consult It**: When tasked with legacy code ingestion or forensic analysis.

**What's Inside**:
1. Containment protocol (`dev/legacy_staging/`)
2. Trace capture via `dev/emulator_harness` (40-byte POD TraceEvents)
3. Algorithmic kernel extraction methodology
4. Failure mode codification into negative tests
5. Cratify certification workflow

**Golden Reference**: Point agents directly to `dev/emulator_harness/src/reducer.rs` as the canonical example of zero-allocation state processing.

## Tier 3: Mechanical Bouncer (ast_auditor verify)

**Location**: `scripts/ast_auditor verify`

**What It Does**: Instant local feedback loop that "slaps the agent's hand" if it tries shortcuts.

**Verification Steps**:
1. `cargo check --workspace --all-targets` - Workspace compilation
2. `cargo run -p cratify -- audit` - Invariant enforcement
3. `git grep` for stubs and banned patterns - Production safety
4. `cargo test -p emulator_harness` - Core functionality

**Agent Behavior**: Script runs automatically after every major change. If any check fails, agent must fix before continuing.

## Agent Workflow (Best Practices)

### When Starting a Session

```bash
# 1. Read AGENTS.md to understand constraints
cat AGENTS.md | less

# 2. Check current status
cargo run -p ast_auditor -- verify
```

### When Modifying Core Crates

```bash
# Make changes...

# 3. Verify with mechanical bouncer
cargo run -p ast_auditor -- verify

# If any check fails, fix and re-run until all pass
```

### When Processing Legacy Code

```bash
# 1. Consult RFC-0005 for ingestion protocol
cat docs/rfc/RFC-0005-FORENSIC-INGESTION.md

# 2. Stage in dev/legacy_staging/ (do NOT modify in place)

# 3. Trace via emulator_harness
cargo run -p emulator_harness

# 4. Document in docs/forensics/

# 5. Verify with ast_auditor verify
cargo run -p ast_auditor -- verify
```

## Golden References for Agents

| Task | Reference File |
|------|----------------|
| Zero-allocation state processing | `dev/emulator_harness/src/reducer.rs` |
| Forensic methodology | `docs/rfc/RFC-0005-FORENSIC-INGESTION.md` |
| Case study template | `docs/forensics/0001_aas_omni_galaxy_view.md` |
| Negative contracts | `tests/negative_contracts/test_omni_anti_patterns.rs` |

## Active Immune System Components

The agent operates within a fully instrumented immune system:

- **Agent Evasion Detection**: Catches `.unwrap()`, `Mutex`, `todo!()` in production
- **Chaos Injector**: Tests recovery from CPU starvation, buffer overflow, thread storms
- **IPC Fuzzer**: Validates POD guards against malformed data and hardware faults
- **RLS Boundary Tests**: Ensures math engines remain bounded under numerical stress
- **Cache Line Isolation Benchmark**: Quantifies padding ROI

## Version

**Phase 38 Immune Ledger Active** - Agent governance fully operational.
