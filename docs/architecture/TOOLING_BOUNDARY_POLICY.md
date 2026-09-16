# Tooling Boundary Policy

## Purpose

Aaroneous is a Rust component framework, not the development environment that builds,
operates, or experiments on it. Product code must not become a dependency sink for personal
automation, local-agent orchestration, model prompts, provisioning, or research prototypes.

## Product repository boundary

This repository contains only artifacts required to define, build, test, release, deploy, or
operate Aaroneous as a reproducible product:

- Rust workspace crates, tests, auditors, and project commands.
- Rust-owned runtime interfaces, including active WGSL shader sources and active WIT contracts.
- Declarative CI, deployment, and configuration files required to reproduce supported builds.
- Documentation, schemas, fixtures, and versioned compatibility contracts.

New repository-management, verification, migration, or release logic must be implemented in
Rust by default. A non-Rust script is permitted only when it is a thin platform or CI adapter
that cannot reasonably be expressed by a Rust command. Its README or source header must name
the Rust command or platform capability it wraps.

## Private dev-tools boundary

The private companion repository is [Aarogaming/aaroneous-devtools](https://github.com/Aarogaming/aaroneous-devtools).
It is intentionally not a submodule or workspace member of Aaroneous.

A separate private dev-tools repository may contain:

- Workstation provisioning, local-model runners, prompts, agent profiles, and private secrets.
- One-off recovery tools, experimental data harvesters, dashboards, and research prototypes.
- Cross-project operational automation and machine-specific launchers.

It may invoke documented Aaroneous commands, but Aaroneous must not require it to build, test,
run CI, release, or execute production components. It must not be added as a Git submodule,
workspace member, build dependency, runtime dependency, or implicit local prerequisite.

## Self-hosting guardrails

- Product crates may not invoke agent runners, code-generation services, or personal toolchains
  as part of normal execution.
- Autonomous adaptation features must operate through explicit, typed capability boundaries and
  may not write source, alter build configuration, or replace binaries without a separately
  reviewed control-plane protocol.
- Experimental tooling may consume product telemetry and public formats, but it may not define
  the product's authoritative safety, compatibility, or release decisions.
- Runtime behavior must remain reproducible from the public repository and declared inputs.

## Migration rule

Existing Python, PowerShell, and shell scripts are migration candidates. Each must be classified
as one of:

1. A Rust command to port into the workspace.
2. A thin platform wrapper retained with a documented reason.
3. A private dev-tools concern to remove from this repository.
4. A generated or fixture artifact to retain only when an active contract requires it.

No new general-purpose scripting stack may be introduced while this inventory is incomplete.

## Enforcement

The native hardening gate verifies that this policy is present. Changes that add non-Rust
automation, development dependencies, or self-hosting behavior require an explicit update to
this policy, the hardening matrix, and the canonical worklist.
