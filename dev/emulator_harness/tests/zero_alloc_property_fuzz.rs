//! Property-based zero-allocation verification across 50,000 randomized trace event reductions.

use emulator_harness::{ReductionBudget, TraceEvent, reduce_trace_bounded};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct FuzzCountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

// SAFETY: FuzzCountingAllocator delegates directly to System allocator without modifying pointers or layout.
unsafe impl GlobalAlloc for FuzzCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        // SAFETY: Delegating allocation directly to System allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Delegating deallocation directly to System allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: FuzzCountingAllocator = FuzzCountingAllocator;

/// Deterministic LCG pseudo-random number generator.
fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

#[test]
fn test_50k_randomized_traces_zero_heap_allocation() {
    let initial_count = ALLOC_COUNT.load(Ordering::SeqCst);
    let mut state: u64 = 0xDEAD_BEEF_CAFE_BABEu64;
    let target_addr: u64 = 0x4000;

    for _ in 0..50_000 {
        let event0 = TraceEvent {
            timestamp_qpc: next_u64(&mut state),
            pc: next_u64(&mut state),
            memory_addr: target_addr,
            prev_value: (next_u64(&mut state) & 0xFFFF_FFFF) as u32,
            next_value: (next_u64(&mut state) & 0xFFFF_FFFF) as u32,
            opcode: ((next_u64(&mut state) % 4) + 1) as u16,
            access_kind: 2, // Write
            reserved: [0; 5],
        };

        let event1 = TraceEvent {
            timestamp_qpc: next_u64(&mut state),
            pc: next_u64(&mut state),
            memory_addr: target_addr,
            prev_value: event0.next_value,
            next_value: (next_u64(&mut state) & 0xFFFF_FFFF) as u32,
            opcode: ((next_u64(&mut state) % 4) + 1) as u16,
            access_kind: ((next_u64(&mut state) % 3) + 1) as u8,
            reserved: [0; 5],
        };

        let trace = [event0, event1];

        let summary = reduce_trace_bounded(&trace, target_addr, ReductionBudget::with_limit(100))
            .expect("Reduction step must succeed");

        assert_eq!(summary.delta.initial_val, event0.prev_value);
    }

    let final_count = ALLOC_COUNT.load(Ordering::SeqCst);
    assert_eq!(
        initial_count,
        final_count,
        "Zero allocation property violated: {} allocations occurred",
        final_count - initial_count
    );
}
