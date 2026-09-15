//! Pure-safe trace reducer for extracting state deltas from contiguous memory buffers.
//! Operates with zero heap allocations and bounded iteration.

use crate::types::TraceEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceError {
    EmptyTraceBuffer,
    NoStateTransitionFound,
    InvalidAccessType,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct StateDelta {
    pub target_addr: u64,
    pub initial_val: u32,
    pub final_val: u32,
    pub write_count: u32,
    pub reserved: u32, // Explicit 8-byte alignment padding
}

/// Slices an execution trace slice to extract the net state mutation on a given address.
/// Guaranteed O(N) bounded iteration, zero-allocation, and panic-free.
#[doc = "hot_path"]
pub fn extract_state_delta(
    events: &[TraceEvent],
    target_addr: u64,
) -> Result<StateDelta, TraceError> {
    if events.is_empty() {
        return Err(TraceError::EmptyTraceBuffer);
    }

    let mut initial: Option<u32> = None;
    let mut final_val = 0u32;
    let mut write_count = 0u32;

    for event in events {
        // Filter strictly for memory writes (access_kind == 2)
        if event.memory_addr == target_addr && event.access_kind == 2 {
            if initial.is_none() {
                initial = Some(event.prev_value);
            }
            final_val = event.next_value;
            write_count = write_count.saturating_add(1);
        }
    }

    match initial {
        Some(init) => Ok(StateDelta {
            target_addr,
            initial_val: init,
            final_val,
            write_count,
            reserved: 0,
        }),
        None => Err(TraceError::NoStateTransitionFound),
    }
}
