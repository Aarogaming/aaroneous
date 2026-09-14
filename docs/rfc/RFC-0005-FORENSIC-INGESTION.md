


## Workflow Diagram
Legacy Code -> Stage (dev/legacy_staging/) 
    |
Emulator Harness Trace (dev/emulator_harness)
    |
Forensic Triage (Kernel / Failure)
    |
Synthesis (crates/<target>)
    |
Codification (docs/forensics/ + cratify rules + negative tests)


## References
- RFC-0001: Zero-Allocation Hot Path Constraints
- RFC-0003: Cratify Governance Invariants  
- Phase 38 Immune Ledger Specification

