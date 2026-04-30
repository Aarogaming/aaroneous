// Task Analysis Engine
// Analyzes incoming tasks using LLM reasoning to determine approach, complexity, and resource needs

use crate::llm::{LLMClient, TaskAnalysisContext, TaskAnalysis, Complexity};
use crate::specialist_memory::SpecialistMemory;
use crate::agents::SpecialistAgent;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use std::sync::Arc;
use tracing::{debug, info};

/// Task submitted to the hive for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub description: String,
    pub data_sample: Option<String>,
    pub priority: TaskPriority,
    pub deadline_secs: Option<u64>,
    pub required_skills: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskPriority {
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "Low"),
            TaskPriority::Normal => write!(f, "Normal"),
            TaskPriority::High => write!(f, "High"),
            TaskPriority::Critical => write!(f, "Critical"),
        }
    }
}

/// Result of task analysis
#[derive(Debug, Clone, Serialize)]
pub struct TaskAnalysisResult {
    pub task_id: String,
    pub analysis: TaskAnalysis,
    pub estimated_xp: u32,
    pub required_skills: Vec<String>,
    pub recommended_specialists: Vec<SpecialistRecommendation>,
    pub analysis_time_ms: u128,
    pub confidence: f32, // 0.0-1.0
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecialistRecommendation {
    pub specialist_name: String,
    pub suitability_score: f32, // 0.0-1.0
    pub matching_skills: Vec<String>,
    pub learning_opportunity: bool, // Should this specialist take it to learn?
}

/// Task Analysis Engine
pub struct TaskAnalysisEngine {
    llm_client: Arc<LLMClient>,
}

impl TaskAnalysisEngine {
    /// Create new task analysis engine
    pub fn new(llm_client: Arc<LLMClient>) -> Self {
        Self { llm_client }
    }

    /// Analyze a task using LLM reasoning
    pub async fn analyze_task(
        &self,
        task: &Task,
        available_specialists: &[SpecialistAgent],
    ) -> Result<TaskAnalysisResult> {
        let start = Instant::now();
        info!("Analyzing task: {} ({})", task.id, task.name);

        // Build context for LLM
        let context = TaskAnalysisContext {
            task_id: task.id.clone(),
            file_name: task.name.clone(),
            file_size: task.data_sample.as_ref().map(|s| s.len() as u64).unwrap_or(0),
            file_type: task.tags.join(","),
            data_sample: task.data_sample.clone().unwrap_or_default(),
            specialist_skills: self.extract_available_skills(available_specialists),
            specialist_domain: available_specialists
                .first()
                .map(|s| s.domain.to_string())
                .unwrap_or_default(),
            team_context: format!("{} specialists available", available_specialists.len()),
        };

        // Get LLM analysis
        let analysis = self.llm_client.analyze_task(&context).await?;

        debug!("Task analysis complete: {:?}", analysis);

        // Calculate XP based on complexity
        let estimated_xp = self.estimate_xp_reward(&analysis);

        // Extract required skills from task and analysis
        let mut required_skills = task.required_skills.clone();
        required_skills.extend(
            analysis
                .suggested_collaborators
                .iter()
                .cloned()
                .take(3),
        );
        required_skills.sort();
        required_skills.dedup();

        // Find matching specialists
        let recommended = self
            .find_matching_specialists(&analysis, available_specialists)
            .await;

        let analysis_time = start.elapsed().as_millis();

        let result = TaskAnalysisResult {
            task_id: task.id.clone(),
            analysis,
            estimated_xp,
            required_skills,
            recommended_specialists: recommended,
            analysis_time_ms: analysis_time,
            confidence: 0.85, // Default confidence from LLM
        };

        info!(
            "Task {} analyzed in {}ms: complexity={:?}, estimated_xp={}",
            task.id,
            analysis_time,
            result.analysis.complexity,
            result.estimated_xp
        );

        Ok(result)
    }

    /// Analyze multiple tasks concurrently
    pub async fn analyze_tasks(
        &self,
        tasks: &[Task],
        available_specialists: &[SpecialistAgent],
    ) -> Result<Vec<TaskAnalysisResult>> {
        let mut results = Vec::new();

        for task in tasks {
            match self.analyze_task(task, available_specialists).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!("Failed to analyze task {}: {}", task.id, e);
                }
            }
        }

        // Sort by priority and complexity
        results.sort_by(|a, b| {
            let a_complexity = self.complexity_score(&a.analysis.complexity);
            let b_complexity = self.complexity_score(&b.analysis.complexity);
            b_complexity.partial_cmp(&a_complexity).unwrap()
        });

