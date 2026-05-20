// Error Recovery System
// Learns from task failures and generates recovery strategies

use crate::llm::LLMClient;
use crate::specialist_memory::{SpecialistMemory, MemoryEntry, MemorySource, MemoryType, Confidence};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use chrono;

/// Error that occurred during task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub task_id: String,
    pub specialist_id: String,
    pub error_type: ErrorType,
    pub message: String,
    pub context: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorType {
    ResourceExhaustion,      // Out of memory, disk space, etc.
    TimeoutExceeded,          // Task took too long
    InvalidInput,             // Bad data or parameters
    ExternalServiceFailed,    // API, network, dependency issue
    SkillGapFound,            // Specialist lacks required skill
    DataFormatMismatch,       // Input/output format incompatibility
    ConcurrencyConflict,      // Race condition or deadlock
    UnexpectedFailure,        // Unknown error
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::ResourceExhaustion => write!(f, "ResourceExhaustion"),
            ErrorType::TimeoutExceeded => write!(f, "TimeoutExceeded"),
            ErrorType::InvalidInput => write!(f, "InvalidInput"),
            ErrorType::ExternalServiceFailed => write!(f, "ExternalServiceFailed"),
            ErrorType::SkillGapFound => write!(f, "SkillGapFound"),
            ErrorType::DataFormatMismatch => write!(f, "DataFormatMismatch"),
            ErrorType::ConcurrencyConflict => write!(f, "ConcurrencyConflict"),
            ErrorType::UnexpectedFailure => write!(f, "UnexpectedFailure"),
        }
    }
}

/// Recovery strategy generated from error analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStrategy {
    pub strategy_id: String,
    pub error_type: ErrorType,
    pub actions: Vec<RecoveryAction>,
    pub success_probability: f32,
    pub estimated_time_minutes: u32,
    pub prerequisites: Vec<String>, // Skills or resources needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub sequence: u32,
    pub action_type: ActionType,
    pub description: String,
    pub expected_outcome: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionType {
    Retry,              // Retry same operation
    Escalate,           // Ask for help
    Fallback,           // Use alternative approach
    SkillBoost,         // Acquire missing skill
    ResourceAllocate,   // Allocate more resources
    CacheInvalidate,    // Clear cached data
    WaitAndRetry,       // Wait for resource availability
}

/// Failure analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureAnalysis {
    pub error: ExecutionError,
    pub root_cause: String,
    pub contributing_factors: Vec<String>,
    pub recovery_strategy: RecoveryStrategy,
    pub learning_points: Vec<String>,
}

/// Error recovery engine
pub struct ErrorRecoveryEngine {
    llm_client: Arc<LLMClient>,
    max_retry_attempts: u32,
    learning_enabled: bool,
}

impl ErrorRecoveryEngine {
    /// Create new error recovery engine
    pub fn new(llm_client: Arc<LLMClient>) -> Self {
        Self {
            llm_client,
            max_retry_attempts: 3,
            learning_enabled: true,
        }
    }

    /// Analyze an execution error and generate recovery strategy
    pub async fn analyze_error(
        &self,
        error: ExecutionError,
        memory: Option<&mut SpecialistMemory>,
    ) -> Result<FailureAnalysis> {
        info!("Analyzing error for task {}: {}", error.task_id, error.message);

        // Determine root cause based on error type
        let root_cause = self.determine_root_cause(&error);
        let contributing_factors = self.extract_factors(&error);

        // Generate recovery strategy
        let recovery_strategy = self.generate_strategy(&error, &root_cause).await?;

        // Extract learning points
        let learning_points = self.extract_lessons(&error, &root_cause, &contributing_factors);

        let analysis = FailureAnalysis {
            error: error.clone(),
            root_cause,
            contributing_factors,
            recovery_strategy,
            learning_points,
        };

        // If learning is enabled and memory provided, record lesson
        if self.learning_enabled {
            if let Some(mem) = memory {
                self.record_learning(&error, &analysis, mem).await;
            }
        }

        Ok(analysis)
    }

