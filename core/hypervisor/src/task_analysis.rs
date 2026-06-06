// Task Analysis Module
// Provides task and analysis result types consumed by `task_routing.rs`.

use serde::{Deserialize, Serialize};

/// A task scheduled for routing and execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Per-analysis classification details produced by an upstream analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysis {
    pub analysis_type: String,
    pub estimated_time_minutes: u32,
}

/// Specialist recommendation surfaced as part of an analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistRecommendation {
    pub specialist_name: String,
    pub suitability_score: f32,
}

/// Full analysis result bundle handed to the task router.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAnalysisResult {
    pub analysis: TaskAnalysis,
    pub recommended_specialists: Vec<SpecialistRecommendation>,
}
