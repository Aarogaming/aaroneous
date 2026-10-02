//! Property-based zero-allocation verification for IPC ring buffer events.

use ipc_bus::IpcEvent;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct IpcCountingAllocator;

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    // Scopes the counter to the test's own thread, so allocations made by
    // unrelated threads the process may start (e.g. the test harness's own
    // worker/timeout machinery) can't be mistaken for allocations made by
    // the hot-path code this test is actually measuring.
    static COUNTING_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

// SAFETY: IpcCountingAllocator delegates directly to System allocator without modifying pointers or layout.
unsafe impl GlobalAlloc for IpcCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING_ACTIVE.with(Cell::get) {
            ALLOC_COUNT.fetch_add(1, Ordering::SeqCst);
        }
        // SAFETY: Delegating allocation directly to System allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Delegating deallocation directly to System allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static A: IpcCountingAllocator = IpcCountingAllocator;

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

#[test]
fn test_20k_ipc_events_zero_heap_allocation() {
    COUNTING_ACTIVE.with(|active| active.set(true));
    let initial_allocs = ALLOC_COUNT.load(Ordering::SeqCst);
    let mut state: u64 = 0xCAFE_BABE_1234_5678u64;

    for i in 0..20_000 {
        let event = IpcEvent::new(
            next_u64(&mut state),
            (next_u64(&mut state) & 0xFFFF) as u16,
            (next_u64(&mut state) & 0xFFFF) as u16,
            (next_u64(&mut state) & 0xFFFF_FFFF) as u32,
            (next_u64(&mut state) & 0xFFFF_FFFF) as u32,
        );

        assert_eq!(bytemuck::bytes_of(&event).len(), 24);

        if i % 5_000 == 0 {
            let current_allocs = ALLOC_COUNT.load(Ordering::SeqCst);
            assert_eq!(
                initial_allocs, current_allocs,
                "Heap allocation detected at iteration {}",
                i
            );
        }
    }

    let final_allocs = ALLOC_COUNT.load(Ordering::SeqCst);
    COUNTING_ACTIVE.with(|active| active.set(false));
    assert_eq!(
        initial_allocs, final_allocs,
        "IPC event property test must perform 0 heap allocations! (Before: {}, After: {})",
        initial_allocs, final_allocs
    );
}
