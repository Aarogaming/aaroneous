# Forensic Case Studies Index

This directory contains post-mortem analyses of legacy code modules that were 
dissected, diagnosed, and rebased into the certified Aaroneous substrate.

## Purpose

Every entry documents:
1. **Theoretical Intent** - What the code was trying to achieve mathematically
2. **Pathology Analysis** - Why it failed in practice (anti-patterns identified)  
3. **Rebase Implementation** - How the kernel was extracted and certified
4. **Codified Invariants** - Cratify rules and negative tests that prevent recurrence

## Workflow

When analyzing a legacy module:

1. Stage in `dev/legacy_staging/<artifact_id>/`
2. Run through `dev/emulator_harness` for trace analysis
3. Document findings here following the template below
4. Link to corresponding negative test in `tests/negative_contracts/`

## Template

See `0001_aas_omni_galaxy_view.md` for a complete example.


## Case Studies

| ID | Module | Origin | Status | Negative Test |
|----|--------|--------|--------|---------------|
| 0001 | AAS Omni Galaxy View | Legacy Python Suite | Rebased | `test_omni_nan_saturation` |





## Verification Checklist

Before committing any forensic documentation:

- [ ] Case study follows template structure (Provenance, Intent, Pathology, Rebase, Invariants)
- [ ] Corresponding negative test exists in `tests/negative_contracts/`
- [ ] Cratify audit passes for all modified crates
- [ ] Workspace compiles without errors: `cargo check --workspace`
- [ ] All derive macros are explicit (no manual unsafe impls)


## Next Steps

1. Add more case studies to `docs/forensics/` following the template
2. Extend `tests/negative_contracts/` with additional regression tests
3. Integrate verification loop into CI/CD pipeline