    /// Determine root cause from error details
    fn determine_root_cause(&self, error: &ExecutionError) -> String {
        match error.error_type {
            ErrorType::ResourceExhaustion => {
                "Insufficient system resources (memory, CPU, disk)".to_string()
            }
            ErrorType::TimeoutExceeded => {
                "Task execution exceeded time limit - possible infinite loop or slow operation"
                    .to_string()
            }
            ErrorType::InvalidInput => {
                "Input data does not meet expected format or constraints".to_string()
            }
            ErrorType::ExternalServiceFailed => {
                "Dependency (API, database, service) unavailable or returned error".to_string()
            }
            ErrorType::SkillGapFound => {
                "Specialist lacks required skill for this task".to_string()
            }
            ErrorType::DataFormatMismatch => {
                "Data format incompatibility between pipeline stages".to_string()
            }
            ErrorType::ConcurrencyConflict => {
                "Race condition or synchronization issue with concurrent access".to_string()
            }
            ErrorType::UnexpectedFailure => {
                "Unknown error - requires investigation".to_string()
            }
        }
    }

    /// Extract contributing factors
    fn extract_factors(&self, error: &ExecutionError) -> Vec<String> {
        let mut factors = vec![];

        // Add message as first factor
        factors.push(format!("Error message: {}", error.message));

        // Add context if available
        if let Some(ctx) = &error.context {
            factors.push(format!("Context: {}", ctx));
        }

        // Add error type specific factors
        match error.error_type {
            ErrorType::TimeoutExceeded => {
                factors.push("Check task complexity vs timeout value".to_string());
                factors.push("Verify system load at time of error".to_string());
            }
            ErrorType::SkillGapFound => {
                factors.push("Specialist needs skill training".to_string());
                factors.push("Consider specialist reassignment".to_string());
            }
            _ => {}
        }

        factors
    }

    /// Extract learning points from failure
    fn extract_lessons(
        &self,
        _error: &ExecutionError,
        root_cause: &str,
        factors: &[String],
    ) -> Vec<String> {
        let mut lessons = vec![];

        lessons.push(format!("Root cause: {}", root_cause));
        lessons.push(format!("Factors involved: {:?}", factors));
        lessons.push("Review error type patterns for this specialist".to_string());
        lessons.push("Consider preventive measures for similar errors".to_string());

        lessons
    }

    /// Generate recovery strategy based on error type
    async fn generate_strategy(
        &self,
        error: &ExecutionError,
        _root_cause: &str,
    ) -> Result<RecoveryStrategy> {
        let strategy_id = format!("recovery-{}-{}", error.task_id, uuid::Uuid::new_v4());

        let actions = match error.error_type {
            ErrorType::TimeoutExceeded => self.timeout_recovery(),
            ErrorType::ResourceExhaustion => self.resource_recovery(),
            ErrorType::SkillGapFound => self.skill_gap_recovery(),
            ErrorType::ExternalServiceFailed => self.external_service_recovery(),
            ErrorType::InvalidInput => self.input_validation_recovery(),
            ErrorType::DataFormatMismatch => self.format_conversion_recovery(),
            ErrorType::ConcurrencyConflict => self.concurrency_recovery(),
            ErrorType::UnexpectedFailure => self.generic_recovery(),
        };

        Ok(RecoveryStrategy {
            strategy_id,
            error_type: error.error_type,
            actions,
            success_probability: 0.65,
            estimated_time_minutes: 5,
            prerequisites: vec![],
        })
    }

