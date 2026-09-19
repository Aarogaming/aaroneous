//! Integration test measuring heap allocations during bounded trace reduction.
//! Proves zero heap allocation during execution of the bounded reducer reference path.

use emulator_harness::{ReductionBudget, TraceEvent, extract_state_delta, reduce_trace_bounded};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct CountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        // Pure pass-through to `System`'s own `alloc` with the exact same
        // `layout` this fn was called with; `GlobalAlloc::alloc`'s
        // precondition (non-zero-size, validly-constructed `Layout`) is
        // therefore satisfied by whichever caller upheld this fn's own
        // documented safety contract - this wrapper adds nothing that
        // could violate it.
        // SAFETY: layout is forwarded unchanged from this fn's own caller-checked precondition; see rationale above.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        // Pure pass-through to `System`'s own `dealloc` with the exact
        // same `ptr`/`layout` this fn was called with; `GlobalAlloc::
        // dealloc`'s precondition (`ptr` was allocated by this allocator
        // with a matching `layout`) is therefore satisfied by whichever
        // caller upheld this fn's own documented safety contract.
        // SAFETY: ptr/layout are forwarded unchanged from this fn's own caller-checked precondition; see rationale above.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: CountingAllocator = CountingAllocator;

#[test]
fn test_bounded_reducer_zero_heap_allocation_measurement() {
    let trace = [
        TraceEvent {
            timestamp_qpc: 1,
            pc: 0x1000,
            memory_addr: 0x4000,
            prev_value: 0,
            next_value: 10,
            opcode: 1,
            access_kind: 2, // Write
            reserved: [0; 5],
        },
        TraceEvent {
            timestamp_qpc: 2,
            pc: 0x1004,
            memory_addr: 0x4000,
            prev_value: 10,
            next_value: 10,
            opcode: 2,
            access_kind: 1, // Read
            reserved: [0; 5],
        },
        TraceEvent {
            timestamp_qpc: 3,
            pc: 0x1008,
            memory_addr: 0x4000,
            prev_value: 10,
            next_value: 99,
            opcode: 3,
            access_kind: 2, // Write
            reserved: [0; 5],
        },
    ];

    // Measure allocations across reduction execution
    let allocs_before = ALLOC_COUNT.load(Ordering::SeqCst);

    let summary = reduce_trace_bounded(&trace, 0x4000, ReductionBudget::with_limit(100))
        .expect("Reduction should succeed");

    let allocs_after = ALLOC_COUNT.load(Ordering::SeqCst);

    assert_eq!(
        allocs_before, allocs_after,
        "reduce_trace_bounded must perform ZERO dynamic heap allocations! (Before: {}, After: {})",
        allocs_before, allocs_after
    );

    assert_eq!(summary.delta.initial_val, 0);
    assert_eq!(summary.delta.final_val, 99);
    assert_eq!(summary.delta.write_count, 2);
    assert_eq!(summary.events_processed, 3);
    assert_eq!(summary.writes_observed, 2);
    assert_eq!(summary.reads_observed, 1);

    // Also verify extract_state_delta has zero heap allocations
    let allocs_before_extract = ALLOC_COUNT.load(Ordering::SeqCst);
    let delta = extract_state_delta(&trace, 0x4000).expect("Extract should succeed");
    let allocs_after_extract = ALLOC_COUNT.load(Ordering::SeqCst);

    assert_eq!(
        allocs_before_extract, allocs_after_extract,
        "extract_state_delta must perform ZERO dynamic heap allocations!"
    );
    assert_eq!(delta.initial_val, 0);
    assert_eq!(delta.final_val, 99);
}
