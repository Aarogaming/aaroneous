use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::federation::bootstrap::Manifest;
use crate::federation::communication::{CommunicationBus, SpecialistMessage};

/// Metadata for a specific version of a WebAssembly component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVersionInfo {
    pub url: String,
    pub hash: String,
}

/// A component registered in the central registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryComponent {
    pub latest: String,
    pub versions: HashMap<String, RegistryVersionInfo>,
}

/// The entire component registry index (e.g. from registry.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub schema_version: String,
    pub components: HashMap<String, RegistryComponent>,
}

/// Represents a loaded WebAssembly agent bundle
#[derive(Debug, Clone)]
pub struct AgentBundle {
    pub name: String,
    pub version: Version,
    pub path: PathBuf,
}

/// Centralized component registry for versioning and modular hot-swapping
pub struct ComponentRegistry {
    index: Arc<RwLock<RegistryIndex>>,
    active_bundles: Arc<RwLock<HashMap<String, AgentBundle>>>,
    bus: Arc<CommunicationBus>,
}

impl ComponentRegistry {
    /// Create a new Component Registry attached to the communication bus
    pub fn new(bus: Arc<CommunicationBus>) -> Self {
        let default_index = RegistryIndex {
            schema_version: "1.0".to_string(),
            components: HashMap::new(),
        };

        Self {
            index: Arc::new(RwLock::new(default_index)),
            active_bundles: Arc::new(RwLock::new(HashMap::new())),
            bus,
        }
    }

    /// Load the component registry index from a JSON file.
    pub async fn load_index_from_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read registry index: {}", e))?;
        let index: RegistryIndex = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse registry index: {}", e))?;

        let mut w_index = self.index.write().await;
        *w_index = index;
        info!(
            "Loaded component registry index with {} components.",
            w_index.components.len()
        );
        self.bus.broadcast(SpecialistMessage::StatusUpdate(format!(
            "Registry index loaded: {} components",
            w_index.components.len()
        )));
        Ok(())
    }

    /// Resolves the best matching version of a component based on a SemVer requirement (e.g. "^1.0.0").
    pub async fn resolve_dependency(
        &self,
        component_name: &str,
        req_str: &str,
    ) -> Result<RegistryVersionInfo, String> {
        let req = VersionReq::parse(req_str)
            .map_err(|e| format!("Invalid version requirement: {}", e))?;

        let r_index = self.index.read().await;
        let component = r_index
            .components
            .get(component_name)
            .ok_or_else(|| format!("Component '{}' not found in registry", component_name))?;

        let mut best_match: Option<(Version, RegistryVersionInfo)> = None;

        for (v_str, info) in &component.versions {
            if let Ok(v) = Version::parse(v_str)
                && req.matches(&v)
            {
                if let Some((best_v, _)) = &best_match {
                    if v > *best_v {
                        best_match = Some((v, info.clone()));
                    }
                } else {
                    best_match = Some((v, info.clone()));
                }
            }
        }

        best_match
            .map(|(_, info)| info)
            .ok_or_else(|| format!("No version of '{}' satisfies {}", component_name, req_str))
    }

    /// Register a loaded agent bundle into the active state.
    pub async fn register_bundle(
        &self,
        manifest: Manifest,
        sab_path: &Path,
        _rel_path: &Path,
    ) -> Result<(), String> {
        // Find version in manifest or default
        let v = Version::parse(&manifest.version).unwrap_or_else(|_| Version::new(1, 0, 0));
        let bundle = AgentBundle {
            name: manifest
                .target
                .recommended_modules()
                .first()
                .map(|m| m.name().to_string())
                .unwrap_or_default(),
            version: v,
            path: sab_path.to_path_buf(),
        };

        let mut active = self.active_bundles.write().await;
        info!(
            "Registered active bundle: {} v{}",
            bundle.name, bundle.version
        );
        self.bus.broadcast(SpecialistMessage::StatusUpdate(format!(
            "Bundle registered: {} v{}",
            bundle.name, bundle.version
        )));
        active.insert(bundle.name.clone(), bundle);
        Ok(())
    }

    /// Get the active version of a running component
    pub async fn get_active_version(&self, name: &str) -> Option<Version> {
        let active = self.active_bundles.read().await;
        active.get(name).map(|b| b.version.clone())
    }
}
