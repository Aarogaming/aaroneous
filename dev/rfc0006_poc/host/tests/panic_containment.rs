//! RFC-0006 Section 8, criterion 3.

mod common;

use std::path::PathBuf;
use std::process::Command;

use rfc0006_host::{LoadedPlugin, TickOutcome};

#[test]
fn a_panic_inside_plugin_tick_is_caught_by_the_plugin_itself_and_reported_as_faulted() {
    let path = common::plugin_path("rfc0006_plugin_panicker");
    let mut plugin =
        LoadedPlugin::load(&path).expect("panicker should load: it reports a valid ABI version");

    // Repeated panicking ticks never escalate past a clean `Faulted` result -
    // in-process, with no subprocess isolation needed, because the plugin
    // catches its own panic before it can reach the `extern "C"` boundary.
    for frame in 0..5 {
        match plugin.tick() {
            TickOutcome::Faulted => {}
            other => panic!("frame {frame}: expected Faulted, got {other:?}"),
        }
    }
}

#[test]
fn an_uncaught_panic_at_the_extern_c_boundary_safely_aborts_only_the_child_process() {
    let host_bin = tick_once_bin_path();
    let plugin = common::plugin_path("rfc0006_plugin_panicker_raw");

    let status = Command::new(&host_bin)
        .arg(&plugin)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", host_bin.display()));

    assert!(
        !status.success(),
        "an uncaught panic crossing the extern \"C\" boundary should abort the child \
         process (stable Rust's default non-unwinding-FFI behavior), not exit 0"
    );

    // The parent - this test process - is demonstrably unaffected: it can
    // still load and tick a well-behaved plugin normally right afterward.
    let hello = common::plugin_path("rfc0006_plugin_hello");
    let mut ok_plugin =
        LoadedPlugin::load(&hello).expect("host process is healthy after the child's abort");
    match ok_plugin.tick() {
        TickOutcome::Ok(_) => {}
        other => panic!("expected Ok after the unrelated child's abort, got {other:?}"),
    }
}

fn tick_once_bin_path() -> PathBuf {
    let mut path = std::env::current_exe().expect("current_exe");
    path.pop(); // drop the test binary's own file name
    if path.ends_with("deps") {
        path.pop(); // -> target/<profile>
    }
    path.push(if cfg!(windows) {
        "tick_once.exe"
    } else {
        "tick_once"
    });
    assert!(
        path.exists(),
        "expected the `tick_once` helper binary built alongside this test at {}",
        path.display()
    );
    path
}
