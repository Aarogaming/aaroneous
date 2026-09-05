use anyhow::{bail, Result};
use std::process::Command;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// SOVEREIGN-05: Dynamic API Exporter (Live-reloading C-ABI)
/// Allows the AI engine to write raw Rust code, compile it into a dynamic .dll / .so,
/// and hot-swap it directly into the running hypervisor without restarting.
pub struct PluginCompiler {
    pub staging_dir: PathBuf,
}

impl PluginCompiler {
    pub fn new() -> Self {
        let staging_dir = std::env::temp_dir().join("aaroneous_plugins");
        let _ = std::fs::create_dir_all(&staging_dir);
        Self { staging_dir }
    }

    /// Compiles a raw string of Rust code into a C-ABI DLL
    pub fn compile_dynamic_plugin(&self, plugin_name: &str, rust_code: &str) -> Result<PathBuf> {
        let plugin_dir = self.staging_dir.join(plugin_name);
        let src_dir = plugin_dir.join("src");
        let _ = std::fs::create_dir_all(&src_dir);

        // 1. Write Cargo.toml
        let cargo_toml = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
aaroneous_api = {{ path = "d:/Aaroneous/crates/aaroneous_api" }}
eframe = "0.34"
"#,
            plugin_name
        );
        std::fs::write(plugin_dir.join("Cargo.toml"), cargo_toml)?;

        // 2. Write src/lib.rs
        std::fs::write(src_dir.join("lib.rs"), rust_code)?;

        info!("Compiling sovereign plugin '{}' at {:?}...", plugin_name, plugin_dir);

        // 3. Execute Cargo Build
        let output = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&plugin_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Plugin compilation failed:\n{}", stderr);
        }

        // 4. Return path to the compiled DLL
        let dll_path = plugin_dir.join("target").join("release").join(format!("{}.dll", plugin_name));
        if dll_path.exists() {
            info!("Successfully compiled sovereign plugin: {:?}", dll_path);
            Ok(dll_path)
        } else {
            bail!("DLL not found after successful compilation.")
        }
    }
}