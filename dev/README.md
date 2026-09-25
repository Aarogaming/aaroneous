# Development Harnesses & Fixtures (`dev/`)

`dev/` holds test harnesses, fixtures, and a small amount of retained reference material. It is not a
source of governance: binding rules live in [AGENTS.md](../AGENTS.md) and
[docs/CRATIFY_SPEC.md](../docs/CRATIFY_SPEC.md).

## Contents

| Path | Purpose |
|---|---|
| [`emulator_harness/`](emulator_harness/) | Golden dogfooding harness (workspace member, `kernel` profile). Run by gate 8: `cargo test -p emulator_harness`. |
| [`chaos_injector/`](chaos_injector/) | Synthetic stress tester reproducing historical duty-cycle failure modes. |
| [`canary_legacy_fixture/`](canary_legacy_fixture/) | Deliberately non-compliant legacy module used to exercise Cratify certification. |
| `legacy_staging/` | Local-only (git-ignored) staging sandbox for external code awaiting onboarding; see [component onboarding](../docs/architecture/component_onboarding_specification.md). |
| [`tools/protocol/linking_protocol_spec.json`](tools/protocol/linking_protocol_spec.json) | Machine-readable schema for inter-program tensor and command packets. |
| [`tools/auditors/`](tools/auditors/) | Reference prompts for resilience and systems-health audits. |
| `tools/auto_wrapper/generated/`, `tools/omni/output/` | Retained generated output from the transferred tooling (historical). |

## Transferred tooling

The developer scripts that used to live under `dev/tools/` (diagnostics, maintenance, runners, the
Omni projector, and the auto-wrapper generator) moved to the private companion workspace on
2026-09-16 as part of the tooling separation. Aaroneous never depends on them to build, test, release,
or run. Historical design documents formerly indexed here are in [docs/archive/](../docs/archive/).
