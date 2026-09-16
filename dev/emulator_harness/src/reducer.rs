//! Pure-safe trace reducer for extracting state deltas from contiguous memory buffers.
//! Operates with zero heap allocations and bounded iteration.

use crate::types::TraceEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceError {
    EmptyTraceBuffer,
    NoStateTransitionFound,
    InvalidAccessType,
    BudgetExceeded { max_events: usize, processed: usize },
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

/// Execution budget configuration for bounded trace reduction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionBudget {
    /// Maximum number of trace events to scan in a single reduction pass.
    pub max_events: usize,
}

impl ReductionBudget {
    pub const UNLIMITED: Self = Self {
        max_events: usize::MAX,
    };

    pub const fn with_limit(max_events: usize) -> Self {
        Self { max_events }
    }
}

/// Telemetry metrics produced by a bounded reduction scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReductionSummary {
    pub delta: StateDelta,
    pub events_processed: usize,
    pub writes_observed: usize,
    pub reads_observed: usize,
}

/// Slices an execution trace slice to extract the net state mutation on a given address.
/// Guaranteed O(N) bounded iteration, zero-allocation, and panic-free.
#[doc = "hot_path"]
pub fn extract_state_delta(
    events: &[TraceEvent],
    target_addr: u64,
) -> Result<StateDelta, TraceError> {
    reduce_trace_bounded(events, target_addr, ReductionBudget::UNLIMITED)
        .map(|summary| summary.delta)
}

/// Bounded deterministic execution-host reduction pass.
/// Scans events within the given budget, computing the net state mutation
/// and observation telemetry with strictly zero heap allocations.
#[doc = "hot_path"]
pub fn reduce_trace_bounded(
    events: &[TraceEvent],
    target_addr: u64,
    budget: ReductionBudget,
) -> Result<ReductionSummary, TraceError> {
    if events.is_empty() {
        return Err(TraceError::EmptyTraceBuffer);
    }

    let mut initial: Option<u32> = None;
    let mut final_val = 0u32;
    let mut write_count = 0u32;
    let mut read_count = 0u32;
    let mut processed = 0usize;

    for event in events {
        if processed >= budget.max_events {
            return Err(TraceError::BudgetExceeded {
                max_events: budget.max_events,
                processed,
            });
        }
        processed = processed.saturating_add(1);

        if event.memory_addr == target_addr {
            match event.access_kind {
                // 1 = Read
                1 => {
                    read_count = read_count.saturating_add(1);
                }
                // 2 = Write
                2 => {
                    if initial.is_none() {
                        initial = Some(event.prev_value);
                    }
                    final_val = event.next_value;
                    write_count = write_count.saturating_add(1);
                }
                _ => {}
            }
        }
    }

    match initial {
        Some(init) => Ok(ReductionSummary {
            delta: StateDelta {
                target_addr,
                initial_val: init,
                final_val,
                write_count,
                reserved: 0,
            },
            events_processed: processed,
            writes_observed: write_count as usize,
            reads_observed: read_count as usize,
        }),
        None => Err(TraceError::NoStateTransitionFound),
    }
}
