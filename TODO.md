# Aaroneous Master Roadmap & Priorities Redirect

> **CANONICAL ROADMAP LOCATION**: See [docs/roadmap.md](docs/roadmap.md) for the authoritative, evidence-labeled product roadmap, pillar status, and release schedule.
> **DEPRECATION POLICY**: See [docs/DEPRECATION_POLICY.md](docs/DEPRECATION_POLICY.md) for the v0.3.3 alias deprecation schedule and removal checklist.
> **HISTORICAL ARCHIVE**: Completed phases (Phases 1–37) and historical defect audits are preserved in `docs/archive/stale_sprawl/blueprints/COMPLETED_PHASES_ARCHIVE.md`.

---

## Terminology Reference (Canonical)

| Concept | Technical Term | Crate / Location | Status |
|---------|---------------|------------------|--------|
| **Aaroneous CLI** | Supervisory Control Daemon & SI CLI | `hypervisor` (`core/hypervisor`) | **Implemented** |
| **Capabilities** | Domain Task Engines & MCP Tools | `capabilities` (`crates/capabilities`) | **Implemented** |
| **Orchestrator** | Task Scheduler & Core Affinity | `orchestrator` (`crates/orchestrator`) | **Implemented** |
| **Platform Bridge** | Win32 HID, DXGI Screen Capture, WASAPI | `platform_bridge` (`crates/platform_bridge`) | **Implemented** |
| **Studio HUD** | Desktop Cockpit & Telemetry UI | `studio_hud` (`crates/studio_hud`) | **Implemented** |
| **Adaptation Engine** | Polyglot AST Transpiler & Mutation | `adaptation_engine` (`crates/adaptation_engine`) | **Implemented** |
| **IPC Bus** | Lock-Free SPMC/SWMR Ring Buffers & WAL | `ipc_bus` (`crates/ipc_bus`) | **Implemented** |
| **Compute Substrate** | `.si` Containers + SSM Recurrence + JIT | `compute` (`crates/compute`) | **Implemented** |
| **Omni Galaxy** | 3D Spatial Knowledge Graph Index | `omni` (`crates/omni`) | **Implemented** |
| **Adaptation Plane** | Closed-Loop Adaptive Control | `adaptation_plane` (`crates/adaptation_plane`) | **Implemented** |
| **Resource Governance** | Hardware Thermals & Budget | `governance` (`crates/governance`) | **Implemented** |

---

*Refer to [docs/roadmap.md](docs/roadmap.md) for active and future milestone tracking.*
