# Cartridge Specification: Solid-State Cartridge v3.0 & Tensor Structures

**Version:** `v1.7.0`  
**Classification:** Core Data Format Specification  

---

## 1. The `.si` v3.0 Solid-State Cartridge Container

The `.si` (Synthetic Intelligence) v3.0 format is a single-file, zero-deserialization container storing neural weights, affine projection matrices, continuous state-space matrices, and dynamic habit caches.

### 1.1 Four-Block Container Hierarchy
The cartridge is segmented into four sequential, 64-byte aligned blocks:

```
+-------------------------------------------------------------------------+
|                  .si v3.0 Solid-State Cartridge Layout                  |
+--------------------------+----------------------------------------------+
| Block                    | Content & Function                           |
+--------------------------+----------------------------------------------+
| Block 0: Header & Seals  | SiHeaderV3, SHA-256 seal, cryptographic signature|
| Block 1: Tensor Slab     | Static parameter weights (IEEE 754 f32/f16)   |
| Block 2: Affine Projections| Linear projection tensors & latent embeddings |
| Block 3: Dynamic Habits  | Compiled Cranelift native execution caches    |
+--------------------------+----------------------------------------------+
```

### 1.2 Binary Header Layout (`#[repr(C, align(64))]`)
```rust
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SiHeaderV3 {
    pub magic: [u8; 4],            // b"SI30"
    pub version: u32,              // 3
    pub tensor_count: u32,         // Total tensors in registry
    pub block_flags: u32,          // Capability & encryption flags
    pub state_dimension: u32,      // Dimension (e.g. 256)
    pub sequence_length: u32,      // Context window capacity
    pub rkyv_offset: u64,          // Byte offset to rkyv archive
    pub rkyv_length: u64,          // Byte length of rkyv archive
    pub tensor_payload_offset: u64,// Byte offset to raw tensor slab
    pub crc32_checksum: u32,       // Header + payload checksum
    pub _reserved: [u8; 12],
}
```

---

## 2. Zero-Copy Architecture via `rkyv`

Unlike conventional formats (such as GGUF, SafeTensors, or Protobuf) that allocate temporary heap objects during parsing, `.si` v3.0 utilizes `rkyv` archives:
- **Direct Pointer Mapping:** Metadata, tensor names, dimensions, and type tags are mapped directly from disk into memory (`memmap2`).
- **Zero Allocations on Open:** Opening a `.si` cartridge performs validation in place and maps pointers directly. Time-to-first-token is reduced to sub-microsecond disk seek latency.

```rust
/// Zero-copy accessor for cartridge metadata
pub fn access_cartridge_meta<'a>(bytes: &'a [u8], offset: usize) -> &'a ArchivedCartridgeMeta {
    unsafe { rkyv::archived_root::<CartridgeMeta>(&bytes[offset..]) }
}
```

---

## 3. Continuous HiPPO State-Space Modeling (SSM)

Aaroneous eliminates quadratic attention complexity on the hot path by leveraging continuous-time linear state-space models based on HiPPO (High-order Polynomial Projection Operators).

### 3.1 Mathematical Recurrence Formulation
Given input signal $u(t)$, the continuous state evolution is represented by:

$$\frac{d}{dt} x(t) = \mathbf{A} x(t) + \mathbf{B} u(t)$$

$$y(t) = \mathbf{C} x(t) + \mathbf{D} u(t)$$

- **$\mathbf{A}$ Transition Matrix:** Initialized using the canonical HiPPO continuous shifted Legendre polynomial matrix:
  $$A_{nk} = -\begin{cases} (2n + 1)^{1/2}(2k + 1)^{1/2} & \text{if } n > k \\ n + 1 & \text{if } n = k \\ 0 & \text{if } n < k \end{cases}$$
- **Discretization:** Continuous state transitions are discretized using the bilinear (Tustin) transform into transition matrices $\mathbf{\bar{A}}$ and $\mathbf{\bar{B}}$:
  $$\mathbf{\bar{A}} = \left(\mathbf{I} - \frac{\Delta}{2}\mathbf{A}\right)^{-1}\left(\mathbf{I} + \frac{\Delta}{2}\mathbf{A}\right), \quad \mathbf{\bar{B}} = \left(\mathbf{I} - \frac{\Delta}{2}\mathbf{A}\right)^{-1} \Delta \mathbf{B}$$

---

## 4. Hardware Dispatch: `cubecl` Parallel Associative Scans

In place of sequential recurrent CPU loops, `.si` v3.0 SSM tensors are evaluated using parallel associative scans:

- **Scan Associativity:** The recurrent state computation $(a_1, b_1) \bullet (a_2, b_2) = (a_1 a_2, a_2 b_1 + b_2)$ is associative, allowing full parallelization across GPU compute units.
- **`cubecl` Kernel Dispatch:** Custom WGSL and compute shaders execute associative scans across batches in $< 180\mu\text{s}$ on modern hardware, streaming output tokens directly into the Sterile Execution Plane.
