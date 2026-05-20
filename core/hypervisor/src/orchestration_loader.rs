use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use anyhow::{Result, Context};

/// Registry of all orchestration patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRegistry {
    pub schema_version: String,
    pub description: String,
    pub patterns: HashMap<String, PatternEntry>,
    pub metadata: RegistryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEntry {
    pub plugin_type: String, // Type of plugin (e.g., Orchestration, FileManagement, etc.)
    pub name: String,
    pub description: String,
    pub repo: String,
    pub branch: String,
    #[serde(default)]
    pub subpath: Option<String>,
    pub language: String,
    pub versions: HashMap<String, PatternVersion>,
    #[serde(default)]
    pub aaroneous_mapping: HashMap<String, String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub contexts: Vec<String>,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternVersion {
    pub url: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryMetadata {
    pub last_updated: String,
    pub maintainer: String,
    pub note: String,
}

/// Pattern loader - pulls orchestration patterns from git and integrates them
pub struct PatternLoader {
    cache_dir: PathBuf,
    registry: Option<OrchestrationRegistry>,
}

impl PatternLoader {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            registry: None,
        }
    }

    /// Load the orchestration patterns registry
    pub fn load_registry(&mut self, registry_path: &PathBuf) -> Result<()> {
        let content = std::fs::read_to_string(registry_path)
            .context("Failed to read orchestration patterns registry")?;
        
        let registry: OrchestrationRegistry = serde_json::from_str(&content)
            .context("Failed to parse orchestration patterns registry")?;
        
        self.registry = Some(registry);
        Ok(())
    }

    /// List all available patterns
    pub fn catalog_plugins(&self) -> (Vec<PatternEntry>, Vec<PatternEntry>) {
        match &self.registry {
            Some(reg) => {
                let core_sabs: Vec<PatternEntry> = reg
                    .patterns
                    .values()
                    .filter(|entry| entry.plugin_type == "Orchestration")
                    .cloned()
                    .collect();

                let utilities: Vec<PatternEntry> = reg
                    .patterns
                    .values()
                    .filter(|entry| entry.plugin_type != "Orchestration")
                    .cloned()
                    .collect();

                (core_sabs, utilities)
            }
            None => (vec![], vec![]),
        }
    }

    /// Get details of a specific pattern
    pub fn get_pattern(&self, name: &str) -> Option<PatternEntry> {
        self.registry.as_ref()?.patterns.get(name).cloned()
    }

    /// Clone a pattern from its git source
    pub fn create_hybrid_workflow(&self, pattern_names: Vec<&str>) -> Result<HybridOrchestration> {
        let mut selected_patterns: Vec<PatternEntry> = pattern_names
            .into_iter()
            .filter_map(|name| self.get_pattern(name))
            .collect();

        selected_patterns.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.name.cmp(&b.name)));

        let patterns: Vec<String> = selected_patterns.into_iter().map(|p| p.name).collect();

        if patterns.is_empty() {
            anyhow::bail!("No valid patterns provided for hybrid orchestration.");
        }

        let workflow = HybridOrchestration::new(
            "HybridWorkflow".to_string(),
            patterns,
            CompositionStrategy::Pipeline,
        );

        Ok(workflow)
    }
    
    pub fn clone_pattern(&self, name: &str) -> Result<PathBuf> {
        let pattern = self.get_pattern(name)
            .context("Pattern not found")?;
        
        // Verify plugin type before proceeding
        if pattern.plugin_type != "Orchestration" {
            anyhow::bail!("Unsupported plugin_type: {}", pattern.plugin_type);
        }

        let target_dir = self.cache_dir.join(name);
        
        // Check if already cloned
        if target_dir.exists() {
            println!("Pattern '{}' already cached at {}", name, target_dir.display());
            return Ok(target_dir);
        }
        
        // Clone the repository
        let repo_url = &pattern.repo;
        let branch = &pattern.branch;
        
        println!("Cloning {} from {} (branch: {})", name, repo_url, branch);
        
        let output = Command::new("git")
            .args(["clone", "--depth", "1", "-b", branch, repo_url, target_dir.to_str().unwrap()])
            .output()
            .context("Failed to execute git clone")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git clone failed: {}", stderr);
        }
        
        println!("Successfully cloned {} to {}", name, target_dir.display());
        
        // If subpath specified, return subdirectory
        if let Some(subpath) = &pattern.subpath {
            let subdir = target_dir.join(subpath);
            if subdir.exists() {
                return Ok(subdir);
            }
        }
        
        Ok(target_dir)
    }

    /// Clone and build a pattern as a SAB (if applicable)
    pub fn load_pattern_as_sab(&self, name: &str) -> Result<PathBuf> {
        let pattern = self.get_pattern(name)
            .context("Pattern not found")?;
        
        let source_dir = self.clone_pattern(name)?;
        
        // Handle based on plugin_type
        match pattern.plugin_type.as_str() {
            "Orchestration" => {
            // Check if it's a Rust project (for SAB building)
        let cargo_toml = source_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            println!("Detected Rust project, building as SAB...");
            
            // For now, just return the source directory
            // In production, this would compile to WASM and create a SAB
            return Ok(source_dir);
        }
        
        // For Python/TypeScript, return the source for integration
        println!("Detected {} orchestration project, returning source for integration", pattern.language);
            }
            "Monitoring" | "DataProcessing" | "Communication" => {
                println!("Plugin type '{}' is not yet supported for execution.", pattern.plugin_type);
                anyhow::bail!("Unsupported plugin_type: {}", pattern.plugin_type);
            }
            _ => {
                println!("Unknown plugin_type: {}", pattern.plugin_type);
                anyhow::bail!("Unknown plugin_type: {}", pattern.plugin_type);
            }
        return Ok(source_dir);
    } else {
        anyhow::bail!("Unsupported plugin_type: {}", pattern.plugin_type);
    }
        Ok(source_dir)
    }

    /// Get the Aaroneous mapping for a pattern
    pub fn get_mapping(&self, name: &str) -> Option<HashMap<String, String>> {
        self.get_pattern(name).map(|p| p.aaroneous_mapping)
    }

    /// Get capabilities of a pattern
    pub fn get_capabilities(&self, name: &str) -> Option<Vec<String>> {
        self.get_pattern(name).map(|p| p.capabilities)
    }

    /// Get the declared contexts for a pattern
    pub fn get_contexts(&self, name: &str) -> Option<Vec<String>> {
        self.get_pattern(name).map(|p| p.contexts)
    }

    /// Get the declared priority for a pattern
    pub fn get_priority(&self, name: &str) -> Option<u32> {
        self.get_pattern(name).map(|p| p.priority)
    }
}

