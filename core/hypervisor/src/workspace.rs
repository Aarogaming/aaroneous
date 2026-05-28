// Aaroneous Workspace Paths
// Platform-agnostic path resolution - eliminates Windows lock-in.

use std::path::PathBuf;

/// WorkspacePaths struct for backward compatibility
pub struct WorkspacePaths;

impl WorkspacePaths {
    pub fn workspace_root() -> PathBuf {
        workspace_root()
    }

    pub fn chromosomes_dir() -> PathBuf {
        chromosomes_dir()
    }

    pub fn shaders_dir() -> PathBuf {
        shaders_dir()
    }

    pub fn synapse_dir() -> PathBuf {
        synapse_dir()
    }

    pub fn scripts_dir() -> PathBuf {
        scripts_dir()
    }

    pub fn config_dir() -> PathBuf {
        config_dir()
    }

    pub fn data_dir() -> PathBuf {
        data_dir()
    }

    /// Discover all workspace directories
    pub fn discover() -> Vec<PathBuf> {
        vec![
            chromosomes_dir(),
            shaders_dir(),
            synapse_dir(),
            scripts_dir(),
            config_dir(),
            data_dir(),
        ]
    }

    /// Path to the models directory
    pub fn models() -> PathBuf {
        workspace_root().join("models")
    }

    /// Path to a sovereign model file (named `{name}.gguf` in the models directory)
    pub fn sovereign_model(name: &str) -> PathBuf {
        Self::models().join(format!("{}.gguf", name))
    }
}

/// Resolve the Aaroneous workspace root directory
/// Uses AARONEOUS_WORKSPACE env var, falls back to platform-appropriate defaults
pub fn workspace_root() -> PathBuf {
    std::env::var("AARONEOUS_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
                .join("aaroneous")
        })
}

/// Resolve a path relative to the workspace root
pub fn workspace_path(relative: &str) -> PathBuf {
    workspace_root().join(relative)
}

/// Resolve chromosomes directory
pub fn chromosomes_dir() -> PathBuf {
    workspace_path("chromosomes")
}

/// Resolve shaders directory
pub fn shaders_dir() -> PathBuf {
    workspace_path("shaders")
}

/// Resolve synapse directory
pub fn synapse_dir() -> PathBuf {
    workspace_path("synapse")
}

/// Resolve scripts directory
pub fn scripts_dir() -> PathBuf {
    workspace_path("scripts")
}

/// Resolve config directory
pub fn config_dir() -> PathBuf {
    workspace_path("config")
}

/// Resolve data directory
pub fn data_dir() -> PathBuf {
    workspace_path("data")
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
        let root = workspace_root();
        assert!(root.has_root() || root.to_string_lossy().contains("aaroneous"));

        let chromo = chromosomes_dir();
        assert!(chromo.to_string_lossy().contains("chromosomes"));
    }
}
