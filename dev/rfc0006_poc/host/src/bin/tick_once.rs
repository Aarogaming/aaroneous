//! Standalone helper for `tests/panic_containment.rs`: loads the plugin
//! named in `argv[1]`, ticks it once, and exits 0. Run as a *child* process
//! specifically so a plugin that panics without catching its own panic
//! (`rfc0006_plugin_panicker_raw`) can be observed aborting that child
//! without ever touching the actual test harness process - the safety
//! property under test genuinely needs a process boundary, not just a
//! `catch_unwind` in the same process (see `panicker_raw`'s doc comment and
//! `dev/rfc0006_poc/FINDINGS.md` for why).

use std::path::PathBuf;

use rfc0006_host::LoadedPlugin;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: tick_once <plugin .so path>");
    let mut plugin = LoadedPlugin::load(&PathBuf::from(path)).expect("tick_once: load failed");
    let _ = plugin.tick();
    println!("tick_once: completed without aborting");
}
