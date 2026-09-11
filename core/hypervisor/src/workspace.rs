// Aaroneous Workspace Paths
// Platform-agnostic path resolution - eliminates Windows lock-in.

use std::path::{Path, PathBuf};

/// Config for workspace root resolution
#[derive(Debug, Clone, Default)]
pub struct WorkspaceConfig {
    pub explicit_root: Option<PathBuf>,
}

impl WorkspaceConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_explicit_root(mut self, root: PathBuf) -> Self {
        self.explicit_root = Some(root);
        self
    }
}

/// WorkspacePaths struct for backward compatibility
pub struct WorkspacePaths {
    root: PathBuf,
}

impl WorkspacePaths {
    pub fn from_config(config: WorkspaceConfig) -> Self {
        let root = config.explicit_root.unwrap_or_else(|| default_workspace_root());
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn workspace_root() -> PathBuf {
        workspace_root(&WorkspaceConfig::new())
    }

    pub fn chromosomes_dir() -> PathBuf {
        chromosomes_dir(&WorkspaceConfig::new())
    }

    pub fn shaders_dir() -> PathBuf {
        shaders_dir(&WorkspaceConfig::new())
    }

    pub fn synapse_dir() -> PathBuf {
        synapse_dir(&WorkspaceConfig::new())
    }

    pub fn scripts_dir() -> PathBuf {
        scripts_dir(&WorkspaceConfig::new())
    }

    pub fn config_dir() -> PathBuf {
        config_dir(&WorkspaceConfig::new())
    }

    pub fn data_dir() -> PathBuf {
        data_dir(&WorkspaceConfig::new())
    }

    /// Discover all workspace directories
    pub fn discover(config: &WorkspaceConfig) -> Self {
        let root = workspace_root(config);
        Self { root }
    }

    /// Path to the models directory
    pub fn models() -> PathBuf {
        Self::workspace_root().join("models")
    }

    /// Path to a sovereign model file (named `{name}.gguf` in the models directory)
    pub fn sovereign_model(name: &str) -> PathBuf {
        Self::models().join(format!("{}.gguf", name))
    }
}

impl Default for WorkspacePaths {
    fn default() -> Self {
        Self::from_config(WorkspaceConfig::new())
    }
}

/// Resolve the Aaroneous workspace root directory
/// Uses injected config, falls back to platform-appropriate defaults
pub fn workspace_root(config: &WorkspaceConfig) -> PathBuf {
    if let Some(explicit) = &config.explicit_root {
        if explicit.exists() {
            return explicit.clone();
        }
    }
    default_workspace_root()
}

fn default_workspace_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local")
        .join("share")
        .join("aaroneous")
}

/// Resolve a path relative to the workspace root
pub fn workspace_path(config: &WorkspaceConfig, relative: &str) -> PathBuf {
    workspace_root(config).join(relative)
}

/// Resolve chromosomes directory
pub fn chromosomes_dir(config: &WorkspaceConfig) -> PathBuf {
    workspace_path(config, "chromosomes")
}

/// Resolve shaders directory
pub fn shaders_dir(config: &WorkspaceConfig) -> PathBuf {
    workspace_path(config, "shaders")
}

/// Resolve synapse directory
pub fn synapse_dir(config: &WorkspaceConfig) -> PathBuf {
    workspace_path(config, "synapse")
}

/// Resolve scripts directory
pub fn scripts_dir(config: &WorkspaceConfig) -> PathBuf {
    workspace_path(config, "scripts")
}

/// Resolve config directory
pub fn config_dir(config: &WorkspaceConfig) -> PathBuf {
    workspace_path(config, "config")
}

/// Resolve data directory
pub fn data_dir(config: &WorkspaceConfig) -> PathBuf {
    workspace_path(config, "data")
}

/// Ensure a directory exists
pub fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_path_resolution() {
        // Should not panic even without env var
        let root = workspace_root(&WorkspaceConfig::new());
        assert!(root.has_root() || root.to_string_lossy().contains("aaroneous"));

        let chromo = chromosomes_dir(&WorkspaceConfig::new());
        assert!(chromo.to_string_lossy().contains("chromosomes"));
    }

    #[test]
    fn test_workspace_with_explicit_config() {
        let explicit_path = PathBuf::from("/tmp/test_workspace");
        std::fs::create_dir_all(&explicit_path).ok();
        let config = WorkspaceConfig::new().with_explicit_root(explicit_path.clone());
        let root = workspace_root(&config);
        assert_eq!(root, explicit_path);
        std::fs::remove_dir(&explicit_path).ok();
    }
}
