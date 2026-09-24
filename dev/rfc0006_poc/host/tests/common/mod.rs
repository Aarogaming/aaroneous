use std::path::PathBuf;

/// Locates a PoC plugin fixture built by `host`'s `build.rs` into the
/// separate `../plugins` workspace's own `target/debug`. Uses
/// `std::env::consts::DLL_PREFIX`/`DLL_SUFFIX` rather than a hardcoded
/// `lib*.so`, since Cargo emits `{name}.dll` (no `lib` prefix) on Windows
/// and `lib{name}.dylib` on macOS - this repo's CI runs both Linux and
/// Windows (`.github/workflows/ci.yml`'s `windows-latest` leg), so a
/// Unix-only assumption here would fail every test on that runner.
pub fn plugin_path(crate_name: &str) -> PathBuf {
    let filename = format!(
        "{}{crate_name}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../plugins/target/debug")
        .join(filename);
    assert!(
        path.exists(),
        "expected plugin fixture built at {} - host's build.rs should have built \
         the ../plugins workspace before this test ran",
        path.display()
    );
    path
}
