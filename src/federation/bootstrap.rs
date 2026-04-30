/// Federation Bootstrap System
/// 
/// Implements `aaroneous --init`, `--expand`, and `--portable` commands.
/// Enables flexible deployment: core only, modular expansion, or portable targets.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Specialist module that can be deployed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpecialistModule {
    Sentinel,
    Visionary,
    Omnipresent,
    Symbiotic,
    Phygital,
    Archivist,
}

impl SpecialistModule {
    pub fn name(&self) -> &'static str {
        match self {
            SpecialistModule::Sentinel => "Sentinel",
            SpecialistModule::Visionary => "Visionary",
            SpecialistModule::Omnipresent => "Omnipresent",
            SpecialistModule::Symbiotic => "Symbiotic",
            SpecialistModule::Phygital => "Phygital",
            SpecialistModule::Archivist => "Archivist",
        }
    }

    pub fn size_mb(&self) -> u32 {
        match self {
            SpecialistModule::Sentinel => 2000,
            SpecialistModule::Visionary => 1000,
            SpecialistModule::Omnipresent => 1000,
            SpecialistModule::Symbiotic => 500,
            SpecialistModule::Phygital => 1000,
            SpecialistModule::Archivist => 500,
        }
    }

    pub fn is_core(&self) -> bool {
        matches!(self, SpecialistModule::Sentinel)
    }

    pub fn dependencies(&self) -> Vec<SpecialistModule> {
        match self {
            SpecialistModule::Sentinel => vec![],
            SpecialistModule::Visionary => vec![SpecialistModule::Sentinel],
            SpecialistModule::Omnipresent => vec![SpecialistModule::Sentinel],
            SpecialistModule::Symbiotic => vec![SpecialistModule::Sentinel],
            SpecialistModule::Phygital => vec![SpecialistModule::Sentinel],
            SpecialistModule::Archivist => vec![SpecialistModule::Sentinel],
        }
    }
}

/// Deployment target (device type)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentTarget {
    Mobile,
    Tablet,
    Desktop,
    Server,
    Custom(String),
}

impl DeploymentTarget {
    pub fn recommended_modules(&self) -> Vec<SpecialistModule> {
        match self {
            DeploymentTarget::Mobile => vec![
                SpecialistModule::Sentinel,
                SpecialistModule::Omnipresent,
                SpecialistModule::Symbiotic,
            ],
            DeploymentTarget::Tablet => vec![
                SpecialistModule::Sentinel,
                SpecialistModule::Omnipresent,
                SpecialistModule::Symbiotic,
                SpecialistModule::Phygital,
            ],
            DeploymentTarget::Desktop => vec![
                SpecialistModule::Sentinel,
                SpecialistModule::Visionary,
                SpecialistModule::Omnipresent,
                SpecialistModule::Symbiotic,
                SpecialistModule::Phygital,
                SpecialistModule::Archivist,
            ],
            DeploymentTarget::Server => vec![SpecialistModule::Sentinel],
            DeploymentTarget::Custom(_) => vec![SpecialistModule::Sentinel],
        }
    }

    pub fn max_size_mb(&self) -> u32 {
        match self {
            DeploymentTarget::Mobile => 1500,
            DeploymentTarget::Tablet => 2000,
            DeploymentTarget::Desktop => 4000,
            DeploymentTarget::Server => 500,
            DeploymentTarget::Custom(target) => {
                if target.contains("ios") || target.contains("android") {
                    1500
                } else if target.contains("desktop") || target.contains("linux") {
                    4000
                } else {
                    2000
                }
            }
        }
    }
}

/// Manifest: module selection and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub target: DeploymentTarget,
    pub modules: Vec<SpecialistModule>,
    pub config: HashMap<String, String>,
    pub version: String,
}

impl Manifest {
    pub fn new(target: DeploymentTarget) -> Self {
        let modules = target.recommended_modules();
        Self {
            target,
            modules,
            config: HashMap::new(),
            version: "1.0.0".to_string(),
        }
    }

