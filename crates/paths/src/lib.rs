//! crates/paths
//! Centralized, Dynamic Workspace and Model Path Resolver for Aaroneous.
//! ZERO Hardcoded Paths: Discovers root directories, user data paths, and local model hubs dynamically.

#![deny(unsafe_code)]
#![allow(unknown_lints)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Named missing-input and validation errors for strict workspace path configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StrictPathsError {
    #[error("Missing required workspace root path")]
    MissingRoot,
    #[error("Specified workspace root path is not absolute: {0}")]
    RootNotAbsolute(PathBuf),
    #[error("Specified workspace root path does not exist: {0}")]
    RootNotFound(PathBuf),
    #[error("Specified workspace root path is not a directory: {0}")]
    RootNotDirectory(PathBuf),
    #[error("Missing required temporary directory path")]
    MissingTempDir,
    #[error("Specified temporary directory path is not absolute: {0}")]
    TempDirNotAbsolute(PathBuf),
    #[error("Specified temporary directory does not exist: {0}")]
    TempDirNotFound(PathBuf),
    #[error("Specified temporary directory is not a directory: {0}")]
    TempDirNotDirectory(PathBuf),
    #[error("Missing required external data root path")]
    MissingDataRoot,
    #[error("Specified external data root path is not absolute: {0}")]
    DataRootNotAbsolute(PathBuf),
    #[error("Specified external data root path does not exist: {0}")]
    DataRootNotFound(PathBuf),
    #[error("Specified external data root path is not a directory: {0}")]
    DataRootNotDirectory(PathBuf),
}

/// Fully explicit, zero-fallback workspace paths configuration for external controllers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictWorkspacePathsConfig {
    pub root: PathBuf,
    pub temp_dir: PathBuf,
    pub data_root: PathBuf,
}

impl StrictWorkspacePathsConfig {
    pub fn new(root: PathBuf, temp_dir: PathBuf, data_root: PathBuf) -> Self {
        Self {
            root,
            temp_dir,
            data_root,
        }
    }

    /// Validates all paths strictly: root, temp_dir, and data_root must be absolute paths,
    /// exist, and be directories.
    pub fn validate(&self) -> Result<(), StrictPathsError> {
        if self.root.as_os_str().is_empty() {
            return Err(StrictPathsError::MissingRoot);
        }
        if !self.root.is_absolute() {
            return Err(StrictPathsError::RootNotAbsolute(self.root.clone()));
        }
        if !self.root.exists() {
            return Err(StrictPathsError::RootNotFound(self.root.clone()));
        }
        if !self.root.is_dir() {
            return Err(StrictPathsError::RootNotDirectory(self.root.clone()));
        }
        if self.temp_dir.as_os_str().is_empty() {
            return Err(StrictPathsError::MissingTempDir);
        }
        if !self.temp_dir.is_absolute() {
            return Err(StrictPathsError::TempDirNotAbsolute(self.temp_dir.clone()));
        }
        if !self.temp_dir.exists() {
            return Err(StrictPathsError::TempDirNotFound(self.temp_dir.clone()));
        }
        if !self.temp_dir.is_dir() {
            return Err(StrictPathsError::TempDirNotDirectory(self.temp_dir.clone()));
        }
        if self.data_root.as_os_str().is_empty() {
            return Err(StrictPathsError::MissingDataRoot);
        }
        if !self.data_root.is_absolute() {
            return Err(StrictPathsError::DataRootNotAbsolute(
                self.data_root.clone(),
            ));
        }
        if !self.data_root.exists() {
            return Err(StrictPathsError::DataRootNotFound(self.data_root.clone()));
        }
        if !self.data_root.is_dir() {
            return Err(StrictPathsError::DataRootNotDirectory(
                self.data_root.clone(),
            ));
        }
        Ok(())
    }
}

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

/// Config POD for workspace discovery - replaces env var injection
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct WorkspacePathsConfig {
    pub explicit_root: Option<PathBuf>,
    pub temp_dir: Option<PathBuf>,
    pub external_data_root: Option<PathBuf>,
}

