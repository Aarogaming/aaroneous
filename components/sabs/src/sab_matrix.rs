use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabMatrix {
    pub schema_version: String,
    pub description: String,
    pub surfaces: Vec<SabSurface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabSurface {
    pub name: String,
    pub best_fit_module: String,
    #[serde(default)]
    pub supporting_modules: Vec<String>,
    pub reason: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub registry_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabManifest {
    pub name: String,
    pub best_fit_module: String,
    #[serde(default)]
    pub supporting_modules: Vec<String>,
    pub reason: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub registry_patterns: Vec<String>,
}

impl From<SabManifest> for SabSurface {
    fn from(manifest: SabManifest) -> Self {
        Self {
            name: manifest.name,
            best_fit_module: manifest.best_fit_module,
            supporting_modules: manifest.supporting_modules,
            reason: manifest.reason,
            artifacts: manifest.artifacts,
            registry_patterns: manifest.registry_patterns,
        }
    }
}

pub struct SabMatrixBuilder {
    registry_dir: PathBuf,
}

impl SabMatrixBuilder {
    pub fn new(registry_dir: impl Into<PathBuf>) -> Self {
        Self {
            registry_dir: registry_dir.into(),
        }
    }

    pub fn build(&self) -> Result<SabMatrix> {
        let mut matrix = SabMatrix::load_default()?;

        for manifest in self.load_manifests()? {
            matrix.upsert_surface(manifest.into());
        }

        Ok(matrix)
    }

    pub fn build_and_save(&self) -> Result<SabMatrix> {
        let matrix = self.build()?;
        fs::create_dir_all(&self.registry_dir)?;
        matrix.save_generated_to(&self.registry_dir)?;
        Ok(matrix)
    }

    fn load_manifests(&self) -> Result<Vec<SabManifest>> {
        Ok(self
            .manifest_paths()?
            .into_iter()
            .map(|path| {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read SAB manifest at {}", path.display()))?;
                let manifest = serde_json::from_str::<SabManifest>(&content)
                    .with_context(|| format!("Failed to parse SAB manifest at {}", path.display()))?;
                Ok(manifest)
            })
            .collect::<Result<Vec<_>>>()?)
    }

    fn manifest_paths(&self) -> Result<Vec<PathBuf>> {
        if !self.registry_dir.exists() {
            return Ok(vec![]);
        }

        let mut manifest_paths: Vec<PathBuf> = fs::read_dir(&self.registry_dir)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.is_file()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| {
                            name.starts_with("sab_")
                                && name.ends_with(".json")
                                && name != "sab_matrix.json"
                                && name != "sab_matrix.generated.json"
                        })
                        .unwrap_or(false)
            })
            .collect();

        manifest_paths.sort();

        Ok(manifest_paths)
    }

    fn cache_is_stale(&self) -> Result<bool> {
        let cache_path = self.registry_dir.join("sab_matrix.generated.json");
        if !cache_path.exists() {
            return Ok(true);
        }

        let cache_mtime = fs::metadata(&cache_path)?.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let newest_manifest = self
            .manifest_paths()?
            .into_iter()
            .filter_map(|path| fs::metadata(&path).ok()?.modified().ok())
            .max();

        Ok(matches!(newest_manifest, Some(manifest_mtime) if manifest_mtime > cache_mtime))
    }
}

impl SabMatrix {
    pub fn load_default() -> Result<Self> {
        let matrix: SabMatrix = serde_json::from_str(include_str!("../registry/sab_matrix.json"))?;
        Ok(matrix)
    }

    pub fn load_from_registry_dir(registry_dir: impl Into<PathBuf>) -> Result<Self> {
        SabMatrixBuilder::new(registry_dir).build()
    }

    pub fn load_cached_from_registry_dir(registry_dir: impl AsRef<Path>) -> Result<Option<Self>> {
        let generated_path = registry_dir.as_ref().join("sab_matrix.generated.json");

        if !generated_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&generated_path)
            .with_context(|| format!("Failed to read generated SAB matrix at {}", generated_path.display()))?;
        let matrix = serde_json::from_str::<SabMatrix>(&content)
            .with_context(|| format!("Failed to parse generated SAB matrix at {}", generated_path.display()))?;

