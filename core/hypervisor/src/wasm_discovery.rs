use crate::unified_registry::{EntryMeta, Registry};
use anyhow::{Result, bail};
/// WASM Plugin Discovery — scans directories for .wasm files and auto-registers them.
///
/// Discovers WASM enzymes in `chromosomes/`, `extensions/wasm/`, and configured paths,
/// validates them, and registers them in the unified registry.
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Metadata for a discovered WASM plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmPlugin {
    /// File path to the .wasm binary
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Whether it's a core WASM module or Component Model component
    pub is_component: bool,
    /// SHA-256 hash of the file
    pub hash: String,
    /// Source directory where it was discovered
    pub source_dir: PathBuf,
}

/// Discover WASM plugins in standard directories.
pub fn discover_wasm_plugins(workspace_root: &Path) -> Result<Vec<WasmPlugin>> {
    let mut plugins = Vec::new();

    let search_dirs = vec![
        workspace_root.join("chromosomes"),
        workspace_root.join("extensions").join("wasm"),
    ];

    for dir in &search_dirs {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "wasm") {
                match load_plugin_metadata(&path, dir) {
                    Ok(plugin) => {
                        info!(
                            "Discovered WASM plugin: {} ({} bytes)",
                            path.display(),
                            plugin.size
                        );
                        plugins.push(plugin);
                    }
                    Err(e) => {
                        warn!("Failed to load plugin {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    info!("Discovered {} WASM plugins", plugins.len());
    Ok(plugins)
}

/// Load metadata for a single WASM file.
fn load_plugin_metadata(path: &Path, source_dir: &Path) -> Result<WasmPlugin> {
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();

    // Check WASM magic bytes
    let mut magic = [0u8; 4];
    let mut file = std::fs::File::open(path)?;
    std::io::Read::read_exact(&mut file, &mut magic)?;
    if &magic != b"\0asm" {
        bail!("Not a valid WASM file (bad magic bytes)");
    }

    // Read version
    let mut version_bytes = [0u8; 4];
    std::io::Read::read_exact(&mut file, &mut version_bytes)?;
    let version = u32::from_le_bytes(version_bytes);

    // Check if it's a Component Model binary (version 1 + layer byte 0x0d)
    let is_component = version == 1 && {
        let mut layer = [0u8; 1];
        if std::io::Read::read(&mut file, &mut layer)? == 1 {
            layer[0] == 0x0d
        } else {
            false
        }
    };

    // Compute hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hex::encode(hasher.finalize());

    Ok(WasmPlugin {
        path: path.to_path_buf(),
        size,
        is_component,
        hash,
        source_dir: source_dir.to_path_buf(),
    })
}

/// Register discovered WASM plugins into a registry.
pub fn register_discovered_plugins(
    registry: &mut Registry<WasmPlugin>,
    workspace_root: &Path,
) -> Result<usize> {
    let plugins = discover_wasm_plugins(workspace_root)?;
    let mut count = 0;

    for plugin in plugins {
        let id = plugin
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let meta = EntryMeta::new("1.0.0").with_tags(vec![
            if plugin.is_component {
                "component".into()
            } else {
                "core-module".into()
            },
            plugin
                .source_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .into(),
        ]);

        if let Err(e) = registry.register(id, plugin, meta) {
            warn!("Failed to register plugin: {}", e);
        } else {
            count += 1;
        }
    }

    info!("Registered {} WASM plugins into registry", count);
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_discover_empty_dir() {
        let dir = std::env::temp_dir().join("test_wasm_discover");
        std::fs::create_dir_all(&dir).ok();

        let plugins = discover_wasm_plugins(&dir).unwrap();
        assert_eq!(plugins.len(), 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_discover_invalid_file() {
        let dir = std::env::temp_dir().join("test_wasm_discover2");
        std::fs::create_dir_all(&dir).ok();

        // Create a non-WASM file
        let path = dir.join("test.wasm");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"not wasm").unwrap();

        let plugins = discover_wasm_plugins(&dir).unwrap();
        assert_eq!(plugins.len(), 0); // Should skip invalid files

        std::fs::remove_dir_all(&dir).ok();
    }
}
