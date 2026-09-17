//! RFC-0006 PoC fixture: reports an ABI version outside the host's
//! supported range. Verifies criterion 2 (Section 8) - a version-mismatched
//! plugin is refused at load with a clear reason, never a crash or a
//! silent no-op. Never dereferences the tick buffer pointer; the only
//! `unsafe` this crate needs at all is opting into the `#[no_mangle]`
//! attribute edition 2024 requires marking explicitly.

use rfc0006_abi::{PLUGIN_ABI_MAX, TickResultRaw};

#[unsafe(no_mangle)]
pub extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_MAX + 1 // deliberately unsupported
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_tick(_buf: *mut u8, _capacity: usize) -> TickResultRaw {
    // Unreachable in a correct host: `load` must refuse this plugin at the
    // version check, before any tick is possible.
    TickResultRaw::faulted()
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_handle_event(_events: *const u8, _count: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_shutdown() {}
