# Architecture Specification: Runtime Physics & Substrate

**Version:** `v1.7.0`  
**Classification:** Core Subsystem Specification  

---

## 1. Sterile Execution Plane (SEP) vs. Control Plane

The Aaroneous architecture enforces an absolute separation between the execution kernel and the control plane:

```
┌────────────────────────────────────────────────────────────────────────┐
│                      CONTROL PLANE (Outer Orbit)                       │
│  - Asynchronous Orchestration (Tokio / DAG Management)                 │
│  - Dynamic LLM Transducer Routing (Gateway & Manager)                  │
│  - Native Desktop Studio HUD (egui / wgpu Render Loops)                │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Memory-Mapped Ring Buffers / CAS
┌───────────────────────────────────▼────────────────────────────────────┐
│                  STERILE EXECUTION PLANE (Core Kernel)                 │
│  - Deterministic Zero-Allocation Hot Loops (Telemetry / Sensory / Motor)│
│  - Fixed-Capacity Contiguity (`[u8; N]`, `[f32; N]`, Pod Slices)       │
│  - 64-Byte Cacheline Aligned Plain Old Data (`bytemuck::Pod`)          │
│  - Continuous HiPPO State-Space Recurrence & GPU Associative Scans     │
└────────────────────────────────────────────────────────────────────────┘
```

### 1.1 Invariant Rules
1. **Zero Hot-Loop Allocations:** No calls to the global allocator (`alloc`, `realloc`, `dealloc`). Dynamic containers (`Vec`, `String`, `HashMap`, `Box`) are strictly prohibited in the hot execution loop.
2. **Fixed-Capacity Contiguity:** Bounded stack arrays (`[u8; N]`, `[f32; N]`), `arrayvec::ArrayVec`, or `smol_str::SmolStr` replace dynamic allocations. Capacities are verified at compile time.
3. **Cacheline Padding & Alignment:** Hot structs derive `#[repr(C, align(64))]` to align with 64-byte L1 CPU cache lines, eliminating cross-thread false sharing.
4. **Zero-Copy Invariants:** Inter-module communication requires `bytemuck::Pod` and `bytemuck::Zeroable`. Payload transfer is zero-copy slice casting over pre-allocated memory-mapped segments.

```
+-------------------------------------------------------------------------+
|                         Memory Hierarchy Budget                         |
+--------------------------+-------------------------+--------------------+
| Tier                     | Maximum Size            | Allocation Class   |
+--------------------------+-------------------------+--------------------+
| L1 Scratchpad (Stack)    | <= 64 KB / Thread       | Static Array / Pod |
| L2/L3 Ring Buffer Slab   | 16 MB - 64 MB Mapped    | Pre-allocated mmap |
| Continuous VRAM Slab     | Fixed Tensor Memory     | WGPU / DX12 Buffer |
+--------------------------+-------------------------+--------------------+
```

---

## 2. Stateless Transducer Model (LLM Interaction Standard)

LLMs in Aaroneous are strictly prohibited from operating as unconstrained agents, conversational chatterboxes, or autonomous black-box tool loop drivers. They are modeled mathematically as **Stateless Transducers**:

### 2.1 Single-Instruction Operational Contract
- **Text-in / Structured-Data-Out:** Models perform deterministic transformations converting high-entropy unstructured user intent or code context into strongly typed, schema-validated binary or tabular action packets.
- **Zero In-Context Autonomy:** Models possess no persistent awareness across turns and execute zero unmonitored background tool loops inside their contexts.

### 2.2 Decoupled Transport & Orchestration
- **LLM Gateway (Transport Layer):** Manages raw REST/HTTP/QUIC transport, TLS termination, retries, and provider connection pools (Ollama, LM Studio, Remote API endpoints). Possesses zero knowledge of hypervisor business state.
- **LLM Manager (State & Priority Layer):** Manages prompt synthesis, schema validation, token budgeting, and priority task scheduling. Business logic remains isolated from transport mechanics.

---

## 3. Machine-Native Linking Protocol & Ring Buffers

Communication across decoupled engine components utilizes Single-Producer Single-Consumer (SPSC) and Single-Producer Multi-Consumer (SPMC) ring buffers over pre-allocated memory-mapped slabs (`.synapse`).

### 3.1 Standard 64-Byte Machine Packet Header
All frames exchanged across the IPC channels or shared memory buffers adhere to a fixed 64-byte aligned ABI layout:

```rust
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MachinePacketHeader {
    /// Magic identifier: 0x4141524F ('AARO')
    pub magic: u32,
    /// Protocol version (e.g. 1)
    pub protocol_version: u16,
    /// Message category opcode
    pub opcode: u16,
    /// Monotonic sequence ID
    pub sequence_id: u64,
    /// Timestamp (microsecond RDTSC timestamp)
    pub timestamp_us: u64,
    /// 128-bit UUID of sender subsystem
    pub sender_id: [u8; 16],
    /// 128-bit UUID of target subsystem (or broadcast)
    pub target_id: [u8; 16],
    /// Payload length in bytes following this header
    pub payload_size: u32,
    /// CRC32-C checksum of payload
    pub payload_checksum: u32,
}
```

