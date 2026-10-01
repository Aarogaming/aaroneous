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

    // The nested build must not inherit the outer build's CARGO_TARGET_DIR (env
    // var or ~/.cargo/config.toml default): if both point at the same
    // directory, the outer cargo process (which is blocked waiting on this
    // build script) and this child `cargo build` contend for the same
    // exclusive target-directory lock, deadlocking indefinitely. OUT_DIR is
    // guaranteed unique per build-script invocation, so a subdirectory of it
    // can never collide with the parent build's own target directory.
    let plugins_target_dir =
        PathBuf::from(std::env::var("OUT_DIR").expect("Cargo sets OUT_DIR for build scripts"))
            .join("plugins-target");

    let status = Command::new(env!("CARGO"))
        .arg("build")
        .arg("--target-dir")
        .arg(&plugins_target_dir)
        .env_remove("CARGO_TARGET_DIR")
        .current_dir(&plugins_dir)
        .status()
        .expect("failed to invoke `cargo build` for the RFC-0006 PoC plugins workspace");
    assert!(
        status.success(),
        "RFC-0006 PoC plugins workspace failed to build; run `cargo build` in {} directly to see the error",
        plugins_dir.display()
    );

    // Tests need to find the built fixtures; the debug profile is always what
    // this `cargo build` (no --release) produces.
    println!(
        "cargo:rustc-env=RFC0006_PLUGINS_TARGET_DIR={}",
        plugins_target_dir.join("debug").display()
    );
}
