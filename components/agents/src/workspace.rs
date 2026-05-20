use std::path::PathBuf;

/// Workspace path resolver for Aaroneous.
/// Mirrors the hypervisor's WorkspacePaths for use in the agents crate.
#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    root: PathBuf,
}

impl WorkspacePaths {
    pub fn discover() -> Self {
        let root = std::env::var("AARONEOUS_WORKSPACE")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::current_dir().ok().filter(|p| {
                    p.join("Cargo.toml").exists() && p.join("core").exists()
                })
            })
            .unwrap_or_else(|| PathBuf::from("D:\\Aaroneous"));
        Self { root }
    }

    pub fn from_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf { &self.root }
    pub fn models(&self) -> PathBuf { self.root.join("models") }
    pub fn config(&self) -> PathBuf { self.root.join("config") }
    pub fn data(&self) -> PathBuf { self.root.join("data") }
    pub fn registry(&self) -> PathBuf { self.root.join("registry") }
    pub fn hive_db(&self) -> PathBuf { self.root.join("hive.db") }

    pub fn sovereign_model(&self, sovereign: &str) -> PathBuf {
        self.models().join(format!("{}-qwen2.5-7b.gguf", sovereign.to_lowercase()))
    }

    pub fn sovereign_hox_preset(&self, name: &str) -> PathBuf {
        self.registry().join(format!("hox_specialist_{}.json", name.to_lowercase()))
    }

    pub fn relic_hox_preset(&self, name: &str) -> PathBuf {
        self.registry().join(format!("hox_relic_{}.json", name.to_lowercase()))
    }
}
