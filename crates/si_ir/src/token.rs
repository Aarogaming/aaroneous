//! crates/si_ir/src/token.rs
//! Discrete Machine Token Vocabulary & Grammar for the .si Machine-Native State Language Model (M-SLM).
//!
//! Enforces zero-allocation, strictly packed, #[repr(C)] or #[repr(u16)] structures
//! to guarantee binary portability, cache-line efficiency, and zero deserialization overhead.

use serde::{Deserialize, Serialize};

/// High-level discrete machine token category identifier.
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MachineTokenKind {
    ControlFlow = 0x0001,
    MemoryOp = 0x0002,
    StateDelta = 0x0003,
    SensoryObservation = 0x0004,
    ActuationIntent = 0x0005,
    ThermodynamicBoundary = 0x0006,
    SyncBarrier = 0x0007,
}

/// Discrete machine opcode token specifying register assignments and execution semantics.
/// Size: 8 bytes, tightly packed, zero heap allocation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OpcodeToken {
    pub opcode_id: u16,
    pub src_reg: u16,
    pub dst_reg: u16,
    pub aux_reg: u16,
}

impl OpcodeToken {
    pub const fn new(opcode_id: u16, src_reg: u16, dst_reg: u16, aux_reg: u16) -> Self {
        Self {
            opcode_id,
            src_reg,
            dst_reg,
            aux_reg,
        }
    }
}

/// Delta transition token for state trajectories and dimensional updates.
/// Size: 16 bytes, tightly packed.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StateDeltaToken {
    pub dimension_idx: u32,
    pub prev_latent_q16: i32,
    pub delta_latent_q16: i32,
    pub confidence_score_q16: u32,
}

impl StateDeltaToken {
    pub const fn new(
        dimension_idx: u32,
        prev_latent_q16: i32,
        delta_latent_q16: i32,
        confidence_score_q16: u32,
    ) -> Self {
        Self {
            dimension_idx,
            prev_latent_q16,
            delta_latent_q16,
            confidence_score_q16,
        }
    }
}

/// The fundamental discrete token envelope for the M-SLM stream.
/// Size: 32 bytes, 8-byte aligned, zero heap allocation.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MachineToken {
    pub kind: MachineTokenKind,     // 2 bytes
    pub flags: u16,                  // 2 bytes
    pub timestamp_cycles: u32,       // 4 bytes
    pub opcode: OpcodeToken,         // 8 bytes
    pub delta: StateDeltaToken,      // 16 bytes
}

impl MachineToken {
    pub const fn from_opcode(opcode: OpcodeToken, cycles: u32) -> Self {
        Self {
            kind: MachineTokenKind::ControlFlow,
            flags: 0,
            timestamp_cycles: cycles,
            opcode,
            delta: StateDeltaToken::new(0, 0, 0, 0),
        }
    }

    pub const fn from_delta(delta: StateDeltaToken, cycles: u32) -> Self {
        Self {
            kind: MachineTokenKind::StateDelta,
            flags: 0,
            timestamp_cycles: cycles,
            opcode: OpcodeToken::new(0, 0, 0, 0),
            delta,
        }
    }
}
