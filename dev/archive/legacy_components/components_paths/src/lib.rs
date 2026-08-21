//! Shared workspace path resolver for Aaroneous.
//!
//! Resolves all subdirectories from a single root, determined by:
//! 1. `AARONEOUS_WORKSPACE` environment variable
//! 2. Current working directory (if it contains `Cargo.toml` and `core/`)
//! 3. Fallback to `D:\Aaroneous` (development default)

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WorkspacePaths {
    root: PathBuf,
}

impl WorkspacePaths {
    /// Discover and construct workspace paths.
    pub fn discover() -> Self {
        let root = std::env::var("AARONEOUS_WORKSPACE")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::current_dir()
                    .ok()
                    .filter(|p| p.join("Cargo.toml").exists() && p.join("core").exists())
                    .ok_or(())
            })
            .unwrap_or_else(|_| PathBuf::from("D:\\Aaroneous"));

        Self { root }
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

    pub fn models(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn specialists(&self) -> PathBuf {
        self.root.join("specialists")
    }

    pub fn config(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn data(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn registry(&self) -> PathBuf {
        self.root.join("registry")
    }

    pub fn templates(&self) -> PathBuf {
        self.root.join("templates")
    }

    pub fn exports(&self) -> PathBuf {
        self.root.join("exports")
    }

    pub fn shards(&self) -> PathBuf {
        self.root.join("shards")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn inbox(&self) -> PathBuf {
        self.root.join("inbox")
    }

    pub fn extensions(&self) -> PathBuf {
        self.root.join("extensions")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    // ── Derived paths ─────────────────────────────────────────────────

    pub fn hive_db(&self) -> PathBuf {
        self.root.join("hive.db")
    }

    pub fn training_data(&self) -> PathBuf {
        self.data().join("training_data")
    }

    pub fn sabs(&self) -> PathBuf {
        self.data().join("sabs")
    }

    pub fn routines(&self) -> PathBuf {
        self.data().join("routines")
    }

    pub fn builds(&self) -> PathBuf {
        self.data().join("builds")
    }

    pub fn fabrication(&self) -> PathBuf {
        self.data().join("fabrication")
    }

    pub fn models_inbox(&self) -> PathBuf {
        self.models().join("inbox")
    }

    pub fn federation_memory(&self) -> PathBuf {
        self.data().join("federation_memory.json")
    }

    pub fn links_config(&self) -> PathBuf {
        self.config().join("links.json")
    }

    pub fn specialist_registry(&self) -> PathBuf {
        self.config().join("specialist_registry.json")
    }

    pub fn target_crates(&self) -> PathBuf {
        self.registry().join("target_crates.txt")
    }

    pub fn wasm_extensions(&self) -> PathBuf {
        self.extensions().join("wasm")
    }

    pub fn sab_template(&self) -> PathBuf {
        self.templates().join("universal_sab")
    }

    pub fn native_template(&self) -> PathBuf {
        self.templates().join("universal_native")
    }

    // ── Sovereign-specific paths ──────────────────────────────────────

    pub fn sovereign_model(&self, sovereign: &str) -> PathBuf {
        self.models()
            .join(format!("{}-qwen2.5-7b.gguf", sovereign.to_lowercase()))
    }

    pub fn sovereign_memory(&self, sovereign: &str) -> PathBuf {
        self.data()
            .join(format!("{}_memory.json", sovereign.to_lowercase()))
    }

    pub fn sovereign_hox_preset(&self, name: &str) -> PathBuf {
        self.registry()
            .join(format!("hox_{}.json", name.to_lowercase()))
    }

    /// Alias for `sovereign_hox_preset` — used by relic agent definitions.
    pub fn relic_hox_preset(&self, name: &str) -> PathBuf {
        self.sovereign_hox_preset(name)
    }

    pub fn sovereign_lora_adapter(&self, sovereign: &str) -> PathBuf {
        self.models()
            .join(format!("{}-lora-adapter.bin", sovereign.to_lowercase()))
    }

    pub fn sovereign_distilled(&self, sovereign: &str) -> PathBuf {
        self.models()
            .join(format!("{}-distilled.gguf", sovereign.to_lowercase()))
    }

    pub fn sovereign_training_data(&self, sovereign: &str) -> PathBuf {
        self.training_data()
            .join(format!("{}-training.jsonl", sovereign.to_lowercase()))
    }

    // ── Fabrication workspace for a given crate ───────────────────────

    pub fn fabrication_workspace(&self, crate_name: &str) -> PathBuf {
        self.fabrication().join(crate_name)
    }

    pub fn fabrication_native_workspace(&self, crate_name: &str) -> PathBuf {
        self.fabrication().join(format!("{}_native", crate_name))
    }
}
