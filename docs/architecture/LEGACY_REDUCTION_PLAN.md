# Legacy Reduction Plan

This plan records the evidence-based path from the inherited mixed-language and metaphor-heavy
surface to a Rust component framework. It is a classification record, not a claim that every
legacy concept is active or correct.

## Completed removals

- Local and cloud agent clients, autonomous host-editing loop, hard-coded tensor indexer, and
disconnected Python SDK/extension stubs were transferred to the private dev-tools archive on
2026-09-16. No active repository source referenced them.
- The canonical verification gate is `ast_auditor verify`; PowerShell and shell gate wrappers
were removed.

## Rust migration candidates

| Concern | Current form | Required Rust outcome |
| --- | --- | --- |
| Text encoding inventory | `scripts/check_text_encoding.py` | An `ast_auditor` subcommand with tracked-file, attribute, and byte-level validation. |
| Encoding normalizer | `scripts/normalize_text_encoding.py` | Retire after the Rust checker is established; do not add a general script runtime. |
| Immune-system smoke test | `scripts/verify_immune_system.sh` | Native test target or `ast_auditor` subcommand. |
| Windows installation and packaging | `deploy/*.ps1` | A reviewed Rust release/install command with explicit Windows APIs and reversible operations. |

## Interface review required before any removal

- WGSL shaders are runtime GPU interfaces and must be traced from their Rust loaders.
- WIT contracts must be split into active external compatibility contracts and the unreferenced
  fabrication catalogue before retirement or native replacement.
- Terraform, Helm, and CI files are declarative deployment/build contracts. They are not general
  application code and require a deployment redesign before replacement.

## Terminology reduction

The active tree still contains mythology and biological metaphors in registry files, documentation,
and public Rust identifiers: for example `hox_*`, `relic`, `specialist`, `genome`, `organ`,
`hive`, `assimilation`, and `symbiotic`.

These cannot be mechanically renamed because many are serialized values or public APIs. Each
rename must start with a compatibility inventory, select a systems term, update serializers and
tests together, and provide a migration rule for stored records. The first bounded target is the
unreferenced registry/configuration catalogue, followed by leaf Rust identifiers. No product
contract should use metaphorical naming where a standard systems name expresses the role.
