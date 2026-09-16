//! Compile-time/test-time syntax validation for WGSL compute shaders.

use std::path::PathBuf;

#[test]
fn test_wgsl_shaders_parse_cleanly() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crates parent")
        .parent()
        .expect("workspace root");

    let shader_dir = workspace_root.join("shaders");
    assert!(
        shader_dir.exists(),
        "Shaders directory missing at {:?}",
        shader_dir
    );

    let entries = std::fs::read_dir(&shader_dir).expect("read shaders dir");
    let mut tested_count = 0;

    for entry in entries {
        let entry = entry.expect("valid dir entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("wgsl") {
            let shader_source = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read shader file {:?}: {}", path, e));

            match naga::front::wgsl::parse_str(&shader_source) {
                Ok(_) => {
                    println!(
                        "Successfully validated WGSL shader: {:?}",
                        path.file_name().unwrap()
                    );
                    tested_count += 1;
                }
                Err(err) => {
                    panic!(
                        "WGSL syntax error in {:?}:\n{}",
                        path,
                        err.emit_to_string(&shader_source)
                    );
                }
            }
        }
    }

    assert!(
        tested_count > 0,
        "No WGSL shaders were found in {:?}",
        shader_dir
    );
}