/// Combine multiple patterns for hybrid orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridOrchestration {
    pub name: String,
    pub patterns: Vec<String>,
    pub composition: CompositionStrategy,
    pub config: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompositionStrategy {
    Sequential,
    Parallel,
    Pipeline,
    Mesh,
    Hierarchical,
}

impl HybridOrchestration {
    /// Create a new hybrid orchestration from multiple patterns
    pub fn new(name: String, patterns: Vec<String>, composition: CompositionStrategy) -> Self {
        Self {
            name,
            patterns,
            composition,
            config: HashMap::new(),
        }
    }

    /// Add configuration for a specific pattern
    pub fn with_pattern_config(mut self, pattern: &str, config: serde_json::Value) -> Self {
        self.config.insert(pattern.to_string(), config);
        self
    }
}

/// Pattern executor - runs intents through loaded patterns
pub struct PatternExecutor {
    loaded_patterns: HashMap<String, PathBuf>,
}

impl PatternExecutor {
    pub fn new() -> Self {
        Self {
            loaded_patterns: HashMap::new(),
        }
    }

    /// Register a loaded pattern
    pub fn register(&mut self, name: String, path: PathBuf) {
        self.loaded_patterns.insert(name, path);
    }

    /// Execute an intent through a specific pattern
    pub fn execute(&self, pattern_name: &str, intent: &str) -> Result<String> {
        let path = self.loaded_patterns.get(pattern_name)
            .context("Pattern not loaded")?;
        
        // Pattern-specific execution would go here
        // For now, return a placeholder
        Ok(format!("Executed '{}' through pattern at {}", intent, path.display()))
    }

    /// List loaded patterns
    pub fn loaded(&self) -> Vec<String> {
        self.loaded_patterns.keys().cloned().collect()
    }
}

