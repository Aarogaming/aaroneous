# Development components

This directory holds Rust development crates that test framework behavior under explicit,
reproducible inputs:

- `emulator_harness/` is the golden deterministic reducer harness and is part of the canonical
  verification gate.
- `chaos_injector/` and `canary_legacy_fixture/` are bounded test fixtures.

Workstation automation, local-model runners, agent profiles, experimental project analysis, and
host diagnostics are maintained in the private
[Aarogaming/aaroneous-devtools](https://github.com/Aarogaming/aaroneous-devtools) companion
repository. They are not framework build, test, runtime, or release dependencies.