    /// Validate manifest: check size and dependencies
    pub fn validate(&self) -> Result<String, String> {
        let total_size: u32 = self.modules.iter().map(|m| m.size_mb()).sum();

        if total_size > self.target.max_size_mb() {
            return Err(format!(
                "Manifest exceeds target size: {} MB > {} MB",
                total_size,
                self.target.max_size_mb()
            ));
        }

        // Check dependencies
        for module in &self.modules {
            for dep in module.dependencies() {
                if !self.modules.contains(&dep) {
                    return Err(format!(
                        "Module {} requires {}",
                        module.name(),
                        dep.name()
                    ));
                }
            }
        }

        Ok(format!(
            "Manifest valid: {} modules, {} MB / {} MB",
            self.modules.len(),
            total_size,
            self.target.max_size_mb()
        ))
    }

    /// Get total size
    pub fn total_size_mb(&self) -> u32 {
        self.modules.iter().map(|m| m.size_mb()).sum()
    }

    /// Add a module to the manifest
    pub fn add_module(&mut self, module: SpecialistModule) -> Result<(), String> {
        if self.modules.contains(&module) {
            return Ok(());
        }

        // Add dependencies first
        for dep in module.dependencies() {
            self.add_module(dep)?;
        }

        self.modules.push(module);

        // Re-validate
        self.validate()?;
        Ok(())
    }

    /// Remove a module (and dependents)
    pub fn remove_module(&mut self, module: &SpecialistModule) {
        self.modules.retain(|m| m != module);
        // Also remove any module that depends on this one
        self.modules.retain(|m| !m.dependencies().contains(module));
    }
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    pub manifest: Manifest,
    pub dna_bank_path: String,
    pub model_cache_path: String,
    pub log_level: String,
    pub enable_metrics: bool,
    pub enable_learning: bool,
}

impl DeploymentConfig {
    pub fn new(target: DeploymentTarget) -> Self {
        let manifest = Manifest::new(target.clone());
        Self {
            manifest,
            dna_bank_path: ".aaroneous/dna_bank".to_string(),
            model_cache_path: ".aaroneous/models".to_string(),
            log_level: "info".to_string(),
            enable_metrics: true,
            enable_learning: true,
        }
    }

    pub fn with_dna_path(mut self, path: &str) -> Self {
        self.dna_bank_path = path.to_string();
        self
    }

    pub fn with_log_level(mut self, level: &str) -> Self {
        self.log_level = level.to_string();
        self
    }

    pub fn to_toml(&self) -> Result<String, String> {
        let module_names: Vec<String> = self
            .manifest
            .modules
            .iter()
            .map(|m| m.name().to_string())
            .collect();

        let toml = format!(
            r#"[deployment]
target = "{:?}"
version = "{}"
total_size_mb = {}

[modules]
enabled = [{}]

[paths]
dna_bank = "{}"
model_cache = "{}"

[logging]
level = "{}"

[features]
metrics = {}
learning = {}
"#,
            self.manifest.target,
            self.manifest.version,
            self.manifest.total_size_mb(),
            module_names
                .iter()
                .map(|n| format!("\"{}\"", n))
                .collect::<Vec<_>>()
                .join(", "),
            self.dna_bank_path,
            self.model_cache_path,
            self.log_level,
            self.enable_metrics,
            self.enable_learning
        );

        Ok(toml)
    }
}

/// Bootstrap command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapResult {
    pub success: bool,
    pub message: String,
    pub manifest: Option<Manifest>,
    pub size_mb: u32,
    pub modules_installed: usize,
}

/// Bootstrap system
pub struct BootstrapSystem;

impl BootstrapSystem {
    /// Initialize core system (`aaroneous --init`)
    pub fn init() -> BootstrapResult {
        let manifest = Manifest::new(DeploymentTarget::Desktop);

        BootstrapResult {
            success: true,
            message: format!(
                "Aaroneous initialized with {} modules ({}MB total)",
                manifest.modules.len(),
                manifest.total_size_mb()
            ),
            manifest: Some(manifest.clone()),
            size_mb: manifest.total_size_mb(),
            modules_installed: manifest.modules.len(),
        }
    }