impl Default for PatternExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod plugin_type_tests {
    use super::*;
    
    #[test]
    fn test_load_pattern_as_sab_with_types() {
        let mut registry = OrchestrationRegistry {
            schema_version: "1.0".to_string(),
            description: "Test Registry".to_string(),
            patterns: HashMap::new(),
            metadata: RegistryMetadata {
                last_updated: "2026-05-11".to_string(),
                maintainer: "Test Maintainer".to_string(),
                note: "For testing purposes".to_string(),
            },
        };

        registry.patterns.insert(
            "orchestration".to_string(),
            PatternEntry {
                name: "Valid Orchestration".to_string(),
                description: "Orchestration Plugin".to_string(),
                repo: "https://fake-url.com/orchestration".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "Rust".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["Orchestrating".to_string()],
                dependencies: vec![],
                contexts: vec![],
                priority: 10,
                plugin_type: "Orchestration".to_string(),
            },
        );

        let loader = PatternLoader {
            cache_dir: PathBuf::from("/tmp/cache"),
            registry: Some(registry),
        };

        // Test valid orchestration pattern as SAB
        let load_result = loader.load_pattern_as_sab("orchestration");
        assert!(load_result.is_ok());

        // Test unsupported plugin type
        let unsupported_result = loader.load_pattern_as_sab("unsupported");
        assert!(unsupported_result.is_err());
    }

    #[test]
        let mut registry = OrchestrationRegistry {
            schema_version: "1.0".to_string(),
            description: "Test Registry".to_string(),
            patterns: HashMap::new(),
            metadata: RegistryMetadata {
                last_updated: "2026-05-11".to_string(),
                maintainer: "Test Maintainer".to_string(),
                note: "For testing purposes".to_string(),
            },
        };

        registry.patterns.insert(
            "orchestration".to_string(),
            PatternEntry {
                name: "Valid Orchestration".to_string(),
                description: "Orchestration Plugin".to_string(),
                repo: "https://fake-url.com/orchestration".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "Rust".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["Orchestrating".to_string()],
                dependencies: vec![],
                contexts: vec![],
                priority: 10,
                plugin_type: "Orchestration".to_string(),
            },
        );

        registry.patterns.insert(
            "unsupported".to_string(),
            PatternEntry {
                name: "Unsupported Plugin".to_string(),
                description: "An unsupported plugin type".to_string(),
                repo: "https://fake-url.com/unsupported".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "Python".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["Unsupported".to_string()],
                dependencies: vec![],
                contexts: vec![],
                priority: 0,
                plugin_type: "UnsupportedType".to_string(),
            },
        );

        let loader = PatternLoader {
            cache_dir: PathBuf::from("/tmp/cache"),
            registry: Some(registry),
        };

        // Test valid orchestration
        let orchestration_result = loader.clone_pattern("orchestration");
        assert!(orchestration_result.is_ok());

        // Test unsupported type
        let unsupported_result = loader.clone_pattern("unsupported");
        assert!(unsupported_result.is_err());
    }

    #[test]
        let mut registry = OrchestrationRegistry {
            schema_version: "1.0".to_string(),
            description: "Test Registry".to_string(),
            patterns: HashMap::new(),
            metadata: RegistryMetadata {
                last_updated: "2026-05-11".to_string(),
                maintainer: "Test Maintainer".to_string(),
                note: "For testing purposes".to_string(),
            },
        };

        registry.patterns.insert(
            "test_orchestration".to_string(),
            PatternEntry {
                name: "Test Orchestration".to_string(),
                description: "An Orchestration Plugin".to_string(),
                repo: "https://fake-url.com/orchestration".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "Rust".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["Orchestrating".to_string()],
                dependencies: vec![],
                contexts: vec![],
                priority: 10,
                plugin_type: "Orchestration".to_string(),
            },
        );

        let loader = PatternLoader {
            cache_dir: PathBuf::from("/tmp/cache"),
            registry: Some(registry),
        };
        
        let patterns = loader.list_patterns();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].2, "Orchestration"); // plugin_type check
    }
}
mod tests {
    use super::*;

