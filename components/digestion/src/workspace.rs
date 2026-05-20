use std::path::PathBuf;

/// Workspace path resolver for Aaroneous.
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
    pub fn data(&self) -> PathBuf { self.root.join("data") }
    pub fn config(&self) -> PathBuf { self.root.join("config") }
    pub fn registry(&self) -> PathBuf { self.root.join("registry") }
    pub fn specialists(&self) -> PathBuf { self.root.join("specialists") }
    pub fn models_inbox(&self) -> PathBuf { self.models().join("inbox") }
}
