# Event-Driven Component Onboarding & Wire Geometry Specification

> **CANONICAL SPECIFICATION**  
> **SCOPE**: Zero-Copy Memory Contracts, Typestate Ingestion Lifecycle, and Microkernel Non-Blocking Drains.  
> **APPLIES TO**: `crates/ipc_bus`, `crates/orchestrator`, `crates/orchestration_plane`, and `core/hypervisor`.  
> **LAST UPDATED**: 2026-09-23

> **Governing policy:** This document specifies the *runtime mechanics* of onboarding (wire geometry, typestate lifecycle, reducer). *Whether* a component may be onboarded, and in what form, is decided by the Cratify graduation gate and dependency admission in [CRATIFY_SPEC.md](../CRATIFY_SPEC.md) sections 5-6: proven in origin, classified as new component vs. upgrade, placed behind a workspace trait, landed inert or run in shadow mode, and origin copy deleted.

---

## 1. Paradigm Shift: From Imperative Harvesting to Reactive Streams

Historically, asset ingestion relied on external, blocking batch scripts (`cratify::harvest`, legacy staging sweeps) that walked the file tree, invoked external compilers synchronously, and populated static directories. This violated several prime invariants:
- Induced micro-architectural jitter and long thread parking times on the hypervisor duty cycle.
- Performed ambient filesystem reads outside configuration-injected descriptors.
- Allocated dynamic heap strings and vectors during AST parsing.

Asset onboarding is now codified as a **pure, reactive, event-driven stream function** embedded within the orchestration plane. Ingestion events arrive as raw zero-copy byte frames over the `ipc_bus`, are processed through a strictly deterministic typestate state machine, and emit immutable telemetry broadcasts without allocating a single byte on the heap.

```
       UniversalClientRequest (544 bytes)
             [UcpRequestType::AssimilationEvent]
                            │
                            ▼
              ipc_bus SWMR Ring Buffer
                            │
                            ▼ (try_recv / Step 0 non-blocking drain)
           core/hypervisor::OrchestrationDaemon
                            │
                            ▼ handle_onboarding_event(&bytes)
          crates/orchestrator::onboarding
   ┌────────────────────────────────────────────────────────┐
   │            Typestate Machine Transition:               │
   │  Idle ──► Quarantined ──► Auditing ──► Synthesizing   │
   │                                             │          │
   │  Committed ◄────────── Certifying ◄─────────┘          │
   │       ▲                      │                         │
   │       │                      ▼ (retries >= max)        │
   │       └──────────────── Rejected                       │
   └────────────────────────────────────────────────────────┘
                            │
                            ▼ to_broadcast()
          UniversalServerBroadcast (304 bytes)
             [UcpBroadcastType::AssimilationState]
```

---

## 2. Wire Geometry & Zero-Copy Memory Contracts

All wire representations conform to fixed-size, C-compatible binary layouts. They derive `bytemuck::Pod` and `bytemuck::Zeroable`, require zero serialization overhead, and feature explicit struct padding to ensure 8-byte natural boundary alignment.

### 2.1 The `AssimilationRecord` (360 Bytes)

Defined in [`crates/ipc_bus/src/universal_protocol.rs`](../../crates/ipc_bus/src/universal_protocol.rs):

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct AssimilationRecord {
    pub source_id: [u8; 16],          // Offset: 0,   Size: 16 bytes
    pub phase: u32,                   // Offset: 16,  Size: 4 bytes
    pub retries: u32,                 // Offset: 20,  Size: 4 bytes
    pub started_at_us: u64,           // Offset: 24,  Size: 8 bytes
    pub mount_path: FixedString256,   // Offset: 32,  Size: 260 bytes (4B len + 256B data)
    pub ir_hash: FixedString64,       // Offset: 292, Size: 68 bytes  (4B len + 64B data)
}
```

#### Memory Layout Verification
$$\text{Total Size} = 16 + 4 + 4 + 8 + 260 + 68 = 360 \text{ bytes}$$
- **Alignment**: 8 bytes (due to `u64 started_at_us`).
- **Trailing Padding**: $360 \pmod 8 = 0$, requiring zero trailing padding.
- **Safety Assertions**: Compile-time constant assertion `const _: () = assert!(core::mem::size_of::<AssimilationRecord>() == 360);`.

### 2.2 Wire Framing Over the Universal Communication Protocol (UCP)

`AssimilationRecord` instances are encapsulated within the fixed-size universal framing envelopes:

1. **Client Ingestion Frame**: [`UniversalClientRequest`](../../crates/ipc_bus/src/universal_protocol.rs) (544 bytes, 8-byte aligned)
   - `request_type`: `UcpRequestType::AssimilationEvent as u32` (`0x0000_0006`).
   - `payload`: First 360 bytes contain the raw `AssimilationRecord` via `bytemuck::bytes_of(&record)`.
   - Conversion Helper: `record.to_client_request(sequence, slot_id, domain_id)`.

2. **Server Telemetry Broadcast**: [`UniversalServerBroadcast`](../../crates/ipc_bus/src/universal_protocol.rs) (304 bytes, 8-byte aligned)
   - `broadcast_type`: `UcpBroadcastType::AssimilationState as u32` (`0x0000_0007`).
   - `payload`: Truncated or summary metrics (source ID, phase, retries, cycle latency).
   - Conversion Helper: `record.to_broadcast(sequence, cycle_latency_us)`.

---

## 3. Typestate Machine: `AssimilationTask<State>`

In [`crates/orchestrator/src/assimilation.rs`](../../crates/orchestrator/src/assimilation.rs), the assimilation lifecycle is implemented as a compile-time typestate machine, preventing invalid phase progressions at compile time.

```
       [Idle]
         │
         │ quarantine(path)
         ▼
    [Quarantined]
         │
         │ begin_audit()
         ▼
     [Auditing]
      │      │
      │ Pass │ Fail
      │      ▼
      │   [Rejected]
      ▼
   [Synthesizing] ◄──────────────┐
         │                       │ (retries < max_retries)
         │ finalize_synthesis()  │
         ▼                       │
    [Certifying] ────────────────┘
      │      │
      │ Pass │ Fail (retries >= max_retries)
      ▼      ▼
  [Committed] [Rejected]