    fn timeout_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::WaitAndRetry,
                description: "Wait 10 seconds for system to stabilize".to_string(),
                expected_outcome: "System resources become available".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Retry,
                description: "Retry task with increased timeout".to_string(),
                expected_outcome: "Task completes within new timeout".to_string(),
            },
            RecoveryAction {
                sequence: 3,
                action_type: ActionType::Fallback,
                description: "Use chunked processing instead of single pass".to_string(),
                expected_outcome: "Task completes in smaller batches".to_string(),
            },
        ]
    }

    fn resource_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::ResourceAllocate,
                description: "Allocate additional memory/CPU".to_string(),
                expected_outcome: "More resources available to task".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::CacheInvalidate,
                description: "Clear memory caches to free space".to_string(),
                expected_outcome: "Memory pressure reduced".to_string(),
            },
            RecoveryAction {
                sequence: 3,
                action_type: ActionType::Retry,
                description: "Retry with reduced dataset".to_string(),
                expected_outcome: "Task completes with available resources".to_string(),
            },
        ]
    }

    fn skill_gap_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::SkillBoost,
                description: "Identify missing skill and request training".to_string(),
                expected_outcome: "Specialist acquires required skill".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Escalate,
                description: "Request help from specialist with skill".to_string(),
                expected_outcome: "Knowledge transferred or task delegated".to_string(),
            },
            RecoveryAction {
                sequence: 3,
                action_type: ActionType::Retry,
                description: "Retry task with new skill/help".to_string(),
                expected_outcome: "Task completes successfully".to_string(),
            },
        ]
    }

    fn external_service_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::WaitAndRetry,
                description: "Wait for service to recover (exponential backoff)".to_string(),
                expected_outcome: "External service becomes available".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Fallback,
                description: "Use cached data or alternative service".to_string(),
                expected_outcome: "Task continues with fallback data source".to_string(),
            },
            RecoveryAction {
                sequence: 3,
                action_type: ActionType::Escalate,
                description: "Alert human if service remains unavailable".to_string(),
                expected_outcome: "Human intervention available".to_string(),
            },
        ]
    }

    fn input_validation_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::Fallback,
                description: "Apply data cleaning/normalization".to_string(),
                expected_outcome: "Input data becomes valid".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Retry,
                description: "Retry with cleaned input".to_string(),
                expected_outcome: "Task processes cleaned data successfully".to_string(),
            },
            RecoveryAction {
                sequence: 3,
                action_type: ActionType::Escalate,
                description: "Request human review if cleanup fails".to_string(),
                expected_outcome: "Human can validate or fix input".to_string(),
            },
        ]
    }

    fn format_conversion_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::Fallback,
                description: "Apply format conversion/transformation".to_string(),
                expected_outcome: "Data converted to expected format".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Retry,
                description: "Retry with converted data".to_string(),
                expected_outcome: "Task processes converted data".to_string(),
            },
        ]
    }

    fn concurrency_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::WaitAndRetry,
                description: "Wait to release locks and retry serially".to_string(),
                expected_outcome: "Conflict resolved, operation completes".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Fallback,
                description: "Use single-threaded processing".to_string(),
                expected_outcome: "Task completes without concurrency".to_string(),
            },
        ]
    }

    fn generic_recovery(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                sequence: 1,
                action_type: ActionType::Retry,
                description: "Simple retry of failed operation".to_string(),
                expected_outcome: "Transient error resolved".to_string(),
            },
            RecoveryAction {
                sequence: 2,
                action_type: ActionType::Escalate,
                description: "Escalate to human if retry fails".to_string(),
                expected_outcome: "Human can investigate".to_string(),
            },
        ]
    }

    /// Record learning from failure in memory
    async fn record_learning(
        &self,
        error: &ExecutionError,
        analysis: &FailureAnalysis,
        memory: &mut SpecialistMemory,
    ) {
        let description = format!(
            "Root Cause: {}\nFactors: {:?}\nLessons: {:?}",
            analysis.root_cause, analysis.contributing_factors, analysis.learning_points
        );

        let entry = MemoryEntry {
            id: Uuid::new_v4().to_string(),
            specialist_id: error.specialist_id.clone(),
            memory_type: MemoryType::Lesson,
            title: format!("Failed {}", error.error_type),
            description,
            context: format!("Task: {}", error.task_id),
            confidence: Confidence::High,
            relevance_score: 0.95,
            usage_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["error".to_string(), error.error_type.to_string()],
            related_memories: vec![],
            source: MemorySource::ErrorRecovery,
        };

        memory.record_memory(entry);

        info!(
            "Recorded failure learning for task {} ({})",
            error.task_id, error.error_type
        );
    }

    /// Get retry count and check if more retries available
    pub fn can_retry(&self, current_attempts: u32) -> bool {
        current_attempts < self.max_retry_attempts
    }

    /// Get next retry delay in seconds (exponential backoff)
    pub fn get_retry_delay(&self, attempt: u32) -> u64 {
        // 2^attempt seconds: 2, 4, 8, 16, etc.
        2_u64.saturating_pow(attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_type_display() {
        assert_eq!(ErrorType::TimeoutExceeded.to_string(), "TimeoutExceeded");
        assert_eq!(ErrorType::SkillGapFound.to_string(), "SkillGapFound");
    }

    #[test]
    fn test_recovery_action_creation() {
        let action = RecoveryAction {
            sequence: 1,
            action_type: ActionType::Retry,
            description: "Retry operation".to_string(),
            expected_outcome: "Success".to_string(),
        };

        assert_eq!(action.sequence, 1);
        assert_eq!(action.action_type, ActionType::Retry);
    }

    #[test]
    fn test_execution_error_creation() {
        let error = ExecutionError {
            task_id: "task-1".to_string(),
            specialist_id: "spec-1".to_string(),
            error_type: ErrorType::TimeoutExceeded,
            message: "Task took too long".to_string(),
            context: None,
            timestamp: chrono::Utc::now(),
        };

        assert_eq!(error.error_type, ErrorType::TimeoutExceeded);
    }

    #[tokio::test]
    async fn test_retry_backoff() {
        let llm = crate::llm::LLMClient::new(crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        })
        .await
        .unwrap();

        let engine = ErrorRecoveryEngine::new(Arc::new(llm));

        assert!(engine.can_retry(0));
        assert!(engine.can_retry(2));
        assert!(!engine.can_retry(3));

        assert_eq!(engine.get_retry_delay(0), 1);
        assert_eq!(engine.get_retry_delay(1), 2);
        assert_eq!(engine.get_retry_delay(2), 4);
        assert_eq!(engine.get_retry_delay(3), 8);
    }

    #[tokio::test]
    async fn test_error_recovery_creation() {
        let llm = crate::llm::LLMClient::new(crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        })
        .await
        .unwrap();

        let engine = ErrorRecoveryEngine::new(Arc::new(llm));
        assert_eq!(engine.max_retry_attempts, 3);
        assert!(engine.learning_enabled);
    }

    #[tokio::test]
    async fn test_root_cause_determination() {
        let llm = crate::llm::LLMClient::new(crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        })
        .await
        .unwrap();

        let engine = ErrorRecoveryEngine::new(Arc::new(llm));

        let error = ExecutionError {
            task_id: "task-1".to_string(),
            specialist_id: "spec-1".to_string(),
            error_type: ErrorType::SkillGapFound,
            message: "Missing skill".to_string(),
            context: None,
            timestamp: chrono::Utc::now(),
        };

        let root_cause = engine.determine_root_cause(&error);
        assert!(root_cause.contains("Specialist lacks required skill"));
    }

    #[tokio::test]
    async fn test_strategy_generation_for_timeout() {
        let llm = crate::llm::LLMClient::new(crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2000,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        })
        .await
        .unwrap();

        let engine = ErrorRecoveryEngine::new(Arc::new(llm));

        let error = ExecutionError {
            task_id: "task-1".to_string(),
            specialist_id: "spec-1".to_string(),
            error_type: ErrorType::TimeoutExceeded,
            message: "Task exceeded 60s timeout".to_string(),
            context: None,
            timestamp: chrono::Utc::now(),
        };

        let root_cause = engine.determine_root_cause(&error);
        let strategy = engine.generate_strategy(&error, &root_cause).await.unwrap();

        assert_eq!(strategy.error_type, ErrorType::TimeoutExceeded);
        assert!(!strategy.actions.is_empty());
        assert!(strategy.success_probability > 0.0 && strategy.success_probability <= 1.0);
    }
}
