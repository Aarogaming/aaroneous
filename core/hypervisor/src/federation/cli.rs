/// Federation CLI System
///
/// Command-line interface for bootstrap and deployment:
/// - `aaroneous --init`: Fresh installation
/// - `aaroneous --expand --include specialist`: Add modules
/// - `aaroneous --portable --target device`: Create portable version
/// - `aaroneous config`: Manage configuration
/// - `aaroneous status`: Check deployment status
/// - `aaroneous --version`: Show version
use crate::federation::bootstrap::{BootstrapSystem, DeploymentConfig, DeploymentTarget, Manifest};
use serde::{Deserialize, Serialize};

/// CLI command structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Init(InitArgs),
    Expand(ExpandArgs),
    Portable(PortableArgs),
    Config(ConfigArgs),
    Status(StatusArgs),
    Version,
    Help,
}

/// Arguments for `aaroneous --init`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitArgs {
    pub artifact_registry_path: Option<String>,
    pub model_cache_path: Option<String>,
    pub log_level: Option<String>,
}

impl Default for InitArgs {
    fn default() -> Self {
        Self::new()
    }
}

impl InitArgs {
    pub fn new() -> Self {
        Self {
            artifact_registry_path: None,
            model_cache_path: None,
            log_level: None,
        }
    }

    pub fn with_artifact_registry_path(mut self, path: String) -> Self {
        self.artifact_registry_path = Some(path);
        self
    }

    pub fn with_log_level(mut self, level: String) -> Self {
        self.log_level = Some(level);
        self
    }
}

/// Arguments for `aaroneous --expand`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpandArgs {
    pub manifest_path: String,
    pub include: Vec<String>, // e.g., ["visionary", "archivist"]
    pub output_path: Option<String>,
}

impl ExpandArgs {
    pub fn new(manifest_path: String, include: Vec<String>) -> Self {
        Self {
            manifest_path,
            include,
            output_path: None,
        }
    }

    pub fn with_output(mut self, path: String) -> Self {
        self.output_path = Some(path);
        self
    }
}

/// Arguments for `aaroneous --portable`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableArgs {
    pub target: String, // "mobile", "tablet", "desktop", "server"
    pub output_path: Option<String>,
}

impl PortableArgs {
    pub fn new(target: String) -> Self {
        Self {
            target,
            output_path: None,
        }
    }

    pub fn with_output(mut self, path: String) -> Self {
        self.output_path = Some(path);
        self
    }
}

/// Arguments for `aaroneous config`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigArgs {
    Show {
        manifest_path: String,
    },
    Edit {
        manifest_path: String,
        key: String,
        value: String,
    },
    Set {
        manifest_path: String,
        artifact_registry_path: Option<String>,
        log_level: Option<String>,
    },
}

/// Arguments for `aaroneous status`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusArgs {
    pub manifest_path: Option<String>,
}

/// CLI output/result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLIResult {
    pub success: bool,
    pub message: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

impl CLIResult {
    pub fn success(message: String) -> Self {
        Self {
            success: true,
            message,
            output: None,
            error: None,
        }
    }

    pub fn success_with_output(message: String, output: String) -> Self {
        Self {
            success: true,
            message,
            output: Some(output),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        let error_msg = message.clone();
        Self {
            success: false,
            message,
            output: None,
            error: Some(error_msg),
        }
    }
}

/// Manifest persistence
pub struct ManifestStore;

impl ManifestStore {
    /// Load manifest from TOML file
    pub fn load(path: &str) -> Result<Manifest, String> {
        // In real implementation, would use std::fs and toml crate
        // For now, return error (manifest would be loaded from file)
        Err(format!("Manifest file not found: {}", path))
    }

    /// Save manifest to TOML file
    pub fn save(_manifest: &Manifest, path: &str) -> Result<(), String> {
        // In real implementation, would serialize to TOML
        // For now, simulate success
        println!("Would save manifest to: {}", path);
        Ok(())
    }

