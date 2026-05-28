// Model Environment Detection
// Auto-detects which model loading software user has installed
// and discovers models from their locations

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info};

/// Supported model loading environments
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelEnvironment {
    LMStudio,      // Jan.ai style - ~/.lm-studio/models
    Ollama,        // ollama pull qwen:1.8b
    LocalAI,       // LocalAI with local models
    CustomPath,    // User-specified directory
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

    pub fn get_search_paths(&self) -> Vec<PathBuf> {
        match self {
            ModelEnvironment::LMStudio => {
                let mut paths = Vec::new();
                if let Ok(home) = std::env::var("USERPROFILE") {
                    paths.push(PathBuf::from(format!("{}/.lm-studio/models", home)));
                    paths.push(PathBuf::from(format!("{}\\AppData\\Local\\LM Studio\\models", home)));
                }
                if let Ok(home) = std::env::var("HOME") {
                    paths.push(PathBuf::from(format!("{}/.lm-studio/models", home)));
                }
                paths
            }
            ModelEnvironment::Ollama => {
                let mut paths = Vec::new();
                if let Ok(home) = std::env::var("USERPROFILE") {
                    paths.push(PathBuf::from(format!("{}/.ollama/models", home)));
                }
                if let Ok(home) = std::env::var("HOME") {
                    paths.push(PathBuf::from(format!("{}/.ollama/models", home)));
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
    pub fn new(environment: ModelEnvironment, model_path: PathBuf, is_installed: bool, confidence: f32) -> Self {
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

    /// Scan for installed model environments
    pub fn scan(&mut self) -> Result<()> {
        info!("Scanning for model environments...");
        self.detected_environments.clear();

        // Check each environment
        self.check_lm_studio();
        self.check_ollama();
        self.check_localai();

        // Sort by detection confidence
        self.detected_environments
            .sort_by(|a, b| b.detection_confidence.partial_cmp(&a.detection_confidence).unwrap());

        info!(
            "Found {} model environments",
            self.detected_environments.len()
        );

        Ok(())
    }

    /// Check for LM Studio installation
    fn check_lm_studio(&mut self) {
        let home = match std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            Ok(h) => h,
            Err(_) => return,
        };

        let model_path = PathBuf::from(format!("{}/.lm-studio/models", home));
        let alt_path = PathBuf::from(format!("{}\\AppData\\Local\\LM Studio\\models", home));

        let (path, confidence) = if model_path.exists() {
            debug!("Found LM Studio models at: {}", model_path.display());
            (model_path, 0.95)
        } else if alt_path.exists() {
            debug!("Found LM Studio models at: {}", alt_path.display());
            (alt_path, 0.95)
        } else {
            // Check if directory structure exists (but maybe no models yet)
            let config_dir = PathBuf::from(format!("{}/.lm-studio", home));
            if config_dir.exists() {
                debug!("Found LM Studio config directory");
                (model_path, 0.7) // Lower confidence if no models
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

    /// Check for Ollama installation
    fn check_ollama(&mut self) {
        // Check if ollama command exists
        let has_ollama = if cfg!(windows) {
            which::which("ollama").is_ok()
        } else {
            std::fs::metadata("/usr/local/bin/ollama").is_ok()
                || std::fs::metadata("/usr/bin/ollama").is_ok()
        };

        if has_ollama {
            debug!("Found Ollama installation");

            // Ollama stores models in different locations
            let model_path = if cfg!(windows) {
                let home = std::env::var("USERPROFILE").unwrap_or_default();
                PathBuf::from(format!("{}/.ollama/models", home))
            } else {
                PathBuf::from("~/.ollama/models")
            };

            self.detected_environments.push(DetectedEnvironment::new(
                ModelEnvironment::Ollama,
                model_path,
                true,
                0.85,
            ));
        }
    }

    /// Check for LocalAI installation
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

        println!("\n✓ Found {} model environment(s):\n", self.detected_environments.len());

        for (idx, env) in self.detected_environments.iter().enumerate() {
            let status = if env.is_installed { "✓ Installed" } else { "📁 Path Only" };
            let confidence = (env.detection_confidence * 100.0) as u32;

            println!("  {}. {} [{}%] {}", idx + 1, env.environment.name(), confidence, status);
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

        println!("\n(Automatically selecting best option: {})\n", 
            self.detected_environments[0].environment.name());

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
        let paths = env.get_search_paths();
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
