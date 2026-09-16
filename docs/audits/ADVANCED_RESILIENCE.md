# Advanced Resilience Audit Profile

Use this profile to extend the automated validation system with evidence that static analysis
cannot provide on its own. It is a review protocol, not an autonomous code-modification tool.

## Scope

1. **Test and state-transition rigor:** identify critical parsers and reducers that need fixed
   trace replay, property tests, or mutation testing.
2. **Hot-path memory and binary efficiency:** verify claimed bounded paths with allocation and
   latency measurements; inspect copies, formatting, growth collections, and blocking behavior.
3. **Build and documentation quality:** evaluate release-profile settings, documentation links,
   compatibility contracts, and reproducible build assumptions.
4. **Supply-chain hygiene:** inspect license and advisory reports with dependency paths and
   recorded mitigations.

## Required evidence

Every finding needs the audited revision, a reproducible command or test, the affected code
path, and an acceptance criterion. Static syntax findings must not be presented as proof of
transitive allocation, timing, race freedom, ABI compatibility, or formal verification.

## Automated foundation

Start from the native audit and canonical gate:

```text
cargo run -p ast_auditor -- audit core/ crates/ dev/emulator_harness/
cargo run -p ast_auditor -- hardening --check
cargo run -p ast_auditor -- verify
```
