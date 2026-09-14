use anyhow::{bail, Result};
use std::path::PathBuf;
use std::process::Command;
use tracing::info;

/// SOVEREIGN-05: Dynamic API Exporter (Live-reloading C-ABI)
/// Allows the AI engine to write raw Rust code, compile it into a dynamic .dll / .so,
/// and hot-swap it directly into the running hypervisor without restarting.
pub struct PluginCompiler {
    pub staging_dir: PathBuf,
    pub workspace_root: PathBuf,
}

impl PluginCompiler {
    /// Construct with explicit staging and workspace root paths (zero ambient authority)
    pub fn new(staging_dir: PathBuf, workspace_root: PathBuf) -> Self {
        Self {
            staging_dir,
            workspace_root,
        }
    }

    /// Construct from injected workspace paths configuration
    pub fn from_workspace_paths(paths: &paths::WorkspacePaths) -> Self {
        Self {
            staging_dir: paths.cache().join("plugin_staging"),
            workspace_root: paths.root().to_path_buf(),
        }
    }

    /// Compiles a raw string of Rust code into a C-ABI DLL
    pub fn compile_dynamic_plugin(&self, plugin_name: &str, rust_code: &str) -> Result<PathBuf> {
        let plugin_dir = self.staging_dir.join(plugin_name);
        let src_dir = plugin_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;

        let api_path = self.workspace_root.join("crates").join("api");
        let api_path_str = api_path.to_string_lossy().replace('\\', "/");

        // 1. Write Cargo.toml
        let cargo_toml = format!(
            r#"
[package]
name = "{plugin_name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
api = {{ path = "{api_path_str}" }}
eframe = "0.34"
"#
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

        // 4. Return path to the compiled dynamic library
        #[cfg(target_os = "windows")]
        let lib_filename = format!("{}.dll", plugin_name);
        #[cfg(target_os = "linux")]
        let lib_filename = format!("lib{}.so", plugin_name);
        #[cfg(target_os = "macos")]
        let lib_filename = format!("lib{}.dylib", plugin_name);
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let lib_filename = format!("{}.dll", plugin_name);

        let dll_path = plugin_dir.join("target").join("release").join(lib_filename);
        if dll_path.exists() {
            info!("Successfully compiled sovereign plugin: {:?}", dll_path);
            Ok(dll_path)
        } else {
            bail!("Compiled library not found at {:?}", dll_path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_compiler_constructor_injection() {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("staging");
        let root = temp.path().join("root");
        let compiler = PluginCompiler::new(staging.clone(), root.clone());
        assert_eq!(compiler.staging_dir, staging);
        assert_eq!(compiler.workspace_root, root);
    }
}