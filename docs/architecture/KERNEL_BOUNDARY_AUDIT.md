# Kernel Boundary Audit

## Purpose and scope

This audit compares the workspace dependency graph at `b6742fc` with the five-ring
topology in `architecture_overview.md`. It is evidence for a staged extraction plan;
it does not move code or change public APIs.

The architecture requires dependencies to point from higher-numbered rings toward
lower-numbered rings. It also describes the Ring 0 hypervisor as a small host for the
execution duty cycle, safety gate, and process lifecycle. The current package graph
does not establish that boundary.

## Observed boundary

`core/hypervisor` is both the composition root and a broad public facade. Its manifest
depends directly on every execution, control, ingress, and tooling concern, including
`compute`, `ipc_bus`, `governance`, `orchestrator`, `capabilities`,
`adaptation_engine`, `platform_bridge`, `llm_gateway`, `hotload`, and `plugin_api`.
It also imports external browser automation, HTTP, QUIC, GPU, database, model,
filesystem-watch, and asynchronous-runtime packages.

Its library re-exports large portions of `orchestrator`, `adaptation_engine`,
`governance`, `compute`, and `omni`. A consumer of the hypervisor library therefore
receives a control-plane and adapter surface rather than a minimal execution-host
contract.

This is a direct conflict with the topology's direction rule: Ring 0 currently depends
on Rings 1 through 3. The graph has no mechanism preventing a future Ring 1 or Ring 2
crate from importing a presentation or ingress crate either.

The presence of an expansive manifest does not mean every imported subsystem runs in a
scan. It does mean the execution host cannot currently be built, reviewed, or measured
independently of those subsystems. In particular, the `compute` and `ipc_bus` crates
also contain configuration, persistence, model, synchronization, and allocation-heavy
facilities, so they are not yet wholly equivalent to a hot-path kernel.

## Boundary classification

| Layer | Current crates or modules | Target contract |
|---|---|---|
| Data contracts | `core-contracts`, `wire`, bounded subsets of `si_ir` and `si_format` | `repr(C)` messages and validation only; no I/O, allocation, or runtime discovery in the scan path. |
| Execution primitives | bounded reducers in `compute`, shared-memory protocol in `ipc_bus` | caller-provided buffers and inputs; pure reduction or reviewed atomic publication; no filesystem, network, timer, allocator, or global reads. |
| Control plane | `orchestrator`, `governance`, `orchestration_plane`, `runtime_monitor`, `autonomic_adaptation` | owns clocks, policies, retries, persistence decisions, and schedule construction; communicates with the execution plane through typed frames. |
| Ingress and adapters | `platform_bridge`, `capabilities`, `llm_gateway`, `adaptation_engine`, `transpiler`, `mcp_server`, `omni` | acquires external input and translates it into validated frames; does not call a reducer through global state. |
| Presentation | `api`, `studio_hud`, `scratchpad` | observes snapshots and submits validated commands; no raw pointer or mutable ring-buffer access. |
| Composition | Hypervisor binaries | constructs dependencies, owns process lifecycle, and wires phases together; does not expose the broad facade as a kernel API. |

## Recommended extraction sequence

1. **Define the minimal host API.** Add a small, dependency-light execution-host crate or
   module containing only the scan input, state, output, and `step` contract. Keep the
   existing `hypervisor` package as the composition application during migration.
2. **Choose one golden path.** Move one existing bounded reducer and its Pod input/output
   types behind that API. Its test must replay fixed inputs, prove stable output, and
   measure allocations for the reduction call.
3. **Invert host dependencies.** Binaries may depend on control and adapter crates. The
   execution-host library must not. Replace direct re-exports with narrow traits and
   typed frames owned by the lower layer.
4. **Enforce the graph.** Add a dependency-policy test that fails when the execution-host
   or its hot-path crates introduce prohibited packages or upward ring dependencies.
   Start with package-level checks; only then add source-level checks for annotated
   reducers.
5. **Split mixed crates incrementally.** Extract only the bounded execution modules from
   `compute` and `ipc_bus` when a golden path needs them. Keep persistence, model loading,
   scheduling, and OS integration in control-plane or adapter packages.

## Acceptance criteria for the first extraction

- The execution-host library builds without HTTP, browser, GUI, database, model,
  filesystem-watch, GPU, network, or asynchronous-runtime dependencies.
- A binary composition root injects every clock, buffer, handle, and policy used by the
  selected path.
- The selected `step` API uses fixed or caller-owned storage and has a deterministic
  replay test plus an allocator-instrumented test.
- CI verifies the dependency policy and runs the golden-path tests on Linux and Windows.
- The public documentation calls the existing hypervisor a composition host until these
  conditions are true.

## Decisions deliberately deferred

This audit does not prescribe a dynamic plugin ABI, a global scheduler rewrite, or a
workspace-wide crate split. Those changes should follow the first proven execution path
and its dependency policy, so the framework gains a tested boundary rather than another
unverified topology diagram.

