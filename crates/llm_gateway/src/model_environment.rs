// Model Environment Detection
// Auto-detects which model loading software user has installed
// and discovers models from their locations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

/// Environment detection config POD - replaces std::env lookups
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct ModelEnvironmentConfig {
    pub check_user_profile: bool,
    pub check_home_dir: bool,
    /// Ollama binary paths for Windows fallback detection (optional)
    pub ollama_bin_paths: [Option<String>; 2],
}

impl ModelEnvironmentConfig {
    pub fn new() -> Self {
        Self {
            check_user_profile: true,
            check_home_dir: true,
            ollama_bin_paths: [
                Some("/usr/local/bin/ollama".to_string()),
                Some("/usr/bin/ollama".to_string()),
            ],
        }
    }

    pub fn with_user_profile(mut self, check: bool) -> Self {
        self.check_user_profile = check;
        self
    }

    pub fn with_home_dir(mut self, check: bool) -> Self {
        self.check_home_dir = check;
        self
    }

    pub fn with_ollama_bin_path<P>(mut self, path: P) -> Self
    where
        P: Into<String>,
    {
        let paths = &self.ollama_bin_paths;
        if paths[0].is_none() {
            self.ollama_bin_paths[0] = Some(path.into());
        } else if paths[1].is_none() {
            self.ollama_bin_paths[1] = Some(path.into());
        }
        self
    }
}

/// Supported model loading environments
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelEnvironment {
    LMStudio,   // Jan.ai style - ~/.lm-studio/models
    Ollama,     // ollama pull qwen:1.8b
    LocalAI,    // LocalAI with local models
    CustomPath, // User-specified directory
}

impl ModelEnvironment {
    pub fn name(&self) -> &'static str {
        match self {
            ModelEnvironment::LMStudio => "LM Studio",
            ModelEnvironment::Ollama => "Ollama",
            ModelEnvironment::LocalAI => "LocalAI",
            ModelEnvironment::CustomPath => "Custom Path",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ModelEnvironment::LMStudio => "LM Studio - Modern UI for local models",
            ModelEnvironment::Ollama => "Ollama - Simple command-line model manager",
            ModelEnvironment::LocalAI => "LocalAI - Kubernetes-ready local AI",
            ModelEnvironment::CustomPath => "Custom directory",
        }
    }

    pub fn website(&self) -> Option<&'static str> {
        match self {
            ModelEnvironment::LMStudio => Some("https://lmstudio.ai"),
            ModelEnvironment::Ollama => Some("https://ollama.ai"),
            ModelEnvironment::LocalAI => Some("https://localai.io"),
            ModelEnvironment::CustomPath => None,
        }
    }

    pub fn get_search_paths(&self, config: &ModelEnvironmentConfig) -> Vec<PathBuf> {
        match self {
            ModelEnvironment::LMStudio => {
                let mut paths = Vec::new();
                // Check USERPROFILE and HOME via config-gated calls
                if std::env::var("USERPROFILE").is_ok() || (config.check_user_profile && std::env::var("HOME").is_ok()) {
                    let home = match std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                        Ok(h) => PathBuf::from(h),
                        Err(_) => return paths,
                    };
                    paths.push(home.join(".lmstudio").join("models"));
                    paths.push(home.join(".cache").join("lm-studio").join("models"));
                    // Legacy locations retained for existing installations.
                    paths.push(home.join(".lm-studio").join("models"));
                    paths.push(
                        home.join("AppData")
                            .join("Local")
                            .join("LM Studio")
                            .join("models"),
                    );
                }
                paths
            }
            ModelEnvironment::Ollama => {
                let mut paths = Vec::new();
                // Ollama paths use standard locations - no config gating needed for these static paths
                if std::env::var("USERPROFILE").is_ok() {
                    let home = PathBuf::from(std::env::var("USERPROFILE").unwrap());
                    paths.push(PathBuf::from(format!("{}/.ollama/models", home.display())));
                }
                if std::env::var("HOME").is_ok() {
                    let home = PathBuf::from(std::env::var("HOME").unwrap());
                    paths.push(PathBuf::from(format!("{}/.ollama/models", home.display())));
                }
                paths
            }
            ModelEnvironment::LocalAI => {
                vec![
                    PathBuf::from("./models"),
                    PathBuf::from("../models"),
                    PathBuf::from("/opt/local-ai/models"),
                ]
            }
            ModelEnvironment::CustomPath => Vec::new(),
        }
    }
}