impl WorkspacePathsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_explicit_root(mut self, root: PathBuf) -> Self {
        self.explicit_root = Some(root);
        self
    }

    pub fn with_temp_dir(mut self, temp: PathBuf) -> Self {
        self.temp_dir = Some(temp);
        self
    }

    pub fn with_external_data_root(mut self, data_root: PathBuf) -> Self {
        self.external_data_root = Some(data_root);
        self
    }

    /// Validates this configuration against the strict external-controller contract.
    ///
    /// Unlike `discover()`, this returns a named `StrictPathsError` if `explicit_root`,
    /// `temp_dir`, or `external_data_root` is missing, non-existent, or not a directory.
    /// It NEVER invokes ambient system discovery, executable traversal, or sibling fallbacks.
    pub fn validate_strict(&self) -> Result<StrictWorkspacePathsConfig, StrictPathsError> {
        let root = self
            .explicit_root
            .clone()
            .ok_or(StrictPathsError::MissingRoot)?;
        if root.as_os_str().is_empty() {
            return Err(StrictPathsError::MissingRoot);
        }
        if !root.is_absolute() {
            return Err(StrictPathsError::RootNotAbsolute(root));
        }
        if !root.exists() {
            return Err(StrictPathsError::RootNotFound(root));
        }
        if !root.is_dir() {
            return Err(StrictPathsError::RootNotDirectory(root));
        }

        let temp_dir = self
            .temp_dir
            .clone()
            .ok_or(StrictPathsError::MissingTempDir)?;
        if temp_dir.as_os_str().is_empty() {
            return Err(StrictPathsError::MissingTempDir);
        }
        if !temp_dir.is_absolute() {
            return Err(StrictPathsError::TempDirNotAbsolute(temp_dir));
        }
        if !temp_dir.exists() {
            return Err(StrictPathsError::TempDirNotFound(temp_dir));
        }
        if !temp_dir.is_dir() {
            return Err(StrictPathsError::TempDirNotDirectory(temp_dir));
        }

        let data_root = self
            .external_data_root
            .clone()
            .ok_or(StrictPathsError::MissingDataRoot)?;
        if data_root.as_os_str().is_empty() {
            return Err(StrictPathsError::MissingDataRoot);
        }
        if !data_root.is_absolute() {
            return Err(StrictPathsError::DataRootNotAbsolute(data_root));
        }
        if !data_root.exists() {
            return Err(StrictPathsError::DataRootNotFound(data_root));
        }
        if !data_root.is_dir() {
            return Err(StrictPathsError::DataRootNotDirectory(data_root));
        }

        Ok(StrictWorkspacePathsConfig {
            root,
            temp_dir,
            data_root,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    root: PathBuf,
    temp_dir: PathBuf,
    data_root: PathBuf,
}

impl Default for WorkspacePaths {
    fn default() -> Self {
        Self::bootstrap_adapter(&WorkspacePathsConfig::new())
    }
}

impl WorkspacePaths {
    /// Construct strictly validated workspace paths for external controllers.
    ///
    /// Requires explicit root, temp_dir, and data_root.
    /// Performs ZERO ambient filesystem traversals, ZERO home directory probes,
    /// and ZERO fallback defaults. If any path is missing, does not exist,
    /// or is not a directory, a typed `StrictPathsError` is returned.
    pub fn from_strict_config(
        config: StrictWorkspacePathsConfig,
    ) -> Result<Self, StrictPathsError> {
        config.validate()?;
        Ok(Self {
            root: config.root,
            temp_dir: config.temp_dir,
            data_root: config.data_root,
        })
    }

    /// Construct strictly validated workspace paths from `WorkspacePathsConfig`.
    ///
    /// Returns `StrictPathsError` if any required path is missing, non-existent,
    /// or not a directory.
    pub fn strict(config: &WorkspacePathsConfig) -> Result<Self, StrictPathsError> {
        let strict_config = config.validate_strict()?;
        Self::from_strict_config(strict_config)
    }

    /// Legacy/bootstrap compatibility adapter for developer and interactive workflows.
    ///
    /// This adapter performs ambient discovery, executable traversal, and sibling folder
    /// guessing if explicit paths are not supplied. External controllers MUST NOT use
    /// this route; use [`Self::from_strict_config`] or [`Self::strict`] instead.
    pub fn bootstrap_adapter(config: &WorkspacePathsConfig) -> Self {
        // 1. Use explicit root from config (replaces env var lookup)
        let root = if let Some(explicit_root) = &config.explicit_root {
            if explicit_root.exists() {
                explicit_root.clone()
            } else {
                Self::discover_root_from_system()
            }
        } else {
            Self::discover_root_from_system()
        };

        let temp_dir = config.temp_dir.clone().unwrap_or_else(|| root.join(".tmp"));

        let data_root = Self::discover_external_data_root(config, &root);

        Self {
            root,
            temp_dir,
            data_root,
        }
    }

    /// Compatibility wrapper delegating to [`Self::bootstrap_adapter`].
    pub fn discover(config: &WorkspacePathsConfig) -> Self {
        Self::bootstrap_adapter(config)
    }

    fn discover_external_data_root(config: &WorkspacePathsConfig, root: &Path) -> PathBuf {
        // 1. Explicitly configured external data root via constructor injection
        if let Some(ref external) = config.external_data_root {
            return external.clone();
        }

        // 2. Default to a sibling directory outside the repository.
        if let Some(parent) = root.parent() {
            let sibling = parent.join("ArcData");
            return sibling;
        }

        // 3. Safe fallback if root has no parent
        root.join("data")
    }

    fn discover_root_from_system() -> PathBuf {
        let is_repo_root = |dir: &Path| {
            dir.join("Cargo.toml").exists()
                && (dir.join("crates").exists() || dir.join("core").exists())
        };

        // 1. Check current executable parent directory and traverse upward
        if let Ok(exe) = std::env::current_exe() {
            let mut curr = exe.parent();
            while let Some(dir) = curr {
                if is_repo_root(dir) {
                    return dir.to_path_buf();
                }
                curr = dir.parent();
            }
        }

        // 2. Default to standard OS Application Data Directory
        dirs::data_local_dir()
            .map(|p| p.join("Aaroneous"))
            .unwrap_or_else(|| PathBuf::from("."))
    }

    /// Construct from an explicit root path.
    pub fn from_root(root: PathBuf) -> Self {
        let temp_dir = root.join(".tmp");
        let data_root = Self::discover_external_data_root(&WorkspacePathsConfig::new(), &root);
        Self {
            root,
            temp_dir,
            data_root,
        }
    }

    /// Construct from config POD (preferred pattern).
    pub fn from_config(config: WorkspacePathsConfig) -> Self {
        Self::discover(&config)
    }

    /// The workspace root directory.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// The workspace temp directory.
    pub fn temp_dir(&self) -> &PathBuf {
        &self.temp_dir
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

    /// The external data root directory.
    pub fn data_root(&self) -> &PathBuf {
        &self.data_root
    }

    /// Alias for external data root.
    pub fn external_data_root(&self) -> &PathBuf {
        &self.data_root
    }

    pub fn data(&self) -> PathBuf {
        self.data_root.clone()
    }

    pub fn si_models(&self) -> PathBuf {
        self.data().join("models")
    }

    pub fn state_banks(&self) -> PathBuf {
        self.data().join("state_banks")
    }

    pub fn skill_storage(&self) -> PathBuf {
        self.data().join("skills")
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

        self.temp_dir.join(file_name)
    }

    pub fn hive_db(&self) -> PathBuf {
        self.root.join("hive.db")
    }

    pub fn node_db(&self) -> PathBuf {
        self.root.join("node.db")
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

    pub fn agent_preset(&self, name: &str) -> PathBuf {
        let agent_file = self
            .registry()
            .join(format!("agent_{}.json", name.to_lowercase()));
        if agent_file.exists() {
            agent_file
        } else {
            self.registry()
                .join(format!("hox_{}.json", name.to_lowercase()))
        }
    }

    pub fn node_preset(&self, name: &str) -> PathBuf {
        self.agent_preset(name)
    }

    pub fn module_preset(&self, name: &str) -> PathBuf {
        let module_store = self
            .registry()
            .join(format!("module_store_{}.json", name.to_lowercase()));
        if module_store.exists() {
            module_store
        } else {
            self.node_preset(name)
        }
    }

    pub fn omni_galaxy_map(&self) -> PathBuf {
        self.dev()
            .join("tools")
            .join("omni")
            .join("output")
            .join("omni_galaxy_map.json")
    }

    pub fn extensions(&self) -> PathBuf {
        let in_tree = self.root.join("data").join("extensions");
        if in_tree.exists() {
            in_tree
        } else {
            self.data().join("extensions")
        }
    }

    pub fn shadow_sandbox(&self) -> PathBuf {
        self.root.join(".sab").join("shadow")
    }

    pub fn cartridges(&self) -> PathBuf {
        self.si_models().join("cartridges")
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
        std::fs::create_dir_all(self.si_models())?;
        std::fs::create_dir_all(self.skill_storage())?;
        std::fs::create_dir_all(self.state_banks())?;
        std::fs::create_dir_all(self.cartridges())?;
        std::fs::create_dir_all(self.cartridges_inbox())?;
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
        reg.register_home_hub(
            "LM Studio Cache (~/.cache/lm-studio)",
            Path::new(".cache/lm-studio/models"),
        );
        reg.register_home_hub(
            "LM Studio Default (~/.lmstudio/models)",
            Path::new(".lmstudio/models"),
        );
        reg.register_home_hub(
            "Ollama Models (~/.ollama/models)",
            Path::new(".ollama/models"),
        );
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

    /// Discover model hub locations using explicitly injected home and local_data roots.
    /// Does not perform ambient directory queries if roots are not provided.
    pub fn discover_locations_with_roots(
        &self,
        home: Option<&Path>,
        local_data: Option<&Path>,
    ) -> Vec<ModelHubLocation> {
        let mut hubs = Vec::new();

        for d in &self.detectors {
            if let Some(rel) = &d.relative_to_home {
                if let Some(h) = home {
                    let target = h.join(rel);
                    hubs.push(ModelHubLocation {
                        name: d.name.clone(),
                        exists: target.exists(),
                        path: target,
                    });
                }
            } else if let Some(rel) = &d.relative_to_local_data
                && let Some(ld) = local_data
            {
                let target = ld.join(rel);
                hubs.push(ModelHubLocation {
                    name: d.name.clone(),
                    exists: target.exists(),
                    path: target,
                });
            }
        }
        hubs
    }

    /// Legacy/bootstrap location discovery querying ambient host user directories.
    pub fn discover_locations(&self) -> Vec<ModelHubLocation> {
        self.discover_locations_with_roots(
            dirs::home_dir().as_deref(),
            dirs::data_local_dir().as_deref(),
        )
    }
}

impl WorkspacePaths {
    /// Explicit scan of user-specified model directories.
    /// Does not query ambient model hub locations unless explicitly passed.
    pub fn scan_explicit_gguf_models(&self, search_dirs: &[PathBuf]) -> Vec<DiscoveredGgufModel> {
        let mut discovered = Vec::new();
        let mut searched_paths = std::collections::HashSet::new();

        for dir in search_dirs {
            if dir.exists() && searched_paths.insert(dir.clone()) {
                Self::scan_directory_for_gguf(dir, "Explicit Search Dir", &mut discovered, 4);
            }
        }
        discovered
    }
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
    fn scan_directory_for_gguf(
        dir: &Path,
        source_hub: &str,
        out: &mut Vec<DiscoveredGgufModel>,
        max_depth: usize,
    ) {
        if max_depth == 0 {
            return;
        }

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str())
                        && ext.eq_ignore_ascii_case("gguf")
                    {
                        let file_name = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
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

/// Pure path normalization that resolves '.' and '..' components deterministically
/// without accessing the filesystem or invoking ambient canonicalization.
pub fn normalize_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                match normalized.components().next_back() {
                    Some(std::path::Component::Normal(_)) => {
                        normalized.pop();
                    }
                    Some(std::path::Component::ParentDir) => {
                        normalized.push("..");
                    }
                    Some(std::path::Component::RootDir) | Some(std::path::Component::Prefix(_)) => {
                        // At root or prefix, '..' is a no-op (cannot escape above root)
                    }
                    None => {
                        normalized.push("..");
                    }
                    _ => {
                        normalized.pop();
                    }
                }
            }
            c => normalized.push(c.as_os_str()),
        }
    }
    normalized
}

/// Platform-agnostic workspace synapse path resolution
pub fn resolve_synapse_path(name: &str, config: &WorkspacePathsConfig) -> PathBuf {
    WorkspacePaths::discover(config).synapse_named(name)
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

    pub fn from_entries(entries: std::collections::HashMap<String, String>) -> Self {
        Self { entries }
    }

    /// Inserts or overrides a configuration key-value pair
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    /// Resolves configuration key from injected registry entries
    pub fn get(&self, key: &str) -> Option<String> {
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
        let paths = WorkspacePaths::discover(&WorkspacePathsConfig::new());
        assert!(!paths.root().as_os_str().is_empty());
        assert_eq!(paths.crates(), paths.root().join("crates"));
        assert_eq!(paths.specialists(), paths.crates().join("specialists"));
        assert_eq!(paths.models(), paths.root().join("models"));
        assert_eq!(paths.cartridges(), paths.si_models().join("cartridges"));
        assert_eq!(paths.cartridges_inbox(), paths.cartridges().join("inbox"));
        assert_eq!(paths.state_banks(), paths.data().join("state_banks"));
        assert_eq!(paths.skill_storage(), paths.data().join("skills"));
    }

    #[test]
    fn test_external_data_root_config_injection() {
        let custom_root = PathBuf::from("X:\\CustomDataRoot");
        let config = WorkspacePathsConfig::new().with_external_data_root(custom_root.clone());
        let paths = WorkspacePaths::discover(&config);
        assert_eq!(paths.data_root(), &custom_root);
        assert_eq!(paths.data(), custom_root);
        assert_eq!(paths.si_models(), custom_root.join("models"));
        assert_eq!(
            paths.cartridges(),
            custom_root.join("models").join("cartridges")
        );
        assert_eq!(paths.skill_storage(), custom_root.join("skills"));
        assert_eq!(paths.state_banks(), custom_root.join("state_banks"));
    }

    #[test]
    fn test_external_data_root_sibling_resolution() {
        let temp = tempfile::tempdir().unwrap();
        let repo_root = temp.path().join("Aaroneous");
        std::fs::create_dir(&repo_root).unwrap();
        let config = WorkspacePathsConfig::new().with_explicit_root(repo_root);
        let paths = WorkspacePaths::discover(&config);
        let data_root = temp.path().join("ArcData");
        assert_eq!(paths.data(), data_root);
        assert_eq!(paths.si_models(), data_root.join("models"));
        assert_eq!(
            paths.cartridges(),
            data_root.join("models").join("cartridges")
        );
    }

    #[test]
    fn test_known_model_hubs_exist() {
        let paths = WorkspacePaths::discover(&WorkspacePathsConfig::new());
        let hubs = paths.get_known_model_hubs();
        assert!(!hubs.is_empty());
    }

    #[test]
    fn test_resolve_synapse_path() {
        let config = WorkspacePathsConfig::new();
        let path = resolve_synapse_path("primary", &config);
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

    #[test]
    fn test_normalize_path_adversarial_matrix() {
        // 1. Excessive upward directory traversal escapes
        assert_eq!(
            normalize_path("../../../../etc/shadow"),
            PathBuf::from("../../../../etc/shadow")
        );
        #[cfg(windows)]
        assert_eq!(
            normalize_path("C:\\..\\..\\Windows\\System32"),
            PathBuf::from("C:\\Windows\\System32")
        );
        assert_eq!(
            normalize_path("/../../../../etc/passwd"),
            PathBuf::from("/etc/passwd")
        );

        // 2. Redundant separators. Path separators follow the host platform.
        #[cfg(windows)]
        assert_eq!(
            normalize_path("foo//bar\\\\baz/../qux"),
            PathBuf::from("foo/bar/qux")
        );
        #[cfg(not(windows))]
        assert_eq!(
            normalize_path("foo//bar/baz/../qux"),
            PathBuf::from("foo/bar/qux")
        );
        assert_eq!(normalize_path("a///b/c/../../d"), PathBuf::from("a/d"));

        // 3. Current-dir noise and trailing slash retention / normalization
        assert_eq!(normalize_path("./a/./b/../c/."), PathBuf::from("a/c"));
        assert_eq!(normalize_path("./././"), PathBuf::from(""));
        assert_eq!(normalize_path("a/b/c/"), PathBuf::from("a/b/c"));

        // 4. Windows drive letter prefixes and root preservation
        #[cfg(windows)]
        assert_eq!(normalize_path("C:foo/../bar"), PathBuf::from("C:bar"));
        #[cfg(not(windows))]
        assert_eq!(normalize_path("C:foo/../bar"), PathBuf::from("bar"));
        #[cfg(windows)]
        assert_eq!(normalize_path("C:\\foo\\..\\bar"), PathBuf::from("C:\\bar"));
        #[cfg(windows)]
        {
            assert_eq!(
                normalize_path(r"\\?\C:\foo\..\bar"),
                PathBuf::from(r"\\?\C:\bar")
            );
            assert_eq!(
                normalize_path(r"\\server\share\a\..\b"),
                PathBuf::from(r"\\server\share\b")
            );
        }

        // 5. Empty path strings and whitespace inputs
        assert_eq!(normalize_path(""), PathBuf::from(""));
        assert_eq!(normalize_path("   "), PathBuf::from("   "));
        assert_eq!(normalize_path("   /../foo"), PathBuf::from("foo"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_preset_resolution_and_fallback_matrix() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Aaroneous");
        let registry = root.join("registry");
        std::fs::create_dir_all(&registry).unwrap();

        let config = WorkspacePathsConfig::new().with_explicit_root(root);
        let paths = WorkspacePaths::discover(&config);

        // Case 1: Legacy hox_*.json exists, agent_*.json does not exist
        let legacy_hox = registry.join("hox_sentinel.json");
        std::fs::write(&legacy_hox, b"{}").unwrap();

        assert_eq!(paths.agent_preset("sentinel"), legacy_hox);
        assert_eq!(paths.node_preset("sentinel"), legacy_hox);
        assert_eq!(paths.module_preset("sentinel"), legacy_hox);

        // Case 2: Canonical agent_*.json exists -> takes precedence over hox_*.json
        let canonical_agent = registry.join("agent_sentinel.json");
        std::fs::write(&canonical_agent, b"{}").unwrap();

        assert_eq!(paths.agent_preset("sentinel"), canonical_agent);
        assert_eq!(paths.node_preset("sentinel"), canonical_agent);

        // Case 3: Canonical module_store_*.json exists -> takes precedence for module presets
        let module_store = registry.join("module_store_sentinel.json");
        std::fs::write(&module_store, b"{}").unwrap();

        assert_eq!(paths.module_preset("sentinel"), module_store);
    }

    #[test]
    fn test_strict_paths_missing_root() {
        let temp = tempfile::tempdir().unwrap();
        let temp_dir = temp.path().join("tmp");
        let data_dir = temp.path().join("data");
        std::fs::create_dir(&temp_dir).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let config = WorkspacePathsConfig::new()
            .with_temp_dir(temp_dir)
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::MissingRoot);

        let err2 = WorkspacePaths::strict(&config).unwrap_err();
        assert_eq!(err2, StrictPathsError::MissingRoot);
    }

    #[test]
    fn test_strict_paths_relative_root_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let temp_dir = temp.path().join("tmp");
        let data_dir = temp.path().join("data");
        std::fs::create_dir(&temp_dir).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let rel_root = PathBuf::from("./relative_workspace");
        let config = WorkspacePathsConfig::new()
            .with_explicit_root(rel_root.clone())
            .with_temp_dir(temp_dir)
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::RootNotAbsolute(rel_root));
    }

    #[test]
    fn test_strict_paths_root_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let non_existent = temp.path().join("non_existent_workspace_dir_12345");
        let temp_dir = temp.path().join("tmp");
        let data_dir = temp.path().join("data");
        std::fs::create_dir(&temp_dir).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(non_existent.clone())
            .with_temp_dir(temp_dir)
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::RootNotFound(non_existent));
    }

    #[test]
    fn test_strict_paths_root_not_directory() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("not_a_dir.txt");
        std::fs::write(&file_path, b"hello").unwrap();
        let temp_dir = temp.path().join("tmp");
        let data_dir = temp.path().join("data");
        std::fs::create_dir(&temp_dir).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(file_path.clone())
            .with_temp_dir(temp_dir)
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::RootNotDirectory(file_path));
    }

    #[test]
    fn test_strict_paths_missing_temp_dir() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::MissingTempDir);
    }

    #[test]
    fn test_strict_paths_relative_temp_dir_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let rel_temp = PathBuf::from("./relative_tmp");
        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_temp_dir(rel_temp.clone())
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::TempDirNotAbsolute(rel_temp));
    }

    #[test]
    fn test_strict_paths_temp_dir_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data_dir = temp.path().join("data");
        let non_existent_temp = temp.path().join("non_existent_tmp");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&data_dir).unwrap();

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_temp_dir(non_existent_temp.clone())
            .with_external_data_root(data_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::TempDirNotFound(non_existent_temp));
    }

    #[test]
    fn test_strict_paths_missing_data_root() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let temp_dir = temp.path().join("tmp");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&temp_dir).unwrap();

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_temp_dir(temp_dir);
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::MissingDataRoot);
    }

    #[test]
    fn test_strict_paths_relative_data_root_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let temp_dir = temp.path().join("tmp");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&temp_dir).unwrap();

        let rel_data = PathBuf::from("./relative_data");
        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_temp_dir(temp_dir)
            .with_external_data_root(rel_data.clone());
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::DataRootNotAbsolute(rel_data));
    }

    #[test]
    fn test_strict_paths_data_root_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let temp_dir = temp.path().join("tmp");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&temp_dir).unwrap();
        let non_existent_data = temp.path().join("non_existent_data_root_67890");

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_temp_dir(temp_dir)
            .with_external_data_root(non_existent_data.clone());
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::DataRootNotFound(non_existent_data));
    }

    #[test]
    fn test_strict_paths_data_root_not_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let temp_dir = temp.path().join("tmp");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&temp_dir).unwrap();
        let file_data = temp.path().join("data_file.txt");
        std::fs::write(&file_data, b"not a dir").unwrap();

        let config = WorkspacePathsConfig::new()
            .with_explicit_root(root)
            .with_temp_dir(temp_dir)
            .with_external_data_root(file_data.clone());
        let err = config.validate_strict().unwrap_err();
        assert_eq!(err, StrictPathsError::DataRootNotDirectory(file_data));
    }

    #[test]
    fn test_strict_paths_supplied_values_and_no_ambient_fallback() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let data_root = temp.path().join("data");
        let temp_dir = temp.path().join("tmp");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&data_root).unwrap();
        std::fs::create_dir(&temp_dir).unwrap();

        let strict_config =
            StrictWorkspacePathsConfig::new(root.clone(), temp_dir.clone(), data_root.clone());

        let ws = WorkspacePaths::from_strict_config(strict_config).unwrap();
        assert_eq!(ws.root(), &root);
        assert_eq!(ws.data_root(), &data_root);
        assert_eq!(ws.temp_dir(), &temp_dir);

        // Verify absence of ambient fallback: empty config strictly fails
        let empty_config = WorkspacePathsConfig::new();
        assert!(WorkspacePaths::strict(&empty_config).is_err());
    }

    #[test]
    fn test_bootstrap_adapter_retains_legacy_behavior() {
        let config = WorkspacePathsConfig::new();
        let legacy_ws = WorkspacePaths::bootstrap_adapter(&config);
        assert!(!legacy_ws.root().as_os_str().is_empty());
        assert!(!legacy_ws.temp_dir().as_os_str().is_empty());
        assert!(!legacy_ws.data_root().as_os_str().is_empty());
    }
}
