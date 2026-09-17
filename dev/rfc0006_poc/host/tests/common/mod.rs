use std::path::PathBuf;

/// Locates a PoC plugin fixture built by `host`'s `build.rs` into the
/// separate `../plugins` workspace's own `target/debug`.
pub fn plugin_path(crate_name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../plugins/target/debug")
        .join(format!("lib{crate_name}.so"));
    assert!(
        path.exists(),
        "expected plugin fixture built at {} - host's build.rs should have built \
         the ../plugins workspace before this test ran",
        path.display()
    );
    path
}
