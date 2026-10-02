use std::path::PathBuf;

/// Locates a PoC plugin fixture built by `host`'s `build.rs`. The plugins
/// workspace is built into a subdirectory of this crate's own `OUT_DIR`
/// (never the outer build's target directory - see `build.rs` for why: a
/// shared target directory lets the outer cargo process and this nested one
/// deadlock on the same exclusive lock) and its exact path is passed through
/// at compile time via `RFC0006_PLUGINS_TARGET_DIR`. Uses
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
    let path = PathBuf::from(env!("RFC0006_PLUGINS_TARGET_DIR")).join(filename);
    assert!(
        path.exists(),
        "expected plugin fixture built at {} - host's build.rs should have built \
         the ../plugins workspace before this test ran",
        path.display()
    );
    path
}
