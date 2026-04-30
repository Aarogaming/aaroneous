// LLM Providers
// Different implementations for various LLM services

mod gguf;
mod mock;

pub use gguf::GGUFProvider;
pub use mock::MockProvider;

use crate::llm::types::*;
use anyhow::Result;
use async_trait::async_trait;

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Analyze a task to determine best approach
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis>;

    /// Find suitable collaborators for a specialist
    async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>>;

    /// Generate execution plan
    async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan>;

    /// Analyze failure and suggest recovery
    async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis>;

    /// Explain a skill
    async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation>;
}