/// Detected model environment with its paths
#[derive(Debug, Clone)]
pub struct DetectedEnvironment {
    pub environment: ModelEnvironment,
    pub model_path: PathBuf,
    pub is_installed: bool,
    pub detection_confidence: f32, // 0.0-1.0
}

impl DetectedEnvironment {
    pub fn new(
        environment: ModelEnvironment,
        model_path: PathBuf,
        is_installed: bool,
        confidence: f32,
    ) -> Self {
        Self {
            environment,
            model_path,
            is_installed,
            detection_confidence: confidence,
        }
    }
}

/// Environment detector
pub struct ModelEnvironmentDetector {
    detected_environments: Vec<DetectedEnvironment>,
}

impl ModelEnvironmentDetector {
    /// Create new detector
    pub fn new() -> Self {
        Self {
            detected_environments: Vec::new(),
        }
    }

    /// Scan for installed model environments with config injection
    pub fn scan(&mut self, config: &ModelEnvironmentConfig) -> Result<()> {
        info!("Scanning for model environments...");
        self.detected_environments.clear();

        // Check each environment
        self.check_lm_studio(config);
        self.check_ollama(config);
        self.check_localai();

        // Sort by detection confidence
        self.detected_environments.sort_by(|a, b| {
            b.detection_confidence
                .partial_cmp(&a.detection_confidence)
                .unwrap()
        });

        info!(
            "Found {} model environments",
            self.detected_environments.len()
        );

        Ok(())
    }

    /// Check for LM Studio installation (with config injection)
    fn check_lm_studio(&mut self, config: &ModelEnvironmentConfig) {
        if !config.check_user_profile && !config.check_home_dir {
            return;
        }

        let home = match std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            Ok(h) => PathBuf::from(h),
            Err(_) => return,
        };

        let candidates = [
            home.join(".lmstudio").join("models"),
            home.join(".cache").join("lm-studio").join("models"),
            home.join(".lm-studio").join("models"),
            home.join("AppData")
                .join("Local")
                .join("LM Studio")
                .join("models"),
        ];

        let (path, confidence) = if candidates[0].exists() {
            debug!("Found LM Studio models at: {}", candidates[0].display());
            (candidates[0].clone(), 0.95)
        } else if candidates[1].exists() {
            debug!("Found LM Studio models at: {}", candidates[1].display());
            (candidates[1].clone(), 0.95)
        } else if candidates[2].exists() {
            debug!("Found LM Studio models at: {}", candidates[2].display());
            (candidates[2].clone(), 0.85)
        } else if candidates[3].exists() {
            debug!("Found LM Studio models at: {}", candidates[3].display());
            (candidates[3].clone(), 0.85)
        } else {
            // Check if directory structure exists (but maybe no models yet)
            let config_dir = home.join(".lmstudio");
            if config_dir.exists() {
                debug!("Found LM Studio config directory");
                (candidates[0].clone(), 0.7) // Lower confidence if no models
            } else {
                return;
            }
        };

