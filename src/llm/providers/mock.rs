// Mock LLM Provider
// Used for testing without calling real APIs

use crate::llm::types::*;
use anyhow::Result;
use async_trait::async_trait;
use tracing::debug;

#[derive(Debug, Clone, Default)]
pub struct MockProvider;

#[async_trait]
impl super::LLMProvider for MockProvider {
    async fn analyze_task(&self, context: &TaskAnalysisContext) -> Result<TaskAnalysis> {
        debug!("Mock: Analyzing task {}", context.task_id);

        Ok(TaskAnalysis {
            task_id: context.task_id.clone(),
            analysis_type: match context.file_type.as_str() {
                "csv" => "data_analysis".to_string(),
                "json" => "data_structure_analysis".to_string(),
                "gguf" => "model_analysis".to_string(),
                _ => "general_analysis".to_string(),
            },
            complexity: Complexity::Moderate,
            recommended_approach: format!(
                "Use {} skills to analyze this file",
                context.specialist_skills.join(" + ")
            ),
            estimated_time_minutes: 30,
            confidence_percentage: 85,
            suggested_collaborators: vec!["Circe".to_string()], // Default suggestion
            potential_risks: vec!["Data quality issues".to_string()],
            reasoning: "Mock analysis for testing".to_string(),
        })
    }

    async fn find_collaborators(
        &self,
        specialist: &SpecialistContext,
    ) -> Result<Vec<CollaboratorSuggestion>> {
        debug!("Mock: Finding collaborators for {}", specialist.name);

        let suggestions = match specialist.name.as_str() {
            "Merlin" => vec![
                CollaboratorSuggestion {
                    specialist_name: "Circe".to_string(),
                    reason: "Complementary analysis skills".to_string(),
                    relevance_score: 0.9,
                    complementary_skills: vec!["statistical_analysis".to_string()],
                },
            ],
            "Ariel" => vec![
                CollaboratorSuggestion {
                    specialist_name: "Hephaestus".to_string(),
                    reason: "Complementary tool skills".to_string(),
                    relevance_score: 0.8,
                    complementary_skills: vec!["system_integration".to_string()],
                },
            ],
            _ => vec![
                CollaboratorSuggestion {
                    specialist_name: "Odin".to_string(),
                    reason: "Leadership coordination".to_string(),
                    relevance_score: 0.7,
                    complementary_skills: vec!["orchestration".to_string()],
                },
            ],
        };

        Ok(suggestions)
    }

    async fn generate_plan(
        &self,
        task: &TaskAnalysis,
        specialist: &SpecialistContext,
    ) -> Result<ExecutionPlan> {
        debug!("Mock: Generating plan for {}", specialist.name);

        Ok(ExecutionPlan {
            task_id: task.task_id.clone(),
            specialist_name: specialist.name.clone(),
            steps: vec![
                PlanStep {
                    sequence: 1,
                    description: "Read and validate input".to_string(),
                    estimated_time_minutes: 5,
                    required_skills: vec!["basic_analysis".to_string()],
                    checkpoints: vec!["Data loaded successfully".to_string()],
                },
                PlanStep {
                    sequence: 2,
                    description: "Analyze data".to_string(),
                    estimated_time_minutes: 20,
                    required_skills: task.suggested_collaborators.clone(),
                    checkpoints: vec!["Analysis complete".to_string()],
                },
                PlanStep {
                    sequence: 3,
                    description: "Generate results".to_string(),
                    estimated_time_minutes: 5,
                    required_skills: vec!["synthesis".to_string()],
                    checkpoints: vec!["Results ready".to_string()],
                },
            ],
            total_estimated_time: 30,
            success_probability: 0.85,
            reasoning: "Standard analysis workflow".to_string(),
        })
    }

    async fn analyze_failure(&self, failure: &FailureContext) -> Result<FailureAnalysis> {
        debug!("Mock: Analyzing failure for {}", failure.task_id);

        Ok(FailureAnalysis {
            root_cause: "Unexpected data format".to_string(),
            explanation: "The file had unexpected structure".to_string(),
            prevention_strategy: "Always validate schema first".to_string(),
            recovery_approach: "Use lenient parsing mode".to_string(),
            new_strategy: "Try strict parsing, fall back to lenient".to_string(),
            confidence_percentage: 80,
        })
    }

    async fn explain_skill(
        &self,
        skill_name: &str,
        specialist: &SpecialistContext,
    ) -> Result<SkillExplanation> {
        debug!("Mock: Explaining skill {} for {}", skill_name, specialist.name);

        Ok(SkillExplanation {
            skill_name: skill_name.to_string(),
            description: format!("This is the {} skill", skill_name),
            use_cases: vec![
                "Use case 1".to_string(),
                "Use case 2".to_string(),
            ],
            example: "Here's an example of how to use it".to_string(),
            how_to_improve: "Practice using it on different tasks".to_string(),
            synergies_with: vec!["other_skill1".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider_creation() {
        let provider = MockProvider::default();
        let cloned = provider.clone();
        assert_eq!(format!("{:?}", cloned), format!("{:?}", provider));
    }

    // Integration tests in main llm::tests verify provider functionality
}
