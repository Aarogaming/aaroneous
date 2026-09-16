//! Sustained 100,000-iteration zero-allocation memory leak & RSS stability test.

use emulator_harness::{ReductionBudget, TraceEvent, reduce_trace_bounded};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct SustainedCountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static DEALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

// SAFETY: SustainedCountingAllocator delegates directly to System allocator without modifying pointers or layout.
unsafe impl GlobalAlloc for SustainedCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        // SAFETY: Delegating allocation request directly to system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        // SAFETY: Delegating deallocation request directly to system allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: SustainedCountingAllocator = SustainedCountingAllocator;

#[test]
fn test_sustained_100k_iterations_zero_heap_allocation() {
    let trace = [
        TraceEvent {
            timestamp_qpc: 10,
            pc: 0x2000,
            memory_addr: 0x8000,
            prev_value: 0,
            next_value: 50,
            opcode: 1,
            access_kind: 2, // Write
            reserved: [0; 5],
        },
        TraceEvent {
            timestamp_qpc: 20,
            pc: 0x2004,
            memory_addr: 0x8000,
            prev_value: 50,
            next_value: 50,
            opcode: 2,
            access_kind: 1, // Read
            reserved: [0; 5],
        },
        TraceEvent {
            timestamp_qpc: 30,
            pc: 0x2008,
            memory_addr: 0x8000,
            prev_value: 50,
            next_value: 12345,
            opcode: 3,
            access_kind: 2, // Write
            reserved: [0; 5],
        },
    ];

    let allocs_before = ALLOC_COUNT.load(Ordering::SeqCst);

    for i in 0..100_000 {
        let summary = reduce_trace_bounded(&trace, 0x8000, ReductionBudget::with_limit(100))
            .expect("Reduction step must succeed");
        assert_eq!(summary.delta.final_val, 12345);
        if i % 25_000 == 0 {
            let current_allocs = ALLOC_COUNT.load(Ordering::SeqCst);
            assert_eq!(
                allocs_before, current_allocs,
                "Heap allocation detected at iteration {}",
                i
            );
        }
    }

    let allocs_after = ALLOC_COUNT.load(Ordering::SeqCst);

    assert_eq!(
        allocs_before, allocs_after,
        "Sustained 100,000 iterations must perform ZERO dynamic heap allocations! (Before: {}, After: {})",
        allocs_before, allocs_after
    );
}
