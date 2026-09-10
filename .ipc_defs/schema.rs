/* 
  SYNAPSE SCHEMA V1
  The byte-level "Common Ground" for Aaroneous.
  This file defines the memory layout shared by Rust, Python, and WASM.
  
  Layout:
  [0..4]   - MAGIC (0xAA 0x55 0xAA 0x55)
  [4..8]   - SCHEMA_VERSION (1)
  [8..12]  - SYSTEM_STATUS (0: Idle, 1: Active, 2: Throttled, 3: Critical)
  [12..16] - COMMAND_PTR (Offset to current command)
  [16..64] - RESERVED_CORE
  [64..]   - DYNAMIC_ENZYME_SPACE
*/

pub const SYNAPSE_MAGIC: [u8; 4] = [0xAA, 0x55, 0xAA, 0x55];
pub const SCHEMA_VERSION: u32 = 1;

#[repr(C)]
pub struct SynapseHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub status: u32,
    pub command_ptr: u32,
}
