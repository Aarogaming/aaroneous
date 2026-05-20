// Aaroneous Configuration Validator
// Ensures all configurations are valid before runtime initialization

use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::workspace::WorkspacePaths;

/// Validated HiveRuntimeConfig - ensures all fields are safe before use
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedRuntimeConfig {
    #[validate(length(min = 1, message = "db_path cannot be empty"))]
    pub db_path: String,

    #[validate(length(min = 1, message = "inbox_folder cannot be empty"))]
    pub inbox_folder: String,

    #[validate(length(min = 1, message = "output_folder cannot be empty"))]
    pub output_folder: String,

    #[validate(range(min = 10, max = 10000, message = "update_interval_ms must be between 10-10000ms"))]
    pub update_interval_ms: u64,

    #[validate(range(min = 1, max = 16, message = "max_concurrent_tasks must be between 1-16"))]
    pub max_concurrent_tasks: usize,

    pub enable_persistence: bool,
    pub enable_ingestion: bool,
    pub enable_dashboard: bool,
    pub crisis_response_enabled: bool,
}

impl ValidatedRuntimeConfig {
    /// Create and validate a runtime config
    pub fn new(
        db_path: String,
        inbox_folder: String,
        output_folder: String,
        update_interval_ms: u64,
        max_concurrent_tasks: usize,
        enable_persistence: bool,
        enable_ingestion: bool,
        enable_dashboard: bool,
        crisis_response_enabled: bool,
    ) -> Result<Self, validator::ValidationErrors> {
        let config = ValidatedRuntimeConfig {
            db_path,
            inbox_folder,
            output_folder,
            update_interval_ms,
            max_concurrent_tasks,
            enable_persistence,
            enable_ingestion,
            enable_dashboard,
            crisis_response_enabled,
        };

        config.validate()?;
        Ok(config)
    }

    /// Load and validate config from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let paths = WorkspacePaths::discover();
        let db_path = std::env::var("AARONEOUS_DB_PATH")
            .unwrap_or_else(|_| paths.hive_db().to_string_lossy().to_string());
        let inbox_folder = std::env::var("AARONEOUS_INBOX")
            .unwrap_or_else(|_| paths.inbox().to_string_lossy().to_string());
        let output_folder = std::env::var("AARONEOUS_OUTPUT")
            .unwrap_or_else(|_| paths.data().join("processed").to_string_lossy().to_string());
        let update_interval_ms = std::env::var("AARONEOUS_UPDATE_INTERVAL")
            .unwrap_or_else(|_| "100".to_string())
            .parse()?;
        let max_concurrent_tasks = std::env::var("AARONEOUS_MAX_TASKS")
            .unwrap_or_else(|_| "4".to_string())
            .parse()?;
        let enable_persistence = std::env::var("AARONEOUS_PERSISTENCE")
            .unwrap_or_else(|_| "true".to_string())
            .parse()?;
        let enable_ingestion = std::env::var("AARONEOUS_INGESTION")
            .unwrap_or_else(|_| "true".to_string())
            .parse()?;
        let enable_dashboard = std::env::var("AARONEOUS_DASHBOARD")
            .unwrap_or_else(|_| "true".to_string())
            .parse()?;
        let crisis_response_enabled = std::env::var("AARONEOUS_CRISIS")
            .unwrap_or_else(|_| "true".to_string())
            .parse()?;

        Self::new(
            db_path,
            inbox_folder,
            output_folder,
            update_interval_ms,
            max_concurrent_tasks,
            enable_persistence,
            enable_ingestion,
            enable_dashboard,
            crisis_response_enabled,
        )
        .map_err(|e| format!("Configuration validation failed: {}", e).into())
    }
}

/// Validated data ingestion configuration
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ValidatedIngestionConfig {
    #[validate(length(min = 1))]
    pub inbox_path: String,

    #[validate(range(min = 1, max = 2000, message = "max_file_size_mb must be between 1-2000MB"))]
    pub max_file_size_mb: u64,

    #[validate(range(min = 100, max = 1000000, message = "content_sample_size must be between 100B-1MB"))]
    pub content_sample_size_bytes: usize,

    pub file_watcher_enabled: bool,
}

impl ValidatedIngestionConfig {
    /// Create and validate an ingestion config
    pub fn new(
        inbox_path: String,
        max_file_size_mb: u64,
        content_sample_size_bytes: usize,
        file_watcher_enabled: bool,
    ) -> Result<Self, validator::ValidationErrors> {
        let config = ValidatedIngestionConfig {
            inbox_path,
            max_file_size_mb,
            content_sample_size_bytes,
            file_watcher_enabled,
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_runtime_config() {
        let config = ValidatedRuntimeConfig::new(
            "hive.db".to_string(),
            "inbox".to_string(),
            "output".to_string(),
            100,
            4,
            true,
            true,
            true,
            true,
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_invalid_update_interval() {
        let config = ValidatedRuntimeConfig::new(
            "hive.db".to_string(),
            "inbox".to_string(),
            "output".to_string(),
            5, // too small
            4,
            true,
            true,
            true,
            true,
        );
        assert!(config.is_err());
    }

    #[test]
    fn test_invalid_concurrent_tasks() {
        let config = ValidatedRuntimeConfig::new(
            "hive.db".to_string(),
            "inbox".to_string(),
            "output".to_string(),
            100,
            20, // too large
            true,
            true,
            true,
            true,
        );
        assert!(config.is_err());
    }

    #[test]
    fn test_valid_ingestion_config() {
        let config = ValidatedIngestionConfig::new(
            "inbox".to_string(),
            500,
            10000,
            true,
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_invalid_file_size() {
        let config = ValidatedIngestionConfig::new(
            "inbox".to_string(),
            3000, // too large
            10000,
            true,
        );
        assert!(config.is_err());
    }
}