```

### 3.1 State Marker Types

Each phase is an empty struct implementing `AssimilationState`:
- `Idle`: Initial allocated slot waiting for target source registration.
- `Quarantined`: Path registered and isolated within sandbox.
- `Auditing`: Static AST invariant analysis underway (via `ast_auditor`).
- `Synthesizing`: Zero-copy Pod contracts and IR DAG being compiled.
- `Certifying`: Regression testing against golden harness traces.
- `Committed`: Successfully integrated into production workspace rings.
- `Rejected`: Violation detected or retry budget exhausted.

### 3.2 Transition Signatures

```rust
impl AssimilationTask<Idle> {
    pub fn new(source_id: [u8; 16], timestamp_us: u64) -> Self;
    pub fn quarantine(self, path: FixedString256) -> AssimilationTask<Quarantined>;
}

impl AssimilationTask<Quarantined> {
    pub fn begin_audit(self) -> Result<AssimilationTask<Auditing>, AssimilationError>;
}

impl AssimilationTask<Auditing> {
    pub fn conclude_audit(self, passed: bool) 
        -> Result<AssimilationTask<Synthesizing>, AssimilationTask<Rejected>>;
}

impl AssimilationTask<Synthesizing> {
    pub fn finalize_synthesis(self, ir_hash: FixedString64) 
        -> Result<AssimilationTask<Certifying>, AssimilationError>;
}

impl AssimilationTask<Certifying> {
    pub fn certify(self, passed: bool, max_retries: u32) 
        -> Result<AssimilationTask<Committed>, Result<AssimilationTask<Synthesizing>, AssimilationTask<Rejected>>>;
}
```

---

## 4. Pure Reactive State-Transition Reducer

For hot-path execution where dynamic typestate wrappers cannot be stored in heterogeneous arrays, the orchestrator provides the zero-allocation pure reducer:

```rust
#[deny(unsafe_code)]
pub fn handle_assimilation_event(bytes: &[u8]) -> Result<AssimilationRecord, AssimilationError>
```

### 4.1 Invariants of the Reducer
1. **Zero Dynamic Allocation**: Never allocates `String`, `Vec`, or `Box`.
2. **Deterministic Time Complexity**: Evaluates in $\mathcal{O}(1)$ time.
3. **Safe Casting**: Direct cast from binary slices using `bytemuck::try_from_bytes::<AssimilationRecord>(bytes)`.
4. **Retry Bounding**: Automatically enforces $R \le R_{\text{max}}$. When retries exceed the limit upon failure, phase transitions to `AssimilationPhase::Rejected`.

---

## 5. Hypervisor Step 0 Integration

In [`core/hypervisor/src/orchestration_daemon.rs`](../../core/hypervisor/src/orchestration_daemon.rs), the microkernel executes a strict 3-phase scan loop. Assimilation frames are drained at the very beginning of the cycle:

```rust
// Step 0: Non-blocking drain of incoming assimilation frames
while let Ok(frame_bytes) = self.assimilation_rx.try_recv() {
    match orchestrator::assimilation::handle_assimilation_event(&frame_bytes) {
        Ok(updated_record) => {
            let broadcast = updated_record.to_broadcast(self.cycle_count, cycle_latency_us);
            let _ = self.bus.publish_broadcast(&broadcast);
        }
        Err(err) => {
            // Emit diagnostic telemetry without crashing hypervisor loop
        }
    }
}
```

Because `try_recv()` is completely non-blocking, the microkernel's real-time guarantee is preserved regardless of how many frames arrive on the assimilation bus.
