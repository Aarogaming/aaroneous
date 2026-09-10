// Simplified build script: copy JSON files as raw bytes into a binary blob
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // CARGO_MANIFEST_DIR points to core/hypervisor
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    // repository root is two levels up
    let repo_root = manifest_dir.parent().and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Unable to find repo root"))?;
    let config_dir = repo_root.join("config");
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let mut out_file = File::create(out_dir.join("config.bin"))?;

    for entry in fs::read_dir(&config_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let data = fs::read(&path)?;
        out_file.write_all(&data)?;
    }
    Ok(())
}
