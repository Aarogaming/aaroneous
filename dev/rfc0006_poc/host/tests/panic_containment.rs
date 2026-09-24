//! RFC-0006 Section 8, criterion 3: "A plugin that panics inside
//! `plugin_tick` does not crash or corrupt the host process."
//!
//! Only `a_panic_inside_plugin_tick_is_caught_by_the_plugin_itself...`
//! below actually demonstrates that criterion: it runs entirely in-process
//! (this test binary IS the host, with the plugin loaded directly into it,
//! matching RFC-0006's in-process-DLL design - no subprocess involved), and
//! that process is never at risk because the plugin catches its own panic
//! before it can reach the `extern "C"` boundary.
//!
//! The second test, `an_uncaught_panic_takes_down_whatever_process_hosts_it`,
//! is a **control case, not a second proof of criterion 3** - a prior
//! version of this file mistakenly framed it as one. `tick_once` (the child
//! process it spawns) *is* the host for the plugin it loads, exactly the
//! same relationship as the first test's process is to its own plugin; the
//! child dying is not "isolation" protecting some separate host, it is the
//! host itself going down. The child is spawned only so *this test suite's
//! own* process (an unrelated bystander that never loads the panicking
//! plugin at all) doesn't get taken down by the assertion. What it actually
//! establishes is the failure mode RFC-0006's mandatory internal
//! `catch_unwind` exists to prevent: a plugin that panics without catching
//! its own panic takes its entire host process down with it - safely (no
//! memory corruption, per the Section 7 correction in
//! `dev/rfc0006_poc/FINDINGS.md`) but completely.

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
    // in-process, in this very process, with no subprocess isolation
    // needed, because the plugin catches its own panic before it can reach
    // the `extern "C"` boundary. This is the actual criterion-3 proof.
    for frame in 0..5 {
        match plugin.tick() {
            TickOutcome::Faulted => {}
            other => panic!("frame {frame}: expected Faulted, got {other:?}"),
        }
    }
}

#[test]
fn an_uncaught_panic_takes_down_whatever_process_hosts_it() {
    // NOT a criterion-3 proof - see the module doc comment. `tick_once` is
    // the host here; it dying is the point being demonstrated, not the
    // safety property under test.
    let host_bin = tick_once_bin_path();
    let plugin = common::plugin_path("rfc0006_plugin_panicker_raw");

    let status = Command::new(&host_bin)
        .arg(&plugin)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {}: {e}", host_bin.display()));

    assert!(
        !status.success(),
        "an uncaught panic crossing the extern \"C\" boundary should abort the process \
         hosting it (stable Rust's default non-unwinding-FFI behavior) - safely, but \
         completely, which is exactly why RFC-0006 must require the mitigation this \
         PoC's `panicker` fixture (and the test above) demonstrates instead"
    );

    // This test suite's own process, in contrast, never loaded the
    // panicking plugin at all - it spawned a disposable child to do that.
    // It was never at risk, and this just confirms it's still healthy.
    let hello = common::plugin_path("rfc0006_plugin_hello");
    let mut ok_plugin = LoadedPlugin::load(&hello)
        .expect("this test process, which never hosted the panicking plugin, is unaffected");
    match ok_plugin.tick() {
        TickOutcome::Ok(_) => {}
        other => panic!("expected Ok, got {other:?}"),
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