        Ok(Some(matrix))
    }

    pub fn save_generated_to(&self, registry_dir: impl AsRef<Path>) -> Result<PathBuf> {
        let generated_path = registry_dir.as_ref().join("sab_matrix.generated.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&generated_path, content)
            .with_context(|| format!("Failed to write generated SAB matrix at {}", generated_path.display()))?;
        Ok(generated_path)
    }

    pub fn load_generated() -> Result<Self> {
        let registry_dir = std::env::current_dir()
            .context("Failed to determine current working directory")?
            .join("registry");

        Self::load_generated_from_registry_dir(registry_dir)
    }

    pub fn load_generated_from_registry_dir(registry_dir: impl Into<PathBuf>) -> Result<Self> {
        let registry_dir = registry_dir.into();

        if registry_dir.exists() {
            let builder = SabMatrixBuilder::new(&registry_dir);
            if !builder.cache_is_stale()? {
                if let Some(matrix) = Self::load_cached_from_registry_dir(&registry_dir)? {
                    return Ok(matrix);
                }
            }

            SabMatrixBuilder::new(registry_dir).build_and_save()
        } else {
            Self::load_default()
        }
    }

    pub fn refresh_generated_cache_from_registry_dir(registry_dir: impl Into<PathBuf>) -> Result<Self> {
        SabMatrixBuilder::new(registry_dir).build_and_save()
    }

    pub fn refresh_generated_cache() -> Result<Self> {
        let registry_dir = std::env::current_dir()
            .context("Failed to determine current working directory")?
            .join("registry");

        if registry_dir.exists() {
            Self::refresh_generated_cache_from_registry_dir(registry_dir)
        } else {
            Self::load_default()
        }
    }

    pub fn upsert_surface(&mut self, surface: SabSurface) {
        if let Some(existing) = self.surfaces.iter_mut().find(|existing| existing.name == surface.name) {
            *existing = surface;
        } else {
            self.surfaces.push(surface);
        }
    }

    pub fn surfaces_for_artifact(&self, artifact: &str) -> Vec<&SabSurface> {
        self.surfaces
            .iter()
            .filter(|surface| {
                surface
                    .artifacts
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(artifact))
            })
            .collect()
    }

    pub fn surface_for_artifact(&self, artifact: &str) -> Option<&SabSurface> {
        self.surfaces_for_artifact(artifact).into_iter().next()
    }

    pub fn surfaces_for_pattern(&self, pattern: &str) -> Vec<&SabSurface> {
        self.surfaces
            .iter()
            .filter(|surface| {
                surface
                    .registry_patterns
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(pattern))
            })
            .collect()
    }

    pub fn surface_for_pattern(&self, pattern: &str) -> Option<&SabSurface> {
        self.surfaces_for_pattern(pattern).into_iter().next()
    }

    pub fn best_fit_module_for_artifact(&self, artifact: &str) -> Option<&str> {
        self.surface_for_artifact(artifact)
            .map(|surface| surface.best_fit_module.as_str())
    }

    pub fn best_fit_module_for_pattern(&self, pattern: &str) -> Option<&str> {
        self.surface_for_pattern(pattern)
            .map(|surface| surface.best_fit_module.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_default_matrix() {
        let matrix = SabMatrix::load_default().unwrap();
        assert!(!matrix.surfaces.is_empty());
    }

    #[test]
    fn finds_known_artifact() {
        let matrix = SabMatrix::load_default().unwrap();
        let surface = matrix.surface_for_artifact("extism.dll").unwrap();
        assert_eq!(surface.best_fit_module, "src/orchestration_loader.rs");
    }

    #[test]
    fn finds_known_pattern() {
        let matrix = SabMatrix::load_default().unwrap();
        let surface = matrix.surface_for_pattern("blackboard").unwrap();
        assert_eq!(surface.best_fit_module, "src/inbox_system.rs");
    }

    #[test]
    fn builder_adds_new_manifest_surface() {
        let temp_dir = std::env::temp_dir().join(format!(
            "sab-matrix-test-{}",
            std::process::id()
        ));

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();

        let manifest_path = temp_dir.join("sab_observability.json");
        let manifest = r#"{
  "name": "observability_pipeline",
  "best_fit_module": "src/enterprise_monitoring.rs",
  "supporting_modules": ["src/phase3_performance_benchmarks.rs"],
  "reason": "Use here when a SAB adds metrics, tracing, alerting, or performance feedback loops.",
  "artifacts": ["prometheus.wasm"],
  "registry_patterns": ["monitoring_agent"]
}"#;
        fs::write(&manifest_path, manifest).unwrap();

        let matrix = SabMatrix::load_from_registry_dir(temp_dir.clone()).unwrap();
        let surface = matrix.surface_for_artifact("prometheus.wasm").unwrap();

        assert_eq!(surface.name, "observability_pipeline");
        assert_eq!(surface.best_fit_module, "src/enterprise_monitoring.rs");

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn saves_and_loads_generated_matrix() {
        let temp_dir = std::env::temp_dir().join(format!(
            "sab-matrix-cache-test-{}",
            std::process::id()
        ));

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();

        let matrix = SabMatrix {
            schema_version: "1.0".to_string(),
            description: "test matrix".to_string(),
            surfaces: vec![SabSurface {
                name: "cached_surface".to_string(),
                best_fit_module: "src/test.rs".to_string(),
                supporting_modules: vec![],
                reason: "cache round trip".to_string(),
                artifacts: vec!["cached.wasm".to_string()],
                registry_patterns: vec!["cached_pattern".to_string()],
            }],
        };

        let generated_path = matrix.save_generated_to(&temp_dir).unwrap();
        assert!(generated_path.exists());

        let loaded = SabMatrix::load_cached_from_registry_dir(&temp_dir).unwrap().unwrap();
        assert_eq!(loaded.surface_for_artifact("cached.wasm").unwrap().name, "cached_surface");

        let round_trip = SabMatrix::load_generated_from_registry_dir(temp_dir.clone()).unwrap();
        assert_eq!(round_trip.surface_for_pattern("cached_pattern").unwrap().best_fit_module, "src/test.rs");

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn stale_generated_matrix_rebuilds_when_manifest_changes() {
        let temp_dir = std::env::temp_dir().join(format!(
            "sab-matrix-stale-test-{}",
            std::process::id()
        ));

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();

        let baseline = SabMatrix {
            schema_version: "1.0".to_string(),
            description: "baseline".to_string(),
            surfaces: vec![SabSurface {
                name: "baseline_surface".to_string(),
                best_fit_module: "src/baseline.rs".to_string(),
                supporting_modules: vec![],
                reason: "baseline cache".to_string(),
                artifacts: vec!["baseline.wasm".to_string()],
                registry_patterns: vec![],
            }],
        };

        baseline.save_generated_to(&temp_dir).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1100));

        let manifest_path = temp_dir.join("sab_new_feature.json");
        let manifest = r#"{
  "name": "new_feature_surface",
  "best_fit_module": "src/new_feature.rs",
  "supporting_modules": [],
  "reason": "new manifest should invalidate cache",
  "artifacts": ["new_feature.wasm"],
  "registry_patterns": ["new_feature_pattern"]
}"#;
        fs::write(&manifest_path, manifest).unwrap();

        let loaded = SabMatrix::load_generated_from_registry_dir(temp_dir.clone()).unwrap();
        assert!(loaded.surface_for_artifact("new_feature.wasm").is_some());
        assert_eq!(loaded.surface_for_pattern("new_feature_pattern").unwrap().best_fit_module, "src/new_feature.rs");

        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn refresh_generated_cache_forces_rebuild_and_save() {
        let temp_dir = std::env::temp_dir().join(format!(
            "sab-matrix-refresh-test-{}",
            std::process::id()
        ));

        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();

        let manifest_path = temp_dir.join("sab_refresh.json");
        let manifest = r#"{
  "name": "refresh_surface",
  "best_fit_module": "src/refresh.rs",
  "supporting_modules": [],
  "reason": "manual refresh should rebuild cache",
  "artifacts": ["refresh.wasm"],
  "registry_patterns": ["refresh_pattern"]
}"#;
        fs::write(&manifest_path, manifest).unwrap();

        let matrix = SabMatrix::refresh_generated_cache_from_registry_dir(temp_dir.clone()).unwrap();
        assert_eq!(matrix.surface_for_artifact("refresh.wasm").unwrap().name, "refresh_surface");
        assert!(temp_dir.join("sab_matrix.generated.json").exists());

        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
