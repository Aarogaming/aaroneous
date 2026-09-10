# Core Contracts ABI & Inter‑ACC Messaging Protocol

**Version:** v1.2.0 (Aaroneous Framework)

---

## 1. Overview
The `core-contracts` crate defines the **zero‑copy binary contracts** used by all ACCs (Accelerated Component Containers) to communicate over the microkernel bus. All public structs implement `bytemuck::Pod` and `Zeroable`, guaranteeing a plain‑old‑data layout that can be safely shared across process boundaries without serialization overhead.

---

## 2. Primary Types

### 2.1 `IpcHeader`
```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct IpcHeader {
    pub dst: u32,
    pub src: u32,
    pub len: u32,
}
```
* **Size:** 12 bytes
* **Purpose:** Prepended to every transport frame. `dst`/`src` are ACC IDs; `len` is the payload byte‑length (excluding the header).
* **Zero‑Copy:** Because of `#[repr(C)]` + `Pod`, the header can be copied directly into a shared‑memory ring buffer.

---

### 2.2 `ComponentManifest`
```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ComponentManifest {
    pub name: [u8; 32],
    pub version: u32,
    pub tier: u8,
    pub capabilities: u64,
    pub methodology: u8,
    pub supported_intents: u16,
    pub priority_weight: u8,
    pub address: u32,
}
```
* **Size:** 64 bytes (validated by unit test `manifest_size`).
* **Fields:**
  - `name` – UTF‑8 bytes, zero‑padded, max 32 characters.
  - `version` – Packed MAJOR << 16 | MINOR << 8 | PATCH via `pack_version`.
  - `tier` – Corresponds to `[tier]` in `cratify.toml` (`0 = user`, `1 = isolated`).
  - `capabilities` – Bitmask from `Capability` flags.
  - `methodology` – Enum `ExecutionMethodology` discriminant.
  - `supported_intents` – Bitmask from `OperationalIntent`.
  - `priority_weight` – Scheduler weight (0‑255).
  - `address` – Physical or virtual address the hypervisor assigns for the component’s entry point.

---

### 2.3 `QueryDescriptor`
```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct QueryDescriptor {
    pub required_capabilities: u64,
    pub methodology: u8,
    pub intent: u16,
    pub priority: u8,
}
```
* **Size:** 24 bytes (validated by unit test `query_size`).
* **Purpose:** Describes a routing request from one ACC to another. The hypervisor’s **CapabilityBroker** matches `required_capabilities` against the target’s `Capability` mask, then selects an ACC whose `methodology` and `intent` are compatible.

---

## 3. Supporting Enums & Bitflags

| Type | Representation | Values |
|------|----------------|--------|
| `HierarchyTier` | `u8` | `Core = 0`, `System = 1`, `SubordinateModule = 2`, `Extension = 3` |
| `ExecutionMethodology` | `u8` | `LocalQuantizedModel = 0x01`, `RemoteApiProxy = 0x02`, `HardwareAcceleratedDirect = 0x03`, `DeterministicSimulation = 0x04` |
| `OperationalIntent` | `u16` (bitflags) | `RealtimeLowLatency`, `BatchProcessing`, `HighThroughput`, `SecureEnclave` |
| `Capability` | `u64` (bitflags) | `INFERENCE_OUT`, `AUDIO_OUT`, `VIDEO_OUT`, `SENSOR_INPUT` |

All enums derive `Serialize`/`Deserialize` for optional JSON/YAML export but retain the raw integer discriminant for the binary ABI.

---

## 4. Memory Layout Guarantees
`bytemuck::Pod` ensures:
1. **No padding beyond C representation** – `#[repr(C)]` forces compiler‑defined layout compatible with C structs.
2. **No `unsafe` required** – Instances can be safely transmuted to byte slices (`&[u8]`) and written/read from shared memory.
3. **Alignment** – Each field’s natural alignment is respected, matching the expectations of the `Disruptor` ring‑buffer implementation used by the hypervisor.

Unit tests in `lib.rs` (`manifest_size`, `query_size`) assert the exact byte length, guaranteeing forward‑compatibility when the hypervisor parses frames.

---

## 5. Inter‑ACC Messaging Flow
1. **Bootstrap** – Each ACC publishes its `ComponentManifest` via a one‑time `cratify translate` step. The hypervisor stores the manifest in a global registry.
2. **Message Construction** – Caller builds a `QueryDescriptor` and payload struct (must also be `Pod`).
3. **Header Prep** – An `IpcHeader` is prepended with the destination ACC ID, source ID, and payload length.
4. **Ring Buffer Write** – The full frame (`IpcHeader` + payload) is written into a lock‑free `Disruptor` ring buffer allocated per tier.
5. **Broker Routing** – `CapabilityBroker` reads the header, looks up the target ACC’s manifest, checks `required_capabilities` against `Capability`, and validates `methodology`/`intent` compatibility.
6. **Delivery** – The frame is copied into the target ACC’s memory region; the ACC reads the `IpcHeader`, verifies `len`, casts the payload to the expected `Pod` type, and processes it.

All steps avoid serialization/deserialization overhead; the only dynamic operation is the bit‑mask match performed by the broker.

---

## 6. Extending the ABI
* To add a new contract, define a `#[repr(C)]` struct, derive `Pod`/`Zeroable`, and add a unit test asserting its `size_of::<T>()`.
* Update `ComponentManifest` with a new `tier` or capability flag as needed; bump the manifest version.
* Ensure any new enum discriminants are stable (do not reorder variants).

---

*This specification is the authoritative reference for developers integrating new ACCs or extending the hypervisor bus.*
