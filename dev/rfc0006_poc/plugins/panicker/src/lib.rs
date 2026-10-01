//! RFC-0006 PoC fixture: panics inside its own tick logic, but - as
//! RFC-0006 Section 7 requires - catches that panic itself, at its own
//! `extern "C"` boundary, before it can ever try to unwind across it.
//! Verifies criterion 3 (Section 8): a panicking plugin returns a clean
//! `TickResultRaw::faulted()`, never crashing the host process, in-process,
//! with no subprocess isolation needed at all.
//!
//! See `dev/rfc0006_poc/FINDINGS.md` for why this - not "the host wraps the
//! call in `catch_unwind`", RFC-0006's current Section 7 wording - is the
//! only placement that can actually work: once a panic reaches a plain
//! `extern "C"` boundary without being caught, stable Rust already forces a
//! safe process abort before the *caller's* `catch_unwind` ever runs.

use rfc0006_abi::{PLUGIN_ABI_MAX, TickResultRaw};

#[unsafe(no_mangle)]
pub extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_MAX
}

/// # Safety
/// `buf` must be valid for `capacity` writable bytes for the duration of
/// this call - the RFC-0006 host contract's `plugin_tick` precondition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plugin_tick(buf: *mut u8, capacity: usize) -> TickResultRaw {
    // SAFETY: see `plugins/hello` - identical precondition, same host
    // contract. `AssertUnwindSafe` below is sound here specifically because
    // a caught panic makes this whole function return `faulted()`
    // unconditionally, so the host never observes (and this plugin never
    // reuses) whatever partial state `slice` was left in.
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, capacity) };
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        panic!("rfc0006_plugin_panicker: deliberate panic inside plugin_tick");
        #[allow(unreachable_code)]
        {
            let _ = slice;
        }
    }));
    match caught {
        Ok(()) => unreachable!("the closure above always panics"),
        Err(_payload) => TickResultRaw::faulted(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_handle_event(_events: *const u8, _count: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_shutdown() {}
