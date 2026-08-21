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
        std::env::temp_dir().join("primary.synapse")
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
        self.root.join("specialists")
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

    // ── Local LLM & GGUF Model Hub Auto-Discovery ─────────────────────

    /// Returns standard default paths for popular local LLM managers (LM Studio, Ollama, HuggingFace, Jan, etc.)
    pub fn get_known_model_hubs(&self) -> Vec<ModelHubLocation> {
        let mut hubs = Vec::new();

        // 1. Aaroneous Workspace models directory
        let ws_models = self.models();
        hubs.push(ModelHubLocation {
            name: "Aaroneous Models Hub".to_string(),
            exists: ws_models.exists(),
            path: ws_models,
        });

        // 2. LM Studio Default Cache Paths
        if let Some(home) = dirs::home_dir() {
            let lm_studio_cache = home.join(".cache").join("lm-studio").join("models");
            hubs.push(ModelHubLocation {
                name: "LM Studio Cache (~/.cache/lm-studio)".to_string(),
                exists: lm_studio_cache.exists(),
                path: lm_studio_cache,
            });

            let lm_studio_dir = home.join(".lmstudio").join("models");
            hubs.push(ModelHubLocation {
                name: "LM Studio Default (~/.lmstudio/models)".to_string(),
                exists: lm_studio_dir.exists(),
                path: lm_studio_dir,
            });

            // 3. Ollama Default Model Cache
            let ollama_home = home.join(".ollama").join("models");
            hubs.push(ModelHubLocation {
                name: "Ollama Models (~/.ollama/models)".to_string(),
                exists: ollama_home.exists(),
                path: ollama_home,
            });

            // 4. HuggingFace Hub Cache
            let hf_hub = home.join(".cache").join("huggingface").join("hub");
            hubs.push(ModelHubLocation {
                name: "HuggingFace Hub Cache".to_string(),
                exists: hf_hub.exists(),
                path: hf_hub,
            });
        }

        // 5. LocalAppData hubs (Windows / Linux)
        if let Some(local_data) = dirs::data_local_dir() {
            let ollama_local = local_data.join("Ollama").join("models");
            hubs.push(ModelHubLocation {
                name: "Ollama LocalAppData".to_string(),
                exists: ollama_local.exists(),
                path: ollama_local,
            });

            let jan_models = local_data.join("jan").join("models");
            hubs.push(ModelHubLocation {
                name: "Jan.ai Models".to_string(),
                exists: jan_models.exists(),
                path: jan_models,
            });

            let gpt4all = local_data.join("nomic.ai").join("GPT4All");
            hubs.push(ModelHubLocation {
                name: "GPT4All Models".to_string(),
                exists: gpt4all.exists(),
                path: gpt4all,
            });
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_paths_discovery_no_hardcoding() {
        let paths = WorkspacePaths::discover();
        assert!(!paths.root().as_os_str().is_empty());
        assert_eq!(paths.crates(), paths.root().join("crates"));
        assert_eq!(paths.models(), paths.root().join("models"));
    }

    #[test]
    fn test_known_model_hubs_exist() {
        let paths = WorkspacePaths::discover();
        let hubs = paths.get_known_model_hubs();
        assert!(!hubs.is_empty());
    }
}