### 3.2 Opcode Classification Taxonomy

| Opcode Range | Domain | Description | Primary Payload Layout |
| :--- | :--- | :--- | :--- |
| `0x0001 - 0x00FF` | **Lifecycle & Handshake** | Subsystem registration, health heartbeats, graceful shutdown | `HandshakePayload` |
| `0x0100 - 0x01FF` | **Task & Intent** | DAG execution steps, task dispatch, priority states | `TaskIntentPayload` |
| `0x0200 - 0x02FF` | **Perception & Motor** | $128 \times 128$ sensory grids, motor intents, delta gates | `[f32; 16384]` / `MotorFrameToken` |
| `0x0300 - 0x03FF` | **Latent Vectors** | Dense embeddings ($\mathbb{R}^{256} / \mathbb{R}^{1024}$), associative recall | `[f32; 256]` / `[f32; 1024]` |
| `0x0400 - 0x04FF` | **AST & Code Mutex** | Dynamic patch proposals, AST diff verification | `AstPatchPayload` |
| `0x0500 - 0x05FF` | **Telemetry & Metrics** | Frame latency, GPU utilization, die thermals, token reserve | `TelemetryStatePayload` |

### 3.3 Ring Buffer Protocol
- Buffer capacities are powers of two ($2^k$). Slot indexing is performed via bitwise masking: `slot = seq & (CAPACITY - 1)`.
- Head and tail sequences advance with `Ordering::Release` on write and `Ordering::Acquire` on read.
- Data transmission avoids serialization; raw binary frames are written directly into slab slots.

---

## 4. Capability Broker & State Synchronization

The hypervisor manages system access and subsystem coordination through two decoupled mechanisms:

### 4.1 CapabilityBroker
- **Granular Token Issuance:** Subsystems cannot invoke Win32 HID, filesystem, or network drivers directly. Operations require an explicit `CapabilityToken` issued by the hypervisor.
- **Revocation Gates:** The broker atomically invalidates tokens using monotonic epoch counters, neutralizing compromised or degraded tasks without restarting the host process.

### 4.2 EngineStatePublisher
- **Zero-Lock State Snapshots:** The core engine maintains a triple-buffered atomic snapshot plane.
- **Dirty-Flag Pacing:** Consumers poll state via atomic dirty-flags. Reading threads incur zero lock contention against writer threads updating telemetry or perception tensors.

---

## 5. Priority-Constrained Backoff Scheduler

Worker threads, transducer queries, and execution graphs are scheduled via microsecond-level hardware timing and thermodynamic resource bounds to prevent queue saturation and resource starvation.

### 5.1 Hardware Timing (RDTSC)
Scheduling quanta are tracked using CPU invariant timestamps (`_rdtsc`). Overhead is sub-nanosecond, eliminating the kernel context switch overhead of OS timing primitives.

### 5.2 Dynamic Backpressure & Pacing Mathematics
When queue saturation occurs, multi-intent spikes emerge, or device temperature crosses defined thresholds, the scheduler applies exponential backoff with bounded jitter:

$$\Delta t_{\text{backoff}} = \min\left(t_{\max},\; t_{\text{base}} \cdot 2^{\text{retries}}\right) \cdot \left(1 + \gamma \cdot \frac{T - T_{\text{target}}}{T_{\text{max}} - T_{\text{target}}}\right)$$

- $T$: Current die/core temperature measured via NVML or OS sensors.
- $T_{\text{target}}$: Thermodynamic equilibrium baseline (default: 65°C).
- $\gamma$: Thermal throttling coefficient.
- If ring buffer headroom falls below 10%, the producer yields execution via `core::hint::spin_loop()` before pausing via thread yield.

---

## 6. Hardware Courtesy & Power-User Execution Invariants

To guarantee respect for host hardware and operator workflows:
1. **Strict Portable Mode (Zero-Footprint):** The engine operates from an unzipped directory with zero Windows Registry modifications, zero persistent background service registrations, and writes exclusively to `./config` and `%LOCALAPPDATA%\Aaroneous`.
2. **Voluntary Yielding (Game & Eco Mode):** The scheduler actively listens for OS full-screen gaming hooks and laptop battery state transitions. Background compute tasks throttle or suspend execution to yield hardware resources.
3. **Anti-Corruption Signal Trapping:** `SIGINT` and `SIGTERM` signals execute deterministic atomic flushes of persistent state stores within 50ms, guaranteeing zero database or index corruption upon forced reboots.
4. **Air-Gapped Default:** Operates 100% offline without background telemetry, analytics, or unrequested network calls.