    #[test]
    fn test_hybrid_orchestration_creation() {
        let mut registry = OrchestrationRegistry {
            schema_version: "1.1".to_string(),
            description: "Test Registry for Hybrid Workflows".to_string(),
            patterns: HashMap::new(),
            metadata: RegistryMetadata {
                last_updated: "2026-05-11".to_string(),
                maintainer: "Tester".to_string(),
                note: "Hybrid workflow test registry".to_string(),
            },
        };

        registry.patterns.insert(
            "fake_pattern_1".to_string(),
            PatternEntry {
                name: "Fake Pattern 1".to_string(),
                description: "First test pattern".to_string(),
                repo: "https://fake-repo-1.com".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "rust".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["fake_capability".to_string()],
                dependencies: vec![],
                contexts: vec![],
                priority: 8,
                plugin_type: "Orchestration".to_string(),
            },
        );
        registry.patterns.insert(
            "fake_pattern_2".to_string(),
            PatternEntry {
                name: "Fake Pattern 2".to_string(),
                description: "Second test pattern".to_string(),
                repo: "https://fake-repo-2.com".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "python".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["another_fake_capability".to_string()],
                dependencies: vec![],
                contexts: vec![],
                priority: 4,
                plugin_type: "WorkflowManagement".to_string(),
            },
        );

        let loader = PatternLoader {
            cache_dir: PathBuf::from("/tmp/test-cache"),
            registry: Some(registry),
        };

        let hybrid_result = loader.create_hybrid_workflow(vec!["crewai", "monitoring_agent"]);
        assert!(hybrid_result.is_ok());

        let hybrid_orchestration = hybrid_result.unwrap();
        assert_eq!(hybrid_orchestration.patterns[0], "CrewAI Pattern");
        assert_eq!(hybrid_orchestration.patterns[1], "Monitoring Agent Pattern");
        assert!(hybrid_orchestration.patterns.contains(&"CrewAI Pattern".to_string()));
        assert!(hybrid_orchestration.patterns.contains(&"Monitoring Agent Pattern".to_string()));
        assert_eq!(hybrid_orchestration.composition, CompositionStrategy::Pipeline);
    }

    fn test_pattern_loader_creation() {
        let loader = PatternLoader::new(PathBuf::from("/tmp/test-cache"));
        assert!(loader.registry.is_none());
    }

    #[test]
    fn test_pattern_executor() {
        let mut executor = PatternExecutor::new();
        executor.register("test".into(), PathBuf::from("/test/path"));
        
        assert!(executor.loaded().contains(&"test".to_string()));
    }

    #[test]
    fn test_hybrid_orchestration() {
        let hybrid = HybridOrchestration::new(
            "my_orchestration".to_string(),
            vec!["crewai".to_string(), "blackboard".to_string()],
            CompositionStrategy::Pipeline,
        );
        
        assert_eq!(hybrid.patterns.len(), 2);
    }

    #[test]
    fn test_pattern_metadata_helpers() {
        let mut registry = OrchestrationRegistry {
            schema_version: "1.2".to_string(),
            description: "Metadata Registry".to_string(),
            patterns: HashMap::new(),
            metadata: RegistryMetadata {
                last_updated: "2026-05-11".to_string(),
                maintainer: "Tester".to_string(),
                note: "Metadata helpers test".to_string(),
            },
        };

        registry.patterns.insert(
            "meta_pattern".to_string(),
            PatternEntry {
                plugin_type: "Orchestration".to_string(),
                name: "Meta Pattern".to_string(),
                description: "Metadata test pattern".to_string(),
                repo: "https://example.com/meta".to_string(),
                branch: "main".to_string(),
                subpath: None,
                language: "rust".to_string(),
                versions: HashMap::new(),
                aaroneous_mapping: HashMap::new(),
                capabilities: vec!["planning".to_string()],
                dependencies: vec!["crewai".to_string()],
                contexts: vec!["planning".to_string(), "workflow".to_string()],
                priority: 12,
            },
        );

        let loader = PatternLoader { cache_dir: PathBuf::from("/tmp/test-cache"), registry: Some(registry) };

        assert_eq!(loader.get_contexts("meta_pattern").unwrap().len(), 2);
        assert_eq!(loader.get_priority("meta_pattern").unwrap(), 12);
    }
}
