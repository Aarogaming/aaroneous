# Aaroneous Architecture Specification


## Version
**1.7.0** - Phase 38 Immune Ledger Active


## System Invariants

### Hot-Path Memory Model
All critical paths must use zero-allocation patterns:
- Stack-allocated arrays ([T; N]) over heap vectors
- bytemuck::Pod types for lock-free transmission
- Lock-free SWMR buffers for telemetry and state

### Type Safety Contract
```rust
// All shared structures must:
#[repr(C)]           // Fixed layout, ABI-compatible
#[derive(Pod, Zeroable)]  // Safe zero-copy serialization
pub struct MyType { ... }
```

### Concurrency Model
- Single-writer: Static buffers with atomic indices
- Multi-reader: Immutable references only
- No mutexes in telemetry hot paths


## Phase 38: Immune Ledger

The Immune Ledger transforms legacy failures into permanent enforcement barriers.

### Three Synchronized Components

1. **Ingestion RFC** (docs/rfc/RFC-0005-FORENSIC-INGESTION.md)
   - Standard operating procedure for legacy code assimilation
   - Defines containment, triage, synthesis, and codification workflow


2. **Forensic Record Schema** (docs/forensics/)
   - Versioned autopsy reports for each dissected module
   - Documents provenance, pathology, rebase, and invariants


3. **Negative Knowledge Ledger** (tests/negative_contracts/)
   - Concrete tests proving legacy failures are blocked
   - Regression guards ensuring anti-patterns cannot recur


### Verification Loop

bash
# Step 1: Verify forensics entries have corresponding tests
for report in docs/forensics/*.md; do
    artifact=$(basename "$report" .md)
    if ! grep -q "test_${artifact}" tests/negative_contracts/*.rs; then
        echo "MISSING TEST: $artifact"
        exit 1
    fi
done

# Step 2: Run Cratify audit across all trees
cargo run -p cratify -- audit crates/ core/ dev/

# Step 3: Workspace compilation
cargo check --workspace
```


## Directory Structure

docs/
├── ARCHITECTURE.md                  # This file
├── rfc/
│   └── RFC-0005-FORENSIC-INGESTION.md  # Ingestion methodology
└── forensics/
    ├── README.md                    # Index of case studies
    ├── 0001_aas_omni_galaxy_view.md # Case study: Omni knowledge graph
    └── 0002_<name>.md              # Future case studies

tests/
└── negative_contracts/
    └── test_omni_anti_patterns.rs   # Regression tests from case studies

dev/
├── emulator_harness/                # Trace-driven state extraction
└── legacy_staging/                  # Isolated legacy analysis sandbox


## References

- RFC-0001: Zero-Allocation Hot Path Constraints
- RFC-0003: Cratify Governance Invariants
- Phase 38 Specification: Immune Ledger & Negative Knowledge

