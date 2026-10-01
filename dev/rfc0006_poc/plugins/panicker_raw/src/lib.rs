//! RFC-0006 PoC fixture: a *non-compliant* plugin - it panics inside
//! `plugin_tick` without catching its own panic, the exact thing RFC-0006
//! Section 7 says every real plugin must not do. It exists only to verify
//! (via `host`'s `tests/panic_containment.rs`, run out-of-process) that
//! letting a panic reach the raw `extern "C"` boundary triggers a *safe*
//! process abort under modern stable Rust - not memory corruption - even
//! though it still takes the whole host process down with it, which is
//! exactly why RFC-0006 must require the in-process catch (`panicker`,
//! this crate's sibling) rather than relying on this fallback. See
//! `dev/rfc0006_poc/FINDINGS.md`.

use rfc0006_abi::{PLUGIN_ABI_MAX, TickResultRaw};

#[unsafe(no_mangle)]
pub extern "C" fn plugin_abi_version() -> u32 {
    PLUGIN_ABI_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_tick(_buf: *mut u8, _capacity: usize) -> TickResultRaw {
    panic!("rfc0006_plugin_panicker_raw: deliberate, deliberately UNCAUGHT panic");
}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_handle_event(_events: *const u8, _count: usize) {}

#[unsafe(no_mangle)]
pub extern "C" fn plugin_shutdown() {}
