//! Rigid POD trace types for dynamic execution instrumentation.
//! Strictly zero-allocation and safe for lock-free SWMR transport.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TraceEvent {
    /// High-precision timestamp (QPC counter delta).
    pub timestamp_qpc: u64, // 8 bytes (Offset 0)
    /// Program Counter / Instruction Pointer in the emulated frame.
    pub pc: u64, // 8 bytes (Offset 8)
    /// Target memory address accessed or mutated.
    pub memory_addr: u64, // 8 bytes (Offset 16)
    /// Value prior to instruction execution.
    pub prev_value: u32, // 4 bytes (Offset 24)
    /// Value post instruction execution.
    pub next_value: u32, // 4 bytes (Offset 28)
    /// Normalized abstract opcode / instruction class.
    pub opcode: u16, // 2 bytes (Offset 32)
    /// Access kind: 0 = None, 1 = Read, 2 = Write, 3 = Execute.
    pub access_kind: u8, // 1 byte  (Offset 34)
    /// Explicit architectural padding to maintain strict 8-byte alignment (Total: 40 bytes).
    pub reserved: [u8; 5], // 5 bytes (Offset 35)
}

impl TraceEvent {
    pub const ZERO: Self = Self {
        timestamp_qpc: 0,
        pc: 0,
        memory_addr: 0,
        prev_value: 0,
        next_value: 0,
        opcode: 0,
        access_kind: 0,
        reserved: [0u8; 5],
    };
}
