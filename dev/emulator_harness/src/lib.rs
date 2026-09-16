//! # Emulator Harness (Trace-Driven Semantic Lifter)
//!
//! Provides zero-allocation trace capture, memory-taint slicing,
//! and state transition extraction for assimilating legacy binaries
//! and foreign code into certified Aaroneous substrates.

pub mod reducer;
pub mod types;

pub use reducer::{
    ReductionBudget, ReductionSummary, StateDelta, TraceError, extract_state_delta,
    reduce_trace_bounded,
};
pub use types::TraceEvent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_event_pod_layout() {
        assert_eq!(core::mem::size_of::<TraceEvent>(), 40);
        assert_eq!(core::mem::align_of::<TraceEvent>(), 8);

        let event = TraceEvent::ZERO;
        let bytes = bytemuck::bytes_of(&event);
        let back: &TraceEvent = bytemuck::from_bytes(bytes);
        assert_eq!(back.timestamp_qpc, 0);
    }

    #[test]
    fn test_zero_allocation_reduction() {
        let events = [
            TraceEvent {
                timestamp_qpc: 10,
                pc: 0x1000,
                memory_addr: 0xCAFE,
                prev_value: 0,
                next_value: 42,
                opcode: 1,
                access_kind: 2, // Write
                reserved: [0; 5],
            },
            TraceEvent {
                timestamp_qpc: 20,
                pc: 0x1004,
                memory_addr: 0xCAFE,
                prev_value: 42,
                next_value: 100,
                opcode: 1,
                access_kind: 2, // Write
                reserved: [0; 5],
            },
        ];

        let delta = extract_state_delta(&events, 0xCAFE).expect("Failed to reduce delta");
        assert_eq!(delta.initial_val, 0);
        assert_eq!(delta.final_val, 100);
        assert_eq!(delta.write_count, 2);
    }

    #[test]
    fn test_bounded_reducer_budget_and_replay() {
        let golden_trace = [
            TraceEvent {
                timestamp_qpc: 100,
                pc: 0x2000,
                memory_addr: 0xBEEF,
                prev_value: 0,
                next_value: 0,
                opcode: 0x10,
                access_kind: 1, // Read
                reserved: [0; 5],
            },
            TraceEvent {
                timestamp_qpc: 105,
                pc: 0x2004,
                memory_addr: 0xBEEF,
                prev_value: 0,
                next_value: 0xAA,
                opcode: 0x11,
                access_kind: 2, // Write
                reserved: [0; 5],
            },
            TraceEvent {
                timestamp_qpc: 110,
                pc: 0x2008,
                memory_addr: 0xDEAD, // Unrelated address
                prev_value: 0,
                next_value: 1,
                opcode: 0x12,
                access_kind: 2,
                reserved: [0; 5],
            },
            TraceEvent {
                timestamp_qpc: 115,
                pc: 0x200C,
                memory_addr: 0xBEEF,
                prev_value: 0xAA,
                next_value: 0xBB,
                opcode: 0x13,
                access_kind: 2, // Write
                reserved: [0; 5],
            },
        ];

        // 1. Deterministic trace replay with full budget
        let summary1 = reduce_trace_bounded(&golden_trace, 0xBEEF, ReductionBudget::UNLIMITED)
            .expect("Reduction should succeed");
        let summary2 = reduce_trace_bounded(&golden_trace, 0xBEEF, ReductionBudget::with_limit(10))
            .expect("Reduction should succeed within budget");

        assert_eq!(summary1, summary2);
        assert_eq!(summary1.delta.initial_val, 0);
        assert_eq!(summary1.delta.final_val, 0xBB);
        assert_eq!(summary1.delta.write_count, 2);
        assert_eq!(summary1.writes_observed, 2);
        assert_eq!(summary1.reads_observed, 1);
        assert_eq!(summary1.events_processed, 4);

        // 2. Budget exhaustion check
        let err = reduce_trace_bounded(&golden_trace, 0xBEEF, ReductionBudget::with_limit(2))
            .expect_err("Budget limit should trigger");
        assert_eq!(
            err,
            TraceError::BudgetExceeded {
                max_events: 2,
                processed: 2
            }
        );
    }
}
