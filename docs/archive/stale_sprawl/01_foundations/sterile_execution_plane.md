# 01: The Sterile Execution Plane Specification

**Version:** v1.7.0 (Aaroneous Core Framework)  
**Status:** Canonical Engineering Specification  
**Domain:** Memory Invariants, Micro-Architectural Sympathy, and Zero-Allocation Safety  

---

## 1. Executive Summary

The **Sterile Execution Plane (SEP)** defines the runtime execution guarantees and memory contracts enforced across the Aaroneous hypervisor and all Accelerated Component Containers (ACCs). In ultra-low-latency real-time telemetry, visual perception (DXGI), motor emulation, and continuous state-space modeling (SSM), non-deterministic heap allocations, locks, and GC-like memory stalls induce unacceptable micro-architectural jitter.

The SEP establishes a deterministic, lock-free execution boundary where **zero heap allocations are permitted inside the hot telemetry, inference, and motor execution loops**.

---

## 2. Core Invariants & Safety Contracts

All modules executing within the Sterile Execution Plane must satisfy four invariant contracts:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Sterile Execution Plane                         │
├────────────────────────────────────────────────────────────────────────┤
│  1. Zero Hot-Loop Allocations     │  `Vec::push`, `Box::new` Forbidden  │
│  2. Fixed-Capacity Contiguity     │  Stack `[u8; N]` / Sized Slices     │
│  3. Pod Zero-Copy Invariants      │  `bytemuck::Pod` + `Zeroable`       │
│  4. Deterministic Slab Execution  │  Pre-allocated L1/L2 Cache-Aligned  │
└────────────────────────────────────────────────────────────────────────┘
```

### Invariant 1: Zero Hot-Loop Allocations
- No invocations of global allocators (`alloc`, `realloc`, `dealloc`) within any function operating on the primary tick, capture, sensory scan, or dispatch paths.
- Dynamic data structures (`std::vec::Vec`, `std::collections::HashMap`, `std::string::String`) are strictly forbidden on the hot path.
- In-place mutation and statically pre-allocated scratch buffers must be utilized exclusively.

### Invariant 2: Fixed-Capacity Contiguity (`[u8; N]`)
- Fixed-capacity array buffers (`[u8; N]`, `[f32; N]`) or inline bounded structures (e.g. `arrayvec::ArrayVec`, `smallvec::SmallVec` with strict stack capacity, or `smol_str::SmolStr`) replace heap strings and dynamic vectors.
- Array lengths and bounds must be provable at compile time or strictly bounded by compile-time constants.

### Invariant 3: Plain Old Data (POD) & Zero-Copy Contracts
- All message passing, IPC transfers, and shared-memory transfers must derive `bytemuck::Pod` and `bytemuck::Zeroable`.
- Types must have well-defined, deterministic ABI layouts (`#[repr(C)]` or `#[repr(C, align(64))]`).
- Memory transfers are conducted via direct slice casting over memory-mapped regions without serialization, deserialization, or heap copy overhead.

### Invariant 4: Cacheline Alignment & Blast Radius Isolation
- State blocks and ring buffer slots must align to 64-byte CPU cache boundaries (`align(64)`) to eliminate false sharing across hardware hyperthreads.
- All ACC definitions must declare `max_blast_radius = "isolated"` in their configuration metadata.

---

## 3. Memory Architecture: Slab Allocation & Lifetimes

To support dynamic capacity demands without violating zero-allocation constraints, the SEP employs a multi-tiered pre-allocated memory topology:

```
+-----------------------------------------------------------------------+
|                       Memory Hierarchy & Budgets                      |
+-----------------------------------+-----------------------------------+
| Tier                              | Budget & Placement               |
+-----------------------------------+-----------------------------------+
| L1 Scratchpad (Stack)             | Fixed <= 64 KB per thread         |
| Core Engine Ring Buffer (L2/L3)   | Pre-allocated Shared Memory Slab  |
| Cartridge Continuous Slab (VRAM)  | Mapped via Direct3D12/Vulkan/mmap |
+-----------------------------------+-----------------------------------+
```

### 3.1 Pre-Allocated Ring Buffer Topology
- Inter-container communications transit single-producer single-consumer (SPSC) or single-producer multi-consumer (SPMC) ring buffers.
- Ring buffer index heads and tails are managed via atomic compare-and-swap (`AtomicUsize`) operations with sequential consistency or acquire-release memory orderings.
- Buffer slots wrap around via bitwise power-of-two masking: `index & (CAPACITY - 1)`.

### 3.2 Thread-Local Slabs
- When temporary variable-length state is required during a transaction, worker threads draw from thread-local pre-allocated slab arenas.
- Slabs reset their cursor at the end of each frame cycle without freeing underlying virtual memory pages, guaranteeing O(1) amortized and worst-case acquisition time.

---

## 4. Rust Idiomatic Enforcement Patterns

### Pattern A: Stack-Allocated Frame Token
```rust
// Fixed-capacity token without heap allocation
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MotorFrameToken {
    pub timestamp_rdtsc: u64,
    pub frame_sequence: u64,
    pub target_coordinates: [f32; 4],
    pub state_mask: u32,
    pub _reserved: [u8; 36],
}
```

### Pattern B: Safe Zero-Copy View
```rust
/// Borrowing from a contiguous pre-allocated byte slab
#[inline(always)]
pub fn parse_motor_frame(slab: &[u8]) -> Result<&MotorFrameToken, TransducerError> {
    if slab.len() < core::mem::size_of::<MotorFrameToken>() {
        return Err(TransducerError::InsufficientBuffer);
    }
    bytemuck::try_from_bytes(&slab[..core::mem::size_of::<MotorFrameToken>()])
        .map_err(|_| TransducerError::AlignmentViolation)
}
```

---

## 5. Verification & Tooling Gates

Compliance with the Sterile Execution Plane is verified via automated CI and compiler toolchains:

1. **Alloc-Counter Tracing**: Critical harness tests execute under an instrumented custom allocator that asserts an allocation count of zero during benchmark iterations.
2. **Clippy & Linter Interlocks**:
   - `clippy::disallowed_methods` flags `.clone()`, `Box::new`, `Vec::push`, and string allocations in SEP-tagged crates.
3. **W^X Memory Enforcement**: Memory-mapped code pages are mapped `RX` and data buffers are mapped `RW`, prohibiting dynamic runtime code modification in sterile execution segments without formal hypervisor authorization.