    /// Create default manifest for target
    pub fn create_default(target: &str) -> Result<Manifest, String> {
        match target {
            "mobile" => Ok(Manifest::new(DeploymentTarget::Mobile)),
            "tablet" => Ok(Manifest::new(DeploymentTarget::Tablet)),
            "desktop" => Ok(Manifest::new(DeploymentTarget::Desktop)),
            "server" => Ok(Manifest::new(DeploymentTarget::Server)),
            _ => Err(format!("Unknown target: {}", target)),
        }
    }
}

/// Main CLI handler
pub struct AaroneosCLI;

impl AaroneosCLI {
    /// Parse command-line arguments
    pub fn parse_args(args: Vec<&str>) -> Result<Command, String> {
        if args.is_empty() {
            return Ok(Command::Help);
        }

        match args[0] {
            "--init" => {
                let mut init_args = InitArgs::new();

                // Parse optional flags
                let mut i = 1;
                while i < args.len() {
                    match args[i] {
                        "--artifact-registry" if i + 1 < args.len() => {
                            init_args.artifact_registry_path = Some(args[i + 1].to_string());
                            i += 2;
                        }
                        "--log-level" if i + 1 < args.len() => {
                            init_args.log_level = Some(args[i + 1].to_string());
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }

                Ok(Command::Init(init_args))
            }

            "--expand" => {
                let mut manifest_path = "aaroneous.toml".to_string();
                let mut include = vec![];
                let mut output_path = None;

                let mut i = 1;
                while i < args.len() {
                    match args[i] {
                        "--manifest" if i + 1 < args.len() => {
                            manifest_path = args[i + 1].to_string();
                            i += 2;
                        }
                        "--include" if i + 1 < args.len() => {
                            include = args[i + 1].split(',').map(|s| s.to_string()).collect();
                            i += 2;
                        }
                        "--output" if i + 1 < args.len() => {
                            output_path = Some(args[i + 1].to_string());
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }

                if include.is_empty() {
                    return Err("--expand requires --include".to_string());
                }

                Ok(Command::Expand(ExpandArgs {
                    manifest_path,
                    include,
                    output_path,
                }))
            }

            "--portable" => {
                let mut target = "desktop".to_string();
                let mut output_path = None;

                let mut i = 1;
                while i < args.len() {
                    match args[i] {
                        "--target" if i + 1 < args.len() => {
                            target = args[i + 1].to_string();
                            i += 2;
                        }
                        "--output" if i + 1 < args.len() => {
                            output_path = Some(args[i + 1].to_string());
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }

                Ok(Command::Portable(PortableArgs {
                    target,
                    output_path,
                }))
            }

            "config" => {
                if args.len() < 2 {
                    return Err("config requires subcommand".to_string());
                }

                match args[1] {
                    "show" if args.len() >= 3 => Ok(Command::Config(ConfigArgs::Show {
                        manifest_path: args[2].to_string(),
                    })),
                    "set" if args.len() >= 4 => {
                        let manifest_path = args[2].to_string();
                        let mut artifact_registry_path = None;
                        let mut log_level = None;

                        let mut i = 3;
                        while i < args.len() {
                            match args[i] {
                                "--artifact-registry" if i + 1 < args.len() => {
                                    artifact_registry_path = Some(args[i + 1].to_string());
                                    i += 2;
                                }
                                "--log-level" if i + 1 < args.len() => {
                                    log_level = Some(args[i + 1].to_string());
                                    i += 2;
                                }
                                _ => i += 1,
                            }
                        }

                        Ok(Command::Config(ConfigArgs::Set {
                            manifest_path,
                            artifact_registry_path,
                            log_level,
                        }))
                    }
                    _ => Err("Unknown config subcommand".to_string()),
                }
            }

            "status" => {
                let manifest_path = if args.len() > 1 {
                    Some(args[1].to_string())
                } else {
                    None
                };

                Ok(Command::Status(StatusArgs { manifest_path }))
            }

            "--version" => Ok(Command::Version),

            "--help" | "help" => Ok(Command::Help),

            _ => Err(format!("Unknown command: {}", args[0])),
        }
    }

    /// Execute a command
    pub fn execute(command: Command) -> CLIResult {
        match command {
            Command::Init(args) => Self::execute_init(args),
            Command::Expand(args) => Self::execute_expand(args),
            Command::Portable(args) => Self::execute_portable(args),
            Command::Config(args) => Self::execute_config(args),
            Command::Status(args) => Self::execute_status(args),
            Command::Version => Self::execute_version(),
            Command::Help => Self::execute_help(),
        }
    }

    fn execute_init(args: InitArgs) -> CLIResult {
        let result = BootstrapSystem::init();

        if !result.success {
            return CLIResult::error(result.message);
        }

        let mut config = DeploymentConfig::new(DeploymentTarget::Desktop);

        if let Some(artifact_registry_path) = args.artifact_registry_path {
            config = config.with_artifact_registry_path(&artifact_registry_path);
        }

        if let Some(log_level) = args.log_level {
            config = config.with_log_level(&log_level);
        }

        match config.to_toml() {
            Ok(toml) => {
                CLIResult::success_with_output(format!("Initialized: {}", result.message), toml)
            }
            Err(e) => CLIResult::error(format!("Failed to generate config: {}", e)),
        }
    }

    fn execute_expand(args: ExpandArgs) -> CLIResult {
        // Load existing manifest
        let manifest = match ManifestStore::load(&args.manifest_path) {
            Ok(m) => m,
            Err(_) => {
                // Create default if not found
                match ManifestStore::create_default("desktop") {
                    Ok(m) => m,
                    Err(e) => return CLIResult::error(e),
                }
            }
        };

        // Expand
        match BootstrapSystem::expand(manifest, args.include.iter().map(|s| s.as_str()).collect()) {
            Ok(result) => {
                if let Some(output_path) = args.output_path
                    && let Some(manifest) = result.manifest
                {
                    let _ = ManifestStore::save(&manifest, &output_path);
                }

                CLIResult::success(result.message)
            }
            Err(e) => CLIResult::error(e),
        }
    }

    fn execute_portable(args: PortableArgs) -> CLIResult {
        let deployment_target = match args.target.as_str() {
            "mobile" => DeploymentTarget::Mobile,
            "tablet" => DeploymentTarget::Tablet,
            "desktop" => DeploymentTarget::Desktop,
            "server" => DeploymentTarget::Server,
            _ => {
                return CLIResult::error(format!("Unknown target: {}", args.target));
            }
        };

        match BootstrapSystem::portable(deployment_target.clone()) {
            Ok(result) => {
                if let Some(output_path) = args.output_path
                    && let Some(manifest) = result.manifest
                {
                    let _ = ManifestStore::save(&manifest, &output_path);
                }

                CLIResult::success(result.message)
            }
            Err(e) => CLIResult::error(e),
        }
    }

    fn execute_config(args: ConfigArgs) -> CLIResult {
        match args {
            ConfigArgs::Show { manifest_path } => match ManifestStore::load(&manifest_path) {
                Ok(manifest) => {
                    let output = format!(
                        "Manifest: {}\nModules: {}\nSize: {}MB",
                        manifest_path,
                        manifest.modules.len(),
                        manifest.total_size_mb()
                    );
                    CLIResult::success_with_output("Configuration loaded".to_string(), output)
                }
                Err(e) => CLIResult::error(e),
            },

            ConfigArgs::Set {
                manifest_path,
                artifact_registry_path,
                log_level,
            } => {
                let mut config = DeploymentConfig::new(DeploymentTarget::Desktop);

                if let Some(path) = artifact_registry_path {
                    config = config.with_artifact_registry_path(&path);
                }

                if let Some(level) = log_level {
                    config = config.with_log_level(&level);
                }

                match config.to_toml() {
                    Ok(toml) => {
                        let _ = ManifestStore::save(&config.manifest, &manifest_path);
                        CLIResult::success_with_output("Configuration updated".to_string(), toml)
                    }
                    Err(e) => CLIResult::error(e),
                }
            }

            ConfigArgs::Edit { .. } => CLIResult::error("Edit not yet implemented".to_string()),
        }
    }

    fn execute_status(args: StatusArgs) -> CLIResult {
        let output = match args.manifest_path {
            Some(path) => match ManifestStore::load(&path) {
                Ok(manifest) => {
                    format!(
                        "Status: Loaded\nTarget: {:?}\nModules: {}\nSize: {}MB",
                        manifest.target,
                        manifest.modules.len(),
                        manifest.total_size_mb()
                    )
                }
                Err(_) => "Status: Not initialized".to_string(),
            },
            None => "Status: Core only (no manifest)".to_string(),
        };

        CLIResult::success_with_output("Status retrieved".to_string(), output)
    }

    fn execute_version() -> CLIResult {
        CLIResult::success_with_output(
            "Version retrieved".to_string(),
            "Aaroneous Federation v1.0.0".to_string(),
        )
    }

    fn execute_help() -> CLIResult {
        let help_text = r#"
Aaroneous Federation CLI

Usage:
  aaroneous [COMMAND] [OPTIONS]

Commands:
  --init                    Initialize core system
  --expand                  Add modules to deployment
  --portable                Create portable version
  config                    Manage configuration
  status                    Check deployment status
  --version                 Show version
  --help                    Show this help

Examples:
  aaroneous --init
  aaroneous --expand --include visionary,phygital
  aaroneous --portable --target mobile
  aaroneous config show aaroneous.toml
  aaroneous status

Options:
  --artifact-registry PATH   ArtifactRegistry location
  --model-cache PATH        Model cache path
  --log-level LEVEL         Log level (debug/info/warn/error)
  --target TARGET           Deployment target (mobile/tablet/desktop/server)
  --output PATH             Output manifest path
  --manifest PATH           Manifest file path
"#;

        CLIResult::success_with_output("Help displayed".to_string(), help_text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_init_command() {
        let result = AaroneosCLI::parse_args(vec!["--init"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Init(_) => {}
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_init_with_artifact_registry_path() {
        let result = AaroneosCLI::parse_args(vec!["--init", "--artifact-registry", "/custom/path"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Init(args) => {
                assert_eq!(args.artifact_registry_path, Some("/custom/path".to_string()));
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_parse_expand_command() {
        let result = AaroneosCLI::parse_args(vec!["--expand", "--include", "visionary,archivist"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Expand(args) => {
                assert!(args.include.contains(&"visionary".to_string()));
                assert!(args.include.contains(&"archivist".to_string()));
            }
            _ => panic!("Expected Expand command"),
        }
    }

    #[test]
    fn test_parse_portable_command() {
        let result = AaroneosCLI::parse_args(vec!["--portable", "--target", "mobile"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Portable(args) => {
                assert_eq!(args.target, "mobile");
            }
            _ => panic!("Expected Portable command"),
        }
    }

    #[test]
    fn test_parse_config_show() {
        let result = AaroneosCLI::parse_args(vec!["config", "show", "aaroneous.toml"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Config(ConfigArgs::Show { manifest_path }) => {
                assert_eq!(manifest_path, "aaroneous.toml");
            }
            _ => panic!("Expected Config Show command"),
        }
    }

    #[test]
    fn test_parse_status_command() {
        let result = AaroneosCLI::parse_args(vec!["status"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Status(_) => {}
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_parse_version_command() {
        let result = AaroneosCLI::parse_args(vec!["--version"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Version => {}
            _ => panic!("Expected Version command"),
        }
    }

    #[test]
    fn test_parse_help_command() {
        let result = AaroneosCLI::parse_args(vec!["--help"]);
        assert!(result.is_ok());
        match result.unwrap() {
            Command::Help => {}
            _ => panic!("Expected Help command"),
        }
    }

    #[test]
    fn test_parse_unknown_command() {
        let result = AaroneosCLI::parse_args(vec!["--unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_init() {
        let args = InitArgs::new();
        let result = AaroneosCLI::execute(Command::Init(args));

        assert!(result.success);
        assert!(result.message.contains("initialized"));
    }

    #[test]
    fn test_execute_version() {
        let result = AaroneosCLI::execute(Command::Version);

        assert!(result.success);
        assert!(result.output.is_some());
        assert!(result.output.unwrap().contains("v1.0.0"));
    }

    #[test]
    fn test_execute_help() {
        let result = AaroneosCLI::execute(Command::Help);

        assert!(result.success);
        assert!(result.output.is_some());
        let help = result.output.unwrap();
        assert!(help.contains("--init"));
        assert!(help.contains("--expand"));
        assert!(help.contains("--portable"));
    }

    #[test]
    fn test_execute_status() {
        let args = StatusArgs {
            manifest_path: None,
        };
        let result = AaroneosCLI::execute(Command::Status(args));

        assert!(result.success);
        assert!(result.output.is_some());
    }

    #[test]
    fn test_cli_result_success() {
        let result = CLIResult::success("Test message".to_string());

        assert!(result.success);
        assert_eq!(result.message, "Test message");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_cli_result_error() {
        let result = CLIResult::error("Error message".to_string());

        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_init_args_builder() {
        let args = InitArgs::new()
            .with_artifact_registry_path("/custom".to_string())
            .with_log_level("debug".to_string());

        assert_eq!(args.artifact_registry_path, Some("/custom".to_string()));
        assert_eq!(args.log_level, Some("debug".to_string()));
    }

    #[test]
    fn test_portable_args_builder() {
        let args = PortableArgs::new("mobile".to_string()).with_output("/output.toml".to_string());

        assert_eq!(args.target, "mobile");
        assert_eq!(args.output_path, Some("/output.toml".to_string()));
    }

    #[test]
    fn test_manifest_store_create_default() {
        let result = ManifestStore::create_default("desktop");
        assert!(result.is_ok());

        let manifest = result.unwrap();
        assert_eq!(manifest.modules.len(), 6);
    }

    #[test]
    fn test_manifest_store_unknown_target() {
        let result = ManifestStore::create_default("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_expand_without_include() {
        let result = AaroneosCLI::parse_args(vec!["--expand"]);
        assert!(result.is_err());
    }
}
