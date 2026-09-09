use crate::semantic_indexing::SemanticIndex;
use anyhow::{Result, anyhow};
use std::collections::HashSet;

/// Knowledge Gap Detector (formerly CuriosityEnzyme)
pub struct KnowledgeGapDetector {
    known_concepts: HashSet<String>,
}

impl KnowledgeGapDetector {
    pub fn new() -> Self {
        Self {
            known_concepts: HashSet::new(),
        }
    }

    /// Return a snapshot of all currently known concepts
    pub fn query_known_concepts(&self) -> Vec<String> {
        let mut concepts: Vec<_> = self.known_concepts.iter().cloned().collect();
        concepts.sort();
        concepts
    }

    /// Register a newly discovered concept
    pub fn add_concept(&mut self, concept: String) {
        self.known_concepts.insert(concept);
    }

    /// Scans the Semantic Index for "entropy" or knowledge gaps.
    pub fn identify_knowledge_gaps(&mut self, index: &SemanticIndex) -> Vec<String> {
        println!("[KnowledgeGapDetector] Analyzing Semantic Index for structural gaps...");

        // 1. Structural Gap Detection based on semantic density and coverage
        let mut gaps = Vec::new();

        if index.entries.is_empty() {
            if !self.known_concepts.contains("unindexed_workspace") {
                gaps.push("workspace semantic index empty; unindexed environment".to_string());
            }
        } else {
            // Find topics with low connectivity or zero access
            for entry in &index.entries {
                if entry.access_count == 0 {
                    let subject = entry.metadata.get("subject").unwrap_or(&entry.id).clone();
                    let gap = format!("unreferenced knowledge cluster: {subject}");
                    if !self.known_concepts.contains(&gap) {
                        gaps.push(gap);
                    }
                }
            }
        }

        // 2. Temporal Forecasting (Predicting future decay)
        let now = chrono::Utc::now();
        for entry in &index.entries {
            let age_days = (now - entry.last_accessed).num_days();
            if age_days > 7 {
                let subject = entry.metadata.get("subject").unwrap_or(&"unknown".to_string()).clone();
                let gap = format!("re-verify stale knowledge: {subject}");
                if !self.known_concepts.contains(&gap) {
                    gaps.push(gap);
                }
            }
        }

        gaps
    }

    /// Predicts future knowledge requirements based on current plan trajectories.
    pub fn forecast_requirements(&self, active_plan: &Option<crate::executive_plan::ExecutivePlan>) -> Vec<String> {
        let mut forecast = Vec::new();
        if let Some(plan) = active_plan {
            println!("[KnowledgeGapDetector] Forecasting requirements for plan: {}", plan.goal);

            // If the plan involves "WASM", predict a need for "WIT-bindings" research
            if plan.goal.to_lowercase().contains("wasm") {
                forecast.push("WASM Component Model (WIT) specifications".to_string());
            }

            // If the plan is complex (>5 steps), predict a need for "distributed consensus"
            if plan.steps.len() > 5 {
                forecast.push("multi-agent coordination protocols".to_string());
            }
        }
        forecast
    }

    /// Generates a "Hunger Intent" to satisfy the identified gaps.
    pub async fn formulate_hunger_intent(&self, gaps: &[String]) -> Result<String> {
        if gaps.is_empty() {
            return Err(anyhow!("No significant knowledge gaps detected."));
        }

        // Aggregate gaps for a holistic research intent
        let target = if gaps.len() > 1 {
            format!("{} (and {} others)", gaps[0], gaps.len() - 1)
        } else {
            gaps[0].clone()
        };

        let intent = format!("Synthesize lore to bridge identified gaps: {}", target);

        println!("[KnowledgeGapDetector] Formulated aggregate hunger intent: {}", intent);
        Ok(intent)
    }
}