        Ok(results)
    }

    /// Analyze task considering specialist's memory and past experience
    pub async fn analyze_task_with_memory(
        &self,
        task: &Task,
        specialist: &SpecialistAgent,
        memory: &SpecialistMemory,
        available_specialists: &[SpecialistAgent],
    ) -> Result<TaskAnalysisResult> {
        let mut result = self.analyze_task(task, available_specialists).await?;

        // Check memory for similar tasks
        let relevant_memories = memory.search_memories(&task.name);
        debug!(
            "Found {} relevant memories for specialist {}",
            relevant_memories.len(),
            specialist.name
        );

        // Boost confidence if specialist has relevant experience
        if !relevant_memories.is_empty() {
            result.confidence = (result.confidence + 0.1).min(1.0);
            debug!(
                "Boosted confidence to {:.2} due to past experience",
                result.confidence
            );
        }

        Ok(result)
    }

    /// Estimate XP reward based on task complexity and effort
    fn estimate_xp_reward(&self, analysis: &TaskAnalysis) -> u32 {
        let base_xp = match analysis.complexity {
            Complexity::Simple => 25,
            Complexity::Moderate => 75,
            Complexity::Complex => 200,
        };

        let time_multiplier = (analysis.estimated_time_minutes as f32 / 30.0).min(3.0);
        let confidence_multiplier = analysis.confidence_percentage as f32 / 100.0;

        (base_xp as f32 * time_multiplier * confidence_multiplier) as u32
    }

    /// Find specialists matching task requirements
    async fn find_matching_specialists(
        &self,
        analysis: &TaskAnalysis,
        available_specialists: &[SpecialistAgent],
    ) -> Vec<SpecialistRecommendation> {
        let mut recommendations = Vec::new();

        for specialist in available_specialists {
            // Simple matching: check if specialist's domain aligns with task
            let mut matching_skills = Vec::new();
            let suitability_score = if analysis
                .suggested_collaborators
                .contains(&specialist.name)
            {
                matching_skills.push(specialist.domain.to_string());
                0.95
            } else {
                // Default based on availability
                0.5
            };

            recommendations.push(SpecialistRecommendation {
                specialist_name: specialist.name.clone(),
                suitability_score,
                matching_skills,
                learning_opportunity: suitability_score < 0.8,
            });
        }

        // Sort by suitability
        recommendations.sort_by(|a, b| {
            b.suitability_score
                .partial_cmp(&a.suitability_score)
                .unwrap()
        });

        recommendations.into_iter().take(5).collect()
    }

    /// Extract available skills from specialist list
    fn extract_available_skills(&self, specialists: &[SpecialistAgent]) -> Vec<String> {
        let mut skills = Vec::new();

        for specialist in specialists {
            // Map specialist domain to skills
            let domain_skills: Vec<String> = match specialist.domain {
                crate::agents::Domain::UserInterface => {
                    vec!["UI Design", "UX Analysis", "Frontend Dev"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                }
                crate::agents::Domain::Knowledge => {
                    vec!["Data Analysis", "Pattern Recognition", "Synthesis"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                }
                crate::agents::Domain::Leadership => {
                    vec!["Coordination", "Priority", "Planning"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                }
                crate::agents::Domain::Experience => {
                    vec!["Memory", "Learning", "Reflection"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                }
                crate::agents::Domain::Manufacturing => {
                    vec!["Processing", "Optimization", "Build"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                }
                crate::agents::Domain::Security => {
                    vec!["Validation", "Verification", "Protection"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                }
                crate::agents::Domain::Undefined => vec!["General".to_string()],
            };

            skills.extend(domain_skills);
        }

        skills.sort();
        skills.dedup();
        skills
    }

    /// Convert complexity to numeric score for sorting
    fn complexity_score(&self, complexity: &Complexity) -> f32 {
        match complexity {
            Complexity::Simple => 1.0,
            Complexity::Moderate => 2.0,
            Complexity::Complex => 3.0,
        }
    }
}

/// Task batch for analyzing multiple related tasks
#[derive(Debug, Clone, Serialize)]
pub struct TaskBatch {
    pub batch_id: String,
    pub tasks: Vec<Task>,
    pub batch_type: TaskBatchType,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum TaskBatchType {
    Sequential,   // Tasks must be done in order
    Parallel,     // Tasks can be done concurrently
    Dependent,    // Some tasks depend on others
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_ordering() {
        assert_eq!(TaskPriority::Low as u32, 1);
        assert_eq!(TaskPriority::Critical as u32, 4);
    }

    #[tokio::test]
    async fn test_complexity_scoring() {
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
        let engine = TaskAnalysisEngine::new(Arc::new(llm));

        assert_eq!(engine.complexity_score(&Complexity::Simple), 1.0);
        assert_eq!(engine.complexity_score(&Complexity::Moderate), 2.0);
        assert_eq!(engine.complexity_score(&Complexity::Complex), 3.0);
    }

    #[tokio::test]
    async fn test_xp_estimation() {
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
        let engine = TaskAnalysisEngine::new(Arc::new(llm));

        let simple_task = TaskAnalysis {
            task_id: "t1".to_string(),
            analysis_type: "test".to_string(),
            complexity: Complexity::Simple,
            recommended_approach: "Direct".to_string(),
            estimated_time_minutes: 5,
            confidence_percentage: 90,
            suggested_collaborators: vec![],
            potential_risks: vec![],
            reasoning: "Simple task".to_string(),
        };

        let xp = engine.estimate_xp_reward(&simple_task);
        assert!(xp > 0);
        assert!(xp < 50); // Should be ~25-30 range
    }

    #[test]
    fn test_task_creation() {
        let task = Task {
            id: "task-1".to_string(),
            name: "Analyze Data".to_string(),
            description: "Analyze customer data".to_string(),
            data_sample: Some("sample data".to_string()),
            priority: TaskPriority::High,
            deadline_secs: Some(3600),
            required_skills: vec!["analysis".to_string()],
            tags: vec!["data".to_string(), "analysis".to_string()],
        };

        assert_eq!(task.id, "task-1");
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.required_skills.len(), 1);
    }

    #[test]
    fn test_task_batch_creation() {
        let batch = TaskBatch {
            batch_id: "batch-1".to_string(),
            tasks: vec![],
            batch_type: TaskBatchType::Sequential,
            created_at: chrono::Utc::now(),
        };

        assert_eq!(batch.batch_id, "batch-1");
    }

    #[test]
    fn test_specialist_recommendation() {
        let rec = SpecialistRecommendation {
            specialist_name: "Merlin".to_string(),
            suitability_score: 0.95,
            matching_skills: vec!["Data Analysis".to_string()],
            learning_opportunity: false,
        };

        assert_eq!(rec.specialist_name, "Merlin");
        assert!(rec.suitability_score > 0.9);
        assert!(!rec.learning_opportunity);
    }
}
