# Systems Health Audit Profile

Use this profile for a bounded repository review of structural hygiene and architectural
correctness. It supplements automated gates; it does not replace them.

## Scope

1. **Maintainability and architectural decay:** identify oversized modules, duplicated logic,
   obsolete desktop assumptions, and brittle abstractions.
2. **Specification alignment:** compare active implementation boundaries with approved component,
   plugin, and execution-host contracts.
3. **Static safety:** review error propagation, parsing boundaries, concurrency ownership, and
   results from `ast_auditor`.
4. **Dependency hygiene:** inspect the dependency graph, enabled features, and reproducible
   vulnerability reports.

## Required evidence

Record the audited revision, commands and their exit status, affected paths, concrete failure
mode, and a bounded next action in `docs/WORKLIST.md`. Historical or aspirational documents are
not proof that a current defect exists.

## Automated foundation

Run the canonical gate and native auditor first:

```text
bash scripts/agent_check.sh
cargo run -p ast_auditor -- hardening --check
```