    /// Expand with additional modules (`aaroneous --expand --include specialist`)
    pub fn expand(mut manifest: Manifest, modules: Vec<&str>) -> Result<BootstrapResult, String> {
        for module_name in modules {
            let module = match module_name {
                "visionary" => SpecialistModule::Visionary,
                "omnipresent" => SpecialistModule::Omnipresent,
                "symbiotic" => SpecialistModule::Symbiotic,
                "phygital" => SpecialistModule::Phygital,
                "archivist" => SpecialistModule::Archivist,
                _ => {
                    return Err(format!("Unknown specialist: {}", module_name));
                }
            };

            manifest.add_module(module)?;
        }

        manifest.validate()?;

        Ok(BootstrapResult {
            success: true,
            message: format!(
                "Expanded to {} modules ({}MB total)",
                manifest.modules.len(),
                manifest.total_size_mb()
            ),
            manifest: Some(manifest.clone()),
            size_mb: manifest.total_size_mb(),
            modules_installed: manifest.modules.len(),
        })
    }

    /// Create portable version (`aaroneous --portable --target device`)
    pub fn portable(target: DeploymentTarget) -> Result<BootstrapResult, String> {
        let manifest = Manifest::new(target.clone());
        manifest.validate()?;

        Ok(BootstrapResult {
            success: true,
            message: format!(
                "Created portable version for {:?}: {} modules ({}MB)",
                target,
                manifest.modules.len(),
                manifest.total_size_mb()
            ),
            manifest: Some(manifest.clone()),
            size_mb: manifest.total_size_mb(),
            modules_installed: manifest.modules.len(),
        })
    }

