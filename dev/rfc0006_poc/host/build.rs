//! Builds the RFC-0006 PoC plugin fixtures (a separate, deliberately
//! non-member nested workspace under `../plugins` - see its own README) so
//! `host`'s tests can load real, independently-compiled `cdylib` artifacts
//! rather than mocks. `../plugins` is excluded from the root Aaroneous
//! workspace precisely so it can set its own build profile independent of
//! this crate's; see `dev/rfc0006_poc/FINDINGS.md`.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let plugins_dir = manifest_dir.join("../plugins");
    println!("cargo:rerun-if-changed={}", plugins_dir.display());

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .current_dir(&plugins_dir)
        .status()
        .expect("failed to invoke `cargo build` for the RFC-0006 PoC plugins workspace");
    assert!(
        status.success(),
        "RFC-0006 PoC plugins workspace failed to build; run `cargo build` in {} directly to see the error",
        plugins_dir.display()
    );
}
