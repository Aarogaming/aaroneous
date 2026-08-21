// LLM Providers
// Different implementations for various LLM services

mod gguf;
pub mod local;
mod mock;
pub mod openai;

pub use gguf::GGUFProvider;
pub use local::LocalLLMProvider;
pub use mock::MockProvider;
pub use openai::OpenAIProvider;

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

    /// Generate UI/UX design variants.
    ///
    /// Used by the Visionary federation specialist to produce candidate
    /// designs for the user to review. Implementations should respect
    /// the `variants_requested` count, style hints, and constraints in
    /// the context.
    async fn generate_design(&self, context: &DesignContext) -> Result<DesignGeneration>;

    /// Generate a plain-text response given an explicit system prompt and user
    /// message.  This is the correct path for sovereign specialists (Odin,
    /// Merlin, Argus …) — it avoids the UI-design framing of `generate_design`
    /// and, for GGUF models, formats the prompt as a proper ChatML / Qwen2
    /// system+user+assistant turn so the model sees a real system turn.
    ///
    /// Domain is passed through for mock routing and caching.
    async fn chat(&self, system_prompt: &str, user_message: &str, domain: &str) -> Result<String>;

    /// Generate a vector embedding for the given text.
    /// Used natively by the Omni Relic / Constellation system for semantic similarity mapping.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}
