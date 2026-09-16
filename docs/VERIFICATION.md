# Verification System

Aaroneous keeps its validation system inside the framework repository. The private dev-tools
repository may assist a developer, but it is never required for validation.

| Layer | Repository-owned mechanism |
| --- | --- |
| Structural invariants | `cargo run -p ast_auditor -- audit core/ crates/ dev/emulator_harness/` |
| Hardening policy and inventory | `cargo run -p ast_auditor -- hardening --check` |
| Encoding contract | `python scripts/check_text_encoding.py` until its Rust replacement is complete |
| Functional and integration tests | `cargo test --workspace` |
| Golden reducer evidence | `cargo test -p emulator_harness` |
| Canonical local and CI gate | `cargo run -p ast_auditor -- verify` |
| Broader review scopes | [Systems health](audits/SYSTEMS_HEALTH.md) and [advanced resilience](audits/ADVANCED_RESILIENCE.md) |

The canonical gate and CI provide the enforceable baseline. The review profiles define additional
evidence required before making assurance claims beyond that baseline.
