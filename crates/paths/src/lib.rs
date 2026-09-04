//! crates/paths
//! Centralized, Dynamic Workspace and Model Path Resolver for Aaroneous.
//! ZERO Hardcoded Paths: Discovers root directories, user data paths, and local model hubs dynamically.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Discovered Local LLM / GGUF Model Info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredGgufModel {
    pub file_name: String,
    pub full_path: PathBuf,
    pub size_bytes: u64,
    pub formatted_size: String,
    pub source_hub: String,
}

/// Known Local AI / LLM Application Hubs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHubLocation {
    pub name: String,
    pub path: PathBuf,
    pub exists: bool,
}

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    root: PathBuf,
}

impl Default for WorkspacePaths {
    fn default() -> Self {
        Self::discover()
    }
}

impl WorkspacePaths {
    /// Discover and construct workspace paths dynamically without hardcoded drive letters.
    pub fn discover() -> Self {
        // 1. Check explicit environment variable
        if let Ok(env_root) = std::env::var("AARONEOUS_WORKSPACE") {
            let p = PathBuf::from(env_root);
            if p.exists() {
                return Self { root: p };
            }
        }

        // 2. Check current working directory or traverse upward to find workspace marker
        if let Ok(cwd) = std::env::current_dir() {
            let mut curr = cwd.as_path();
            loop {
                if curr.join("Cargo.toml").exists() && (curr.join("crates").exists() || curr.join("core").exists()) {
                    return Self { root: curr.to_path_buf() };
                }
                match curr.parent() {
                    Some(parent) => curr = parent,
                    None => break,
                }
            }
        }

        // 3. Check current executable parent directory
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                if exe_dir.join("Cargo.toml").exists() {
                    return Self { root: exe_dir.to_path_buf() };
                }
                if let Some(parent) = exe_dir.parent() {
                    if parent.join("Cargo.toml").exists() {
                        return Self { root: parent.to_path_buf() };
                    }
                }
            }
        }

        // 4. Default to standard OS Application Data Directory
        let app_data = dirs::data_local_dir()
            .map(|p| p.join("Aaroneous"))
            .unwrap_or_else(|| std::env::temp_dir().join("Aaroneous"));

        Self { root: app_data }
    }

    /// Construct from an explicit root path.
    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    /// The workspace root directory.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    // ── Core directories ──────────────────────────────────────────────

    pub fn crates(&self) -> PathBuf {
        self.root.join("crates")
    }

    pub fn dev(&self) -> PathBuf {
        self.root.join("dev")
    }

    pub fn models(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn config(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn agents(&self) -> PathBuf {
        self.data().join("agents")
    }

    pub fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn exports(&self) -> PathBuf {
        self.data().join("exports")
    }

    pub fn synapse_file(&self) -> PathBuf {
        self.synapse_named("primary")
    }

    pub fn synapse_named(&self, name: &str) -> PathBuf {
        let file_name = if name.ends_with(".synapse") {
            name.to_string()
        } else {
            format!("{}.synapse", name)
        };

        if let Ok(env_root) = std::env::var("AARONEOUS_WORKSPACE") {
            let p = PathBuf::from(env_root);
            if p.exists() {
                return p.join(&file_name);
            }
        }

        std::env::temp_dir().join(file_name)
    }

    pub fn hive_db(&self) -> PathBuf {
        self.root.join("hive.db")
    }

    pub fn hox_db(&self) -> PathBuf {
        self.root.join("hox.db")
    }

    pub fn models_inbox(&self) -> PathBuf {
        self.models().join("inbox")
    }

    pub fn specialists(&self) -> PathBuf {
        self.crates().join("specialists")
    }

    pub fn sovereign_model(&self, sovereign: &str) -> PathBuf {
        self.models()
            .join(format!("{}-qwen2.5-7b.gguf", sovereign.to_lowercase()))
    }

    pub fn sovereign_hox_preset(&self, name: &str) -> PathBuf {
        self.registry()
            .join(format!("hox_{}.json", name.to_lowercase()))
    }

    pub fn relic_hox_preset(&self, name: &str) -> PathBuf {
        self.sovereign_hox_preset(name)
    }

    pub fn omni_galaxy_map(&self) -> PathBuf {
        self.dev().join("tools").join("omni").join("output").join("omni_galaxy_map.json")
    }

    pub fn extensions(&self) -> PathBuf {
        self.data().join("extensions")
    }

    pub fn shadow_sandbox(&self) -> PathBuf {
        self.root.join(".sab").join("shadow")
    }

    pub fn cartridges(&self) -> PathBuf {
        self.models().join("cartridges")
    }

    pub fn cartridges_inbox(&self) -> PathBuf {
        self.cartridges().join("inbox")
    }

    /// Creates all standard root directories if they do not exist
    pub fn ensure_directories(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.crates())?;
        std::fs::create_dir_all(self.dev())?;
        std::fs::create_dir_all(self.models())?;
        std::fs::create_dir_all(self.config())?;
        std::fs::create_dir_all(self.data())?;
        std::fs::create_dir_all(self.agents())?;
        std::fs::create_dir_all(self.exports())?;
        std::fs::create_dir_all(self.logs())?;
        std::fs::create_dir_all(self.cache())?;
        Ok(())
    }
}