        let is_installed = path.exists() && std::fs::read_dir(&path).is_ok();
        self.detected_environments.push(DetectedEnvironment::new(
            ModelEnvironment::LMStudio,
            path,
            is_installed,
            confidence,
        ));
    }

    /// Check for Ollama installation (with config injection)
    fn check_ollama(&mut self, config: &ModelEnvironmentConfig) {
        // Check if ollama command exists
        let has_ollama = if cfg!(windows) {
            which::which("ollama").is_ok()
        } else {
            std::fs::metadata(config.ollama_bin_paths[0].as_deref().unwrap_or("/usr/local/bin/ollama")).is_ok()
                || std::fs::metadata(config.ollama_bin_paths[1].as_deref().unwrap_or("/usr/bin/ollama")).is_ok()
        };

        if has_ollama {
            debug!("Found Ollama installation");

            // Ollama stores models in different locations
            let model_path = PathBuf::from("~/.ollama/models");

            self.detected_environments.push(DetectedEnvironment::new(
                ModelEnvironment::Ollama,
                model_path,
                true,
                0.85,
            ));
        }
    }

    /// Check for LocalAI installation (with config injection)
    fn check_localai(&mut self) {
        let has_localai = which::which("local-ai").is_ok();

        if has_localai {
            debug!("Found LocalAI installation");

            let model_path = PathBuf::from("./models");

            self.detected_environments.push(DetectedEnvironment::new(
                ModelEnvironment::LocalAI,
                model_path,
                true,
                0.80,
            ));
        }
    }

    /// Get all detected environments
    pub fn all_environments(&self) -> &[DetectedEnvironment] {
        &self.detected_environments
    }

    /// Get best detected environment
    pub fn get_best_environment(&self) -> Option<&DetectedEnvironment> {
        self.detected_environments.first()
    }

    /// Get environment by type
    pub fn get_environment(&self, env_type: ModelEnvironment) -> Option<&DetectedEnvironment> {
        self.detected_environments
            .iter()
            .find(|e| e.environment == env_type)
    }

    /// Print detected environments in friendly format
    pub fn print_detected_environments(&self) {
        if self.detected_environments.is_empty() {
            println!("\n⚠️  No model loading software detected.\n");
            println!("Aaroneous supports:");
            println!("  • LM Studio (https://lmstudio.ai)");
            println!("  • Ollama (https://ollama.ai)");
            println!("  • LocalAI (https://localai.io)\n");
            println!("Install one of these, then run model discovery again.\n");
            return;
        }

        println!(
            "\n✓ Found {} model environment(s):\n",
            self.detected_environments.len()
        );

        for (idx, env) in self.detected_environments.iter().enumerate() {
            let status = if env.is_installed {
                "✓ Installed"
            } else {
                "📁 Path Only"
            };
            let confidence = (env.detection_confidence * 100.0) as u32;

            println!(
                "  {}. {} [{}%] {}",
                idx + 1,
                env.environment.name(),
                confidence,
                status
            );
            println!("     {}", env.environment.description());
            println!("     Models: {}", env.model_path.display());

            if let Some(website) = env.environment.website() {
                println!("     Website: {}", website);
            }
            println!();
        }
    }

    /// Interactive selection
    pub fn select_environment_interactive(&self) -> Option<&DetectedEnvironment> {
        if self.detected_environments.is_empty() {
            return None;
        }

        if self.detected_environments.len() == 1 {
            println!(
                "\nUsing: {}\n",
                self.detected_environments[0].environment.name()
            );
            return Some(&self.detected_environments[0]);
        }

        println!("\n🔍 Multiple model environments found:");
        println!("Which one should Aaroneous use?\n");

        for (idx, env) in self.detected_environments.iter().enumerate() {
            println!("  {}. {}", idx + 1, env.environment.name());
        }

        println!(
            "\n(Automatically selecting best option: {})\n",
            self.detected_environments[0].environment.name()
        );

        Some(&self.detected_environments[0])
    }
}

impl Default for ModelEnvironmentDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_environment_names() {
        assert_eq!(ModelEnvironment::LMStudio.name(), "LM Studio");
        assert_eq!(ModelEnvironment::Ollama.name(), "Ollama");
    }

    #[test]
    fn test_lm_studio_search_paths() {
        let env = ModelEnvironment::LMStudio;
        let config = ModelEnvironmentConfig::new();
        let paths = env.get_search_paths(&config);
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_detector_creation() {
        let detector = ModelEnvironmentDetector::new();
        assert!(detector.all_environments().is_empty());
    }

    #[test]
    fn test_detected_environment() {
        let env = DetectedEnvironment::new(
            ModelEnvironment::LMStudio,
            PathBuf::from("/models"),
            true,
            0.95,
        );
        assert_eq!(env.environment, ModelEnvironment::LMStudio);
        assert_eq!(env.detection_confidence, 0.95);
    }
}
