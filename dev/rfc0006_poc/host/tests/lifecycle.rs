//! RFC-0006 Section 8, criteria 1 & 2.

mod common;

use rfc0006_host::{LoadError, LoadedPlugin, TickOutcome};

#[test]
fn hello_plugin_loads_ticks_and_unloads_cleanly_across_many_cycles() {
    let path = common::plugin_path("rfc0006_plugin_hello");

    // Repeated load/unload cycles: each `LoadedPlugin` fully drops (running
    // `plugin_shutdown` then `dlclose`-ing via `libloading::Library`'s own
    // `Drop`) before the next iteration's `load` call, so a real leak here
    // would show up as ever-growing open-handle/mapped-library state (e.g.
    // `/proc/self/maps` on Linux) across iterations, not just "it ran once."
    for cycle in 0..20 {
        let mut plugin =
            LoadedPlugin::load(&path).unwrap_or_else(|e| panic!("cycle {cycle}: load failed: {e}"));
        for frame in 0..5 {
            match plugin.tick() {
                TickOutcome::Ok(decoded) => {
                    assert_eq!(decoded.commands.len(), 2, "cycle {cycle} frame {frame}");
                    assert_eq!(decoded.truncated_commands, 0);
                    assert_eq!(decoded.skipped_unknown_ops, 0);
                }
                other => panic!("cycle {cycle} frame {frame}: unexpected outcome {other:?}"),
            }
        }
    }
}

#[test]
fn version_mismatched_plugin_is_refused_at_load_not_crashed_or_silently_accepted() {
    let path = common::plugin_path("rfc0006_plugin_bad_version");

    match LoadedPlugin::load(&path) {
        Err(LoadError::UnsupportedAbiVersion { reported, min, max }) => {
            assert!(
                reported > max || reported < min,
                "reported {reported} should be outside [{min}, {max}]"
            );
        }
        Err(other) => {
            panic!("expected LoadError::UnsupportedAbiVersion, got a different error: {other}")
        }
        Ok(_) => {
            panic!("a plugin reporting an out-of-range ABI version must not be allowed to load")
        }
    }
}
