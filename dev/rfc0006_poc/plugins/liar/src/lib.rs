//! RFC-0006 PoC fixture: writes a header claiming `u32::MAX` commands (far
//! more than could possibly fit in the buffer) and reports having used the
//! entire capacity, while actually writing only the 16-byte header. Verifies
//! criterion 4 (Section 8) - the host must truncate this claim to what the
//! buffer really holds, never read past it.

use rfc0006_abi::{HEADER_LEN, TickResultRaw, WIRE_VERSION};

#[unsafe(no_mangle)]
pub extern "C" fn plugin_abi_version() -> u32 {
    rfc0006_abi::PLUGIN_ABI_MAX
}

/// # Safety
/// `buf` must be valid for `capacity` writable bytes for the duration of
/// this call - the RFC-0006 host contract's `plugin_tick` precondition.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn plugin_tick(buf: *mut u8, capacity: usize) -> TickResultRaw {
    // SAFETY: identical precondition to `plugins/hello` - `buf` is valid
    // for `capacity` writable bytes for the duration of this call. Every
    // write below stays within `slice`'s own bounds (checked by ordinary
    // slice indexing, which panics rather than reading/writing
    // out-of-bounds if this fixture's own arithmetic were ever wrong).
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, capacity) };
    if slice.len() < HEADER_LEN {
        return TickResultRaw::overflow();
    }
    slice[0..4].copy_from_slice(&WIRE_VERSION.to_le_bytes());
    slice[4..8].copy_from_slice(&u32::MAX.to_le_bytes()); // lie #1: command_count
    slice[8..12].copy_from_slice(&u32::MAX.to_le_bytes()); // lie #2: payload_len
    slice[12..16].copy_from_slice(&0u32.to_le_bytes());
    // lie #3: claims it filled the entire buffer, though only the 16-byte
    // header above was actually written.
    TickResultRaw::ok(capacity as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_handle_event(_events: *const u8, _count: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_shutdown() {}
