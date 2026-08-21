# 06: Machine-Native Linking Protocol Specification

## Purpose & Scope

The **Machine-Native Linking Protocol (MNLP)** is the universal, zero-overhead binary communication standard that enables all decoupled programs in the Aaroneous ecosystem (Aaroneous, Marionette, Chimera, Odin, Merlin, Ariel) to discover, link, synchronize state, and exchange dense tensors without converting data into human language or JSON strings.

---

## 💾 Core Packet Header Specification (C-ABI Compatible)

All binary messages exchanged across the NATS bus, local IPC sockets, or shared memory ring buffers adhere to a fixed 64-byte aligned header:

```rust
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MachinePacketHeader {
    /// Magic identifier: 0x4141524F ('AARO')
    pub magic: u32,
    /// Protocol version (e.g. 1)
    pub protocol_version: u16,
    /// Message category opcode (see Opcode Table)
    pub opcode: u16,
    /// Unique message / sequence ID
    pub sequence_id: u64,
    /// Timestamp (microsecond UNIX epoch)
    pub timestamp_us: u64,
    /// 128-bit UUID of sender program / specialist
    pub sender_id: [u8; 16],
    /// 128-bit UUID of destination program / specialist (or broadcast)
    pub target_id: [u8; 16],
    /// Payload length in bytes following this header
    pub payload_size: u32,
    /// CRC32-C checksum of the payload
    pub payload_checksum: u32,
}
```

---

## 📑 Opcode Mapping Table

| Opcode Range | Category | Description | Primary Data Format |
| :--- | :--- | :--- | :--- |
| `0x0001 - 0x00FF` | **Lifecycle & Handshake** | Heartbeats, discovery, registration, graceful shutdown. | `HandshakePayload` struct |
| `0x0100 - 0x01FF` | **Task & Intent (Odin)** | Task DAG generation, step dispatch, status updates. | `TaskIntentPayload` struct |
| `0x0200 - 0x02FF` | **Sensory & Reflex (Marionette)**| 128x128 sensory grid, motor intents, epigenetic gate mask. | `128x128 f32` grid / `MotorIntent` |
| `0x0300 - 0x03FF` | **Knowledge & Embeddings (Merlin)**| Semantic vector queries, embedding transfers, DB updates. | `1024x f32` vector embeddings |
| `0x0400 - 0x04FF` | **Code & Patching (Chimera)** | AST diffs, patch proposals, bytecode inspection requests. | `AstPatchPayload` |
| `0x0500 - 0x05FF` | **Telemetry & UI (Ariel)** | FPS, GPU compute latency, thermal metrics, token reserves. | `SystemTelemetryPayload` |

---

## ⚡ Shared Memory Synapse Buffer Layout

For microsecond-scale state sharing between local programs, a dedicated memory-mapped region (`.synapse`) is maintained with lockless atomics:

```
Offset 0x0000 ┌─────────────────────────────────────────────────────────┐
              │ Header & Global State (Tick counter, Thermal, Backpressure) │
Offset 0x0100 ├─────────────────────────────────────────────────────────┤
              │ 128x128 Sensory Frame Buffer (64 KB, f32 luminance grid)│
Offset 0x10100├─────────────────────────────────────────────────────────┤
              │ Epigenetic Gate Matrix (256 sectors, boolean / u8 mask) │
Offset 0x10200├─────────────────────────────────────────────────────────┤
              │ Active Motor Intent Register (delta_x, delta_y, actions)│
Offset 0x10300├─────────────────────────────────────────────────────────┤
              │ 1024-Dimension Latent Vector Buffer (4 KB, f32)         │
Offset 0x11300├─────────────────────────────────────────────────────────┤
              │ Ring Buffer for Inter-Program Event Queue               │
              └─────────────────────────────────────────────────────────┘
```

---

## 📡 NATS Topic Hierarchy

When messages cross process boundaries where shared memory is not attached, NATS carries the native binary packets over the following topic namespace:

- `aaroneous.v1.discovery.announce` - Program heartbeat and capability announcement.
- `aaroneous.v1.odin.tasks.dispatch` - Task DAG assignment to specialists.
- `aaroneous.v1.odin.tasks.status` - Execution status callbacks.
- `aaroneous.v1.merlin.query.vector` - Vector similarity lookup requests.
- `aaroneous.v1.marionette.perception` - Vision frame broadcast (downscaled).
- `aaroneous.v1.marionette.motor.intent` - Desired peripheral actuation events.
- `aaroneous.v1.chimera.patch.proposal` - Proposed code modifications.
- `aaroneous.v1.ariel.telemetry.stream` - Real-time metrics broadcast for UI rendering.
