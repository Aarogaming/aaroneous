#![deny(unsafe_code)]

//! RFC-0006 PoC fixture: a well-behaved plugin exporting one label and one
//! clickable button. Verifies criterion 1 (Section 8) - a minimal plugin
//! loads, ticks for N frames, and unloads cleanly.

use rfc0006_abi::{CommandBufferWriter, CommandOp, PLUGIN_ABI_MAX, TickResultRaw};

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_MAX
}

/// # Safety
/// `buf` must be valid for `capacity` writable bytes for the duration of
/// this call - the RFC-0006 host contract's `plugin_tick` precondition.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub unsafe extern "C" fn plugin_tick(buf: *mut u8, capacity: usize) -> TickResultRaw {
    // SAFETY: the RFC-0006 host contract guarantees `buf` is valid for
    // `capacity` writable bytes for the duration of this call. This is the
    // plugin's only unsafe operation; everything past this point is safe
    // Rust operating on an ordinary `&mut [u8]`.
    let slice = unsafe { std::slice::from_raw_parts_mut(buf, capacity) };
    write_commands(slice)
}

fn write_commands(slice: &mut [u8]) -> TickResultRaw {
    let Some(mut w) = CommandBufferWriter::new(slice) else {
        return TickResultRaw::overflow();
    };
    let ok = w.push(
        CommandOp::Label,
        0,
        "Hello from a real cdylib!",
        8.0,
        8.0,
        [255, 255, 255, 255],
    ) && w.push(CommandOp::Button, 1, "Click me", 8.0, 28.0, [64, 160, 255, 255]);
    if !ok {
        return TickResultRaw::overflow();
    }
    TickResultRaw::ok(w.finish())
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn plugin_handle_event(_events: *const u8, _count: usize) {}

/// Returns this plugin's Base36 virtual-branch lineage stamp, supplied by the
/// host build as an explicit input. Independently built fixtures use an empty
/// stamp and are refused by a stamped host rather than inheriting ambient state.
/// The host checks this against its own stamp before allowing the cartridge
/// to tick — a mismatch means the plugin was built on a different branch and
/// is refused immediately, before any state-reducing code runs.
#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn plugin_lineage_stamp() -> *const std::ffi::c_char {
    static STAMP_WITH_NUL: std::sync::OnceLock<std::ffi::CString> = std::sync::OnceLock::new();
    STAMP_WITH_NUL
        .get_or_init(|| {
            let stamp = option_env!("AARONEOUS_BUILD_LINEAGE").unwrap_or("");
            std::ffi::CString::new(stamp).unwrap_or_default()
        })
        .as_ptr()
}

#[unsafe(no_mangle)]
#[allow(unsafe_code)]
pub extern "C" fn plugin_shutdown() {}