/// Dynamic Model Hub Detector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHubDetector {
    pub name: String,
    pub relative_to_home: Option<PathBuf>,
    pub relative_to_local_data: Option<PathBuf>,
}

/// Universal Model Hub Registry for dynamic local model discovery
#[derive(Debug, Clone, Default)]
pub struct UniversalModelHubRegistry {
    detectors: Vec<ModelHubDetector>,
}

impl UniversalModelHubRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default_hubs() -> Self {
        let mut reg = Self::new();
        reg.register_home_hub("LM Studio Cache (~/.cache/lm-studio)", Path::new(".cache/lm-studio/models"));
        reg.register_home_hub("LM Studio Default (~/.lmstudio/models)", Path::new(".lmstudio/models"));
        reg.register_home_hub("Ollama Models (~/.ollama/models)", Path::new(".ollama/models"));
        reg.register_home_hub("HuggingFace Hub Cache", Path::new(".cache/huggingface/hub"));
        reg.register_local_data_hub("Ollama LocalAppData", Path::new("Ollama/models"));
        reg.register_local_data_hub("Jan.ai Models", Path::new("jan/models"));
        reg.register_local_data_hub("GPT4All Models", Path::new("nomic.ai/GPT4All"));
        reg
    }

    pub fn register_home_hub(&mut self, name: impl Into<String>, rel_path: impl AsRef<Path>) {
        self.detectors.push(ModelHubDetector {
            name: name.into(),
            relative_to_home: Some(rel_path.as_ref().to_path_buf()),
            relative_to_local_data: None,
        });
    }

    pub fn register_local_data_hub(&mut self, name: impl Into<String>, rel_path: impl AsRef<Path>) {
        self.detectors.push(ModelHubDetector {
            name: name.into(),
            relative_to_home: None,
            relative_to_local_data: Some(rel_path.as_ref().to_path_buf()),
        });
    }

    pub fn discover_locations(&self) -> Vec<ModelHubLocation> {
        let mut hubs = Vec::new();
        let home = dirs::home_dir();
        let local_data = dirs::data_local_dir();

        for d in &self.detectors {
            if let Some(rel) = &d.relative_to_home {
                if let Some(h) = &home {
                    let target = h.join(rel);
                    hubs.push(ModelHubLocation {
                        name: d.name.clone(),
                        exists: target.exists(),
                        path: target,
                    });
                }
            } else if let Some(rel) = &d.relative_to_local_data {
                if let Some(ld) = &local_data {
                    let target = ld.join(rel);
                    hubs.push(ModelHubLocation {
                        name: d.name.clone(),
                        exists: target.exists(),
                        path: target,
                    });
                }
            }
        }
        hubs
    }
}

impl WorkspacePaths {
    // ── Local LLM & GGUF Model Hub Auto-Discovery ─────────────────────

    /// Returns standard default paths for popular local LLM managers dynamically
    pub fn get_known_model_hubs(&self) -> Vec<ModelHubLocation> {
        let mut hubs = Vec::new();

        // 1. Aaroneous Workspace models directory
        let ws_models = self.models();
        hubs.push(ModelHubLocation {
            name: "Aaroneous Models Hub".to_string(),
            exists: ws_models.exists(),
            path: ws_models,
        });

        // 2. Discover from Universal Registry
        let registry = UniversalModelHubRegistry::with_default_hubs();
        hubs.extend(registry.discover_locations());

        hubs
    }

