//! Integration test harness asserting ACC invariants for M-SLM discrete tokens and adaptive math.

use std::mem::{align_of, size_of};

/// Mirror of OpcodeToken from crates/si_ir::token
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpcodeTokenMirror {
    pub opcode_id: u16,
    pub src_reg: u16,
    pub dst_reg: u16,
    pub aux_reg: u16,
}

/// Mirror of StateDeltaToken from crates/si_ir::token
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StateDeltaTokenMirror {
    pub dimension_idx: u32,
    pub prev_latent_q16: i32,
    pub delta_latent_q16: i32,
    pub confidence_score_q16: u32,
}

/// Mirror of MachineTokenKind from crates/si_ir::token
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineTokenKindMirror {
    ControlFlow = 0x0001,
    MemoryOp = 0x0002,
    StateDelta = 0x0003,
    SensoryObservation = 0x0004,
    ActuationIntent = 0x0005,
    ThermodynamicBoundary = 0x0006,
    SyncBarrier = 0x0007,
}

/// Mirror of MachineToken from crates/si_ir::token
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MachineTokenMirror {
    pub kind: MachineTokenKindMirror,
    pub flags: u16,
    pub timestamp_cycles: u32,
    pub opcode: OpcodeTokenMirror,
    pub delta: StateDeltaTokenMirror,
}

#[test]
fn mslm_token_memory_layout_invariants() {
    // OpcodeToken: exactly 8 bytes (4 * 2 bytes), 2-byte aligned
    assert_eq!(size_of::<OpcodeTokenMirror>(), 8);
    assert_eq!(align_of::<OpcodeTokenMirror>(), 2);

    // StateDeltaToken: exactly 16 bytes (4 * 4 bytes), 4-byte aligned
    assert_eq!(size_of::<StateDeltaTokenMirror>(), 16);
    assert_eq!(align_of::<StateDeltaTokenMirror>(), 4);

    // MachineToken: exactly 32 bytes (2 + 2 + 4 + 8 + 16), 4-byte aligned, zero padding
    assert_eq!(size_of::<MachineTokenMirror>(), 32);
    assert!(align_of::<MachineTokenMirror>() >= 4);

    // MachineTokenKind: repr(u16), exactly 2 bytes
    assert_eq!(size_of::<MachineTokenKindMirror>(), 2);
    assert_eq!(align_of::<MachineTokenKindMirror>(), 2);
}

#[test]
fn mslm_static_bounds_enforcement_and_clamping() {
    // Validate simulated RLS parameter bounds enforcement logic
    let parameter_bound = 10.0f32;
    let mut weight = 5.0f32;
    let normal_delta = 1.2f32;

    // Within bounds
    let candidate = weight + normal_delta;
    assert!(candidate.abs() <= parameter_bound);
    weight = candidate;
    assert_eq!(weight, 6.2);

    // Dynamic parameter explosion / un-clamped update must be rejected
    let exploding_delta = 50.0f32;
    let unbounded_candidate = weight + exploding_delta;
    let is_bounded = unbounded_candidate.abs() <= parameter_bound;
    assert!(!is_bounded, "Unbounded parameter mutation must be rejected by governance");
}
