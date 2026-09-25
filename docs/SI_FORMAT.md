# .si Binary Format Specification (SINT v3.0)

The `.si` (SINT - Synthetic Intelligence Native Topology) format is a binary container for solid-state neural model cartridges. It is designed for zero-copy memory mapping, SIMD-aligned weight access, and compile-time lifecycle safety.

## File Layout

```
Offset  Size  Field
──────  ────  ──────────────────────────────────────────
0x00    4     Magic: "SINT" [0x53, 0x49, 0x4E, 0x54]
0x04    2     Version: 3 (u16 little-endian)
0x06    2     Header Size: 64 (u16 little-endian)
0x08    4     Flags: tier bitmask (u32 little-endian)
0x0C    4     CRC32 Checksum (u32 little-endian)
0x10    8     Block 1 Offset (u64 little-endian)
0x18    8     Block 1 Length (u64 little-endian)
0x20    8     Block 2 Offset (u64 little-endian)
0x28    8     Block 2 Length (u64 little-endian)
0x30    8     Block 3 Offset (u64 little-endian)
0x38    8     Block 3 Length (u64 little-endian)
──────  ────  ──────────────────────────────────────────
0x40    ...   Block 1: Frozen Core SSM Weights
              (64-byte aligned, zero-copy mmap target)
              ...
              Block 2: Dynamic Adaptation Matrix
              (LoRA delta, mutable at runtime)
              ...
              Block 3: Episodic Skill Stack
              (mined AST DAGs, habits, fast-reflex pathways)
```

Total header size: **64 bytes**. All block offsets are measured from byte 0 of the file.

## Tier Flags

| Bit | Flag | Purpose | Model Geometry |
|-----|------|---------|----------------|
| 0 | `SI_FLAG_TIER_1_CORTEX` | Heavy inference, deep reasoning | d_model=4096, 4 layers |
| 1 | `SI_FLAG_TIER_2_ROUTER` | Medium dispatch, task routing | d_model=256, 2 layers |
| 2 | `SI_FLAG_TIER_3_REFLEX` | Fast-twitch reactive workers | d_model=256, 2 layers |
| 4 | `SI_FLAG_ENCRYPTED` | Reserved (not yet implemented) | - |
| 5 | `SI_FLAG_COMPRESSED` | Reserved (not yet implemented) | - |

Flags are bitmask-ORable. A cartridge can declare multiple tiers.

## Block Structure

### Block 1: Frozen Core SSM Weights

Immutable base model weights. Memory-mapped directly into active virtual memory via `memmap2`. Contains the frozen selective state-space model (S4/Mamba-style) recurrence parameters:

- 4 recurrent layers
- 1024-element state vectors
- 256 model dimension
- 64 state rank (~890k parameters, ~3.56 MB)

This block is never modified at runtime. It provides the deterministic base behavior that survives catastrophic forgetting.

### Block 2: Dynamic Adaptation Matrix

Mutable low-rank adapter weights (LoRA-style). Contains `ΔW = A_adapt · B_adapt` delta matrices (rank 16, ~64 KB footprint). Updated at runtime via:

- **Error steering**: On panic/error, applies negative gradient step to avoid repeated failures
- **Reinforcement**: On task success, applies reinforcement step to cement optimal paths

This block is the only mutable section of the cartridge.

### Block 3: Episodic Skill Stack

Mined computational DAGs, hotkeys, dimensional signatures, and learned habits. Frozen from high-fitness execution traces. Contains:

- Skill graphs (mined from AST analysis)
- Execution pathway signatures
- Reflex triggers for fast-twitch responses

## Invariants

- **Magic bytes**: Must be `SINT` [0x53, 0x49, 0x4E, 0x54]
- **Version**: Must be ≥ 3 (v3.0 is current canonical)
- **Alignment**: All block offsets must be 64-byte aligned
- **Block lengths**: Must be multiples of 4 (f32 alignment)
- **CRC32**: Computed over all block payload bytes (after header)
- **File integrity**: Block offsets + lengths must fit within file size

## Typestate Lifecycle

The `si_format` crate enforces compile-time lifecycle safety:

```
Cartridge<Raw> → verify_alignment() → Cartridge<Aligned>
Cartridge<Aligned> → verify_smt() → Cartridge<SmtVerified>
Cartridge<SmtVerified> → into_executable() → Cartridge<Executable>
```

Methods like `execute_tick()` are only available on `Cartridge<Executable>`, making it a compile-time error to execute an unverified cartridge.

## Capability Permissions

Cartridges declare required capabilities via the tier flags. The `verify_smt()` transition checks that the granted capability mask includes all required capabilities:

| Capability | Bit | Purpose |
|------------|-----|---------|
| `CAPABILITY_READ_STORAGE` | 0 | Read from storage |
| `CAPABILITY_WRITE_STORAGE` | 1 | Write to storage |
| `CAPABILITY_NETWORK_MESH` | 2 | Network access |
| `CAPABILITY_HARDWARE_ACCEL` | 3 | GPU/hardware acceleration |
| `CAPABILITY_JIT_EXECUTION` | 4 | JIT code execution |

## Tooling

- **`SiForge`**: End-to-end model birth pipeline (distill → align → pack → verify)
- **`SiCartridgeEngine`**: Pack, verify, unpack, diff operations
- **`SiSolidStateLoader`**: Memory-mapped cartridge loading

## Example: Creating a Cartridge

```rust
use si_format::{Cartridge, Raw};

// Load raw bytes
let cartridge = Cartridge::<Raw>::from_buffer(&bytes)?;

// Verify alignment and magic
let aligned = cartridge.verify_alignment()?;

// Verify capabilities
let verified = aligned.verify_smt(granted_mask, required_mask)?;

// Activate for execution
let executable = verified.into_executable();

// Run a tick
executable.execute_tick(tick, &inputs, &mut outputs)?;
```