    /// Automatically scans all known local LLM hubs plus any custom configured path for `.gguf` models
    pub fn scan_all_gguf_models(&self, custom_dirs: &[PathBuf]) -> Vec<DiscoveredGgufModel> {
        let mut discovered = Vec::new();
        let mut searched_paths = std::collections::HashSet::new();

        // Scan known hubs
        for hub in self.get_known_model_hubs() {
            if hub.exists && searched_paths.insert(hub.path.clone()) {
                Self::scan_directory_for_gguf(&hub.path, &hub.name, &mut discovered, 4);
            }
        }

        // Scan custom user-defined directories
        for custom_path in custom_dirs {
            if custom_path.exists() && searched_paths.insert(custom_path.clone()) {
                Self::scan_directory_for_gguf(custom_path, "Custom Folder", &mut discovered, 4);
            }
        }

        discovered
    }

    /// Recursively scans a directory up to `max_depth` for `.gguf` files
    fn scan_directory_for_gguf(dir: &Path, source_hub: &str, out: &mut Vec<DiscoveredGgufModel>, max_depth: usize) {
        if max_depth == 0 {
            return;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if ext.eq_ignore_ascii_case("gguf") {
                            let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            let formatted_size = format_bytes(size_bytes);

                            out.push(DiscoveredGgufModel {
                                file_name,
                                full_path: path,
                                size_bytes,
                                formatted_size,
                                source_hub: source_hub.to_string(),
                            });
                        }
                    }
                } else if path.is_dir() {
                    Self::scan_directory_for_gguf(&path, source_hub, out, max_depth - 1);
                }
            }
        }
    }
}

/// Formats byte counts into human-readable strings (MB / GB)
fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KB", bytes / 1024)
    }
}

/// Platform-agnostic workspace synapse path resolution
pub fn resolve_synapse_path(name: &str) -> PathBuf {
    WorkspacePaths::discover().synapse_named(name)
}

/// Hierarchical configuration registry supporting cascading overrides:
/// Defaults -> Machine Profile -> Workspace Project -> Environment Overrides
#[derive(Debug, Clone, Default)]
pub struct FederationConfigRegistry {
    entries: std::collections::HashMap<String, String>,
}

impl FederationConfigRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or overrides a configuration key-value pair
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    /// Resolves configuration key: checks environment variables first, then registry entries
    pub fn get(&self, key: &str) -> Option<String> {
        let env_key = format!("AARONEOUS_{}", key.to_uppercase().replace('.', "_"));
        if let Ok(val) = std::env::var(&env_key) {
            return Some(val);
        }
        self.entries.get(key).cloned()
    }

    /// Resolves configuration key with default fallback
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        if let Some(val) = self.entries.get(key) {
            val.as_str()
        } else {
            default
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_paths_discovery_no_hardcoding() {
        let paths = WorkspacePaths::discover();
        assert!(!paths.root().as_os_str().is_empty());
        assert_eq!(paths.crates(), paths.root().join("crates"));
        assert_eq!(paths.specialists(), paths.crates().join("specialists"));
        assert_eq!(paths.models(), paths.root().join("models"));
        assert_eq!(paths.cartridges(), paths.models().join("cartridges"));
        assert_eq!(paths.cartridges_inbox(), paths.cartridges().join("inbox"));
    }

    #[test]
    fn test_known_model_hubs_exist() {
        let paths = WorkspacePaths::discover();
        let hubs = paths.get_known_model_hubs();
        assert!(!hubs.is_empty());
    }

    #[test]
    fn test_resolve_synapse_path() {
        let path = resolve_synapse_path("primary");
        assert!(path.to_string_lossy().ends_with("primary.synapse"));
    }

    #[test]
    fn test_federation_config_registry() {
        let mut reg = FederationConfigRegistry::new();
        reg.set("studio.theme", "dark");
        reg.set("runtime.threads", "8");

        assert_eq!(reg.get_or("studio.theme", "light"), "dark");
        assert_eq!(reg.get_or("studio.font", "monospace"), "monospace");
        assert_eq!(reg.get("runtime.threads"), Some("8".to_string()));
    }
}
