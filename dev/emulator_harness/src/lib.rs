//! # Emulator Harness (Trace-Driven Semantic Lifter)
//!
//! Provides zero-allocation trace capture, memory-taint slicing,
//! and state transition extraction for assimilating legacy binaries
//! and foreign code into certified Aaroneous substrates.

pub mod reducer;
pub mod types;

pub use reducer::{StateDelta, TraceError, extract_state_delta};
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
}
