# Portable Core Architecture

## Purpose

Aaroneous supports desktop, server, mobile, embedded Linux, RTOS, PLC, and
microcontroller deployments through one component model without forcing every
target to carry every implementation. Portability is established by separate
resolved dependency graphs, not by compiling the desktop Hypervisor everywhere.

The lowest execution contract is `scan_core`, a dependency-free, unconditionally
`no_std` crate. It owns deterministic state reduction and atomic scan commits.
It does not own storage, networking, serialization, allocation, threads,
operating-system access, device drivers, inference, or presentation.

`core-contracts` is the next portable layer. It owns fixed-size binary metadata,
IPC headers, snapshots, manifests and flight records. Its Serde, bitflags and
bytemuck support is configured without dependency default features, and the
crate is unconditionally `no_std`. Hosted conveniences must be implemented in
higher layers without changing these shared layouts.

## Profiles

| Profile | Required properties | Intended targets |
| --- | --- | --- |
| `pure-core` | `no_std`, no allocator, no unsafe code, no external dependencies, fixed-memory state and output | Every supported target |
| `embedded` | `pure-core` plus selected `no_std` wire formats and statically supplied device adapters | Bare-metal MCU, RTOS, PLC |
| `alloc` | Explicit allocator availability, bounded ownership outside reducer hot paths | Mobile and constrained hosted systems |
| `std` | Filesystem, networking, threads, durable storage and process adapters | Desktop, server, embedded Linux |
| `accelerated` | Explicit GPU/vendor adapter with separately reviewed native provenance | GPU-capable desktop, server and edge systems |

Profiles are additive only through an application composition root. Core crates
must not detect a platform, open a device, load a library, or select an adapter.

## Scan boundary

Each cycle has three phases:

1. A platform adapter acquires an explicit fixed-size input frame.
2. A `ScanReducer` calculates scratch state and output without external side
   effects. `ScanMachine` commits both together only on success.
3. A platform adapter applies the committed output or publishes telemetry.

The double-buffered state and output are supplied at construction. A failed
transition cannot expose partial state. The reducer has no receiver instance,
which prevents the framework from providing hidden mutable reducer storage.
Implementations still require static review for ambient access or interior
mutability before they are accepted for an embedded profile.

## Platform boundary

Heavy implementations may exist in the repository, but they cannot be reachable
from `pure-core` or `embedded` graphs. RocksDB, SQLite, desktop UI, dynamic
loading, model hosting, GPU runtimes, and OS services terminate in explicit
host adapters. An MCU or PLC artifact contains only the scan kernel, selected
fixed-memory components, wire contracts, and its device adapters.

The Rust-owned model host is a `std`/`accelerated` service and is not linked into
embedded control artifacts. Embedded nodes may exchange bounded commands and
telemetry with that host; loss of the host cannot bypass local safety reducers.

## Supported-platform boundary

“All platforms within reason” means targets supported by the selected stable
Rust toolchain and their vendor or community-maintained hardware abstraction
layers. A target is supported only after its exact graph builds and its memory,
timing, wire-compatibility, and hardware behavior pass target-specific gates.
Repository presence, LLVM target availability, or successful host emulation is
not certification of a device.

## Enforcement

The canonical local gate and CI run the native host plus representative ARM
bare-metal and WebAssembly profiles:

```text
cargo check -p scan_core --no-default-features
cargo check -p scan_core --target thumbv7em-none-eabihf --no-default-features
cargo check -p scan_core --target wasm32-unknown-unknown --no-default-features
cargo check -p core-contracts --no-default-features
cargo check -p core-contracts --target thumbv7em-none-eabihf --no-default-features
cargo check -p core-contracts --target wasm32-unknown-unknown --no-default-features
```

This is the first portability invariant. Later slices must add locked graph
policy for `pure-core` and representative cross-target builds before migrating
existing components. No migration may introduce an operating-system or native
dependency into `scan_core`.