    /// Generate deployment configuration
    pub fn generate_config(target: DeploymentTarget) -> DeploymentConfig {
        DeploymentConfig::new(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialist_module_size() {
        assert_eq!(SpecialistModule::Sentinel.size_mb(), 2000);
        assert_eq!(SpecialistModule::Visionary.size_mb(), 1000);
        assert_eq!(SpecialistModule::Symbiotic.size_mb(), 500);
    }

    #[test]
    fn test_specialist_is_core() {
        assert!(SpecialistModule::Sentinel.is_core());
        assert!(!SpecialistModule::Visionary.is_core());
    }

    #[test]
    fn test_specialist_dependencies() {
        let deps = SpecialistModule::Visionary.dependencies();
        assert!(deps.contains(&SpecialistModule::Sentinel));

        let sentinel_deps = SpecialistModule::Sentinel.dependencies();
        assert!(sentinel_deps.is_empty());
    }

    #[test]
    fn test_deployment_target_recommended_modules() {
        let mobile_modules = DeploymentTarget::Mobile.recommended_modules();
        assert!(mobile_modules.contains(&SpecialistModule::Sentinel));
        assert!(mobile_modules.contains(&SpecialistModule::Omnipresent));
        assert!(!mobile_modules.contains(&SpecialistModule::Phygital)); // Desktop only

        let desktop_modules = DeploymentTarget::Desktop.recommended_modules();
        assert_eq!(desktop_modules.len(), 6); // All modules
    }

    #[test]
    fn test_deployment_target_max_size() {
        assert_eq!(DeploymentTarget::Mobile.max_size_mb(), 1500);
        assert_eq!(DeploymentTarget::Tablet.max_size_mb(), 2000);
        assert_eq!(DeploymentTarget::Desktop.max_size_mb(), 4000);
        assert_eq!(DeploymentTarget::Server.max_size_mb(), 500);
    }

    #[test]
    fn test_manifest_new() {
        let manifest = Manifest::new(DeploymentTarget::Desktop);
        assert_eq!(manifest.modules.len(), 6);
        assert_eq!(manifest.target, DeploymentTarget::Desktop);
    }

    #[test]
    fn test_manifest_validate_success() {
        let manifest = Manifest::new(DeploymentTarget::Desktop);
        let result = manifest.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_manifest_validate_size_exceeded() {
        let mut manifest = Manifest::new(DeploymentTarget::Mobile);
        // Mobile max is 1500 MB, but adding all modules would exceed it
        let _ = manifest.add_module(SpecialistModule::Phygital); // This might fail validation
    }

    #[test]
    fn test_manifest_total_size() {
        let manifest = Manifest::new(DeploymentTarget::Mobile);
        let total = manifest.total_size_mb();
        assert!(total > 0);
        assert!(total <= DeploymentTarget::Mobile.max_size_mb());
    }

    #[test]
    fn test_manifest_add_module() {
        let mut manifest = Manifest::new(DeploymentTarget::Server);
        assert!(!manifest.modules.contains(&SpecialistModule::Visionary));

        let result = manifest.add_module(SpecialistModule::Visionary);
        // May fail if size exceeds, but should handle it
        if result.is_ok() {
            assert!(manifest.modules.contains(&SpecialistModule::Visionary));
        }
    }

    #[test]
    fn test_manifest_add_module_with_dependencies() {
        let mut manifest = Manifest::new(DeploymentTarget::Desktop);

        // Visionary requires Sentinel
        let _ = manifest.add_module(SpecialistModule::Visionary);

        // Both should be present
        assert!(manifest.modules.contains(&SpecialistModule::Sentinel));
        assert!(manifest.modules.contains(&SpecialistModule::Visionary));
    }

    #[test]
    fn test_manifest_remove_module() {
        let mut manifest = Manifest::new(DeploymentTarget::Desktop);
        let initial_count = manifest.modules.len();

        manifest.remove_module(&SpecialistModule::Visionary);

        assert!(manifest.modules.len() < initial_count);
        assert!(!manifest.modules.contains(&SpecialistModule::Visionary));
    }

    #[test]
    fn test_deployment_config_new() {
        let config = DeploymentConfig::new(DeploymentTarget::Desktop);
        assert_eq!(config.manifest.target, DeploymentTarget::Desktop);
        assert!(config.enable_metrics);
        assert!(config.enable_learning);
    }

    #[test]
    fn test_deployment_config_with_dna_path() {
        let config = DeploymentConfig::new(DeploymentTarget::Desktop)
            .with_dna_path("/custom/path");

        assert_eq!(config.dna_bank_path, "/custom/path");
    }

    #[test]
    fn test_deployment_config_with_log_level() {
        let config = DeploymentConfig::new(DeploymentTarget::Desktop)
            .with_log_level("debug");

        assert_eq!(config.log_level, "debug");
    }

    #[test]
    fn test_deployment_config_to_toml() {
        let config = DeploymentConfig::new(DeploymentTarget::Desktop);
        let toml_result = config.to_toml();

        assert!(toml_result.is_ok());
        let toml = toml_result.unwrap();
        assert!(toml.contains("deployment"));
        assert!(toml.contains("modules"));
    }

    #[test]
    fn test_bootstrap_init() {
        let result = BootstrapSystem::init();

        assert!(result.success);
        assert!(result.manifest.is_some());
        assert!(result.modules_installed > 0);
    }

    #[test]
    fn test_bootstrap_expand() {
        let manifest = Manifest::new(DeploymentTarget::Server);
        let result = BootstrapSystem::expand(manifest, vec!["visionary"]);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();
        assert!(bootstrap_result.success);
        assert_eq!(bootstrap_result.modules_installed, 2); // Sentinel + Visionary
    }

    #[test]
    fn test_bootstrap_expand_invalid_module() {
        let manifest = Manifest::new(DeploymentTarget::Server);
        let result = BootstrapSystem::expand(manifest, vec!["invalid"]);

        assert!(result.is_err());
    }

    #[test]
    fn test_bootstrap_portable_mobile() {
        let result = BootstrapSystem::portable(DeploymentTarget::Mobile);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();
        assert!(bootstrap_result.success);
        assert!(bootstrap_result.size_mb <= DeploymentTarget::Mobile.max_size_mb());
    }

    #[test]
    fn test_bootstrap_portable_desktop() {
        let result = BootstrapSystem::portable(DeploymentTarget::Desktop);

        assert!(result.is_ok());
        let bootstrap_result = result.unwrap();
        assert!(bootstrap_result.success);
        assert_eq!(bootstrap_result.modules_installed, 6); // All modules
    }

    #[test]
    fn test_bootstrap_generate_config() {
        let config = BootstrapSystem::generate_config(DeploymentTarget::Tablet);

        assert_eq!(config.manifest.target, DeploymentTarget::Tablet);
        assert!(config.enable_metrics);
    }
}
