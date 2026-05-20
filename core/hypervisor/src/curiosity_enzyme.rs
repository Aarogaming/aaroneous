use crate::semantic_indexing::SemanticIndex;
use crate::prefrontal_cortex::PrefrontalCortex;
use anyhow::{Result, anyhow};
use std::collections::HashSet;

pub struct CuriosityEnzyme {
    known_concepts: HashSet<String>,
}

impl CuriosityEnzyme {
    pub fn new() -> Self {
        Self {
            known_concepts: HashSet::new(),
        }
    }

    /// Scans the Semantic Index for "entropy" or knowledge gaps.
    pub fn identify_knowledge_gaps(&mut self, index: &SemanticIndex) -> Vec<String> {
        println!("[CuriosityEnzyme] Analyzing Semantic Index for structural gaps...");
        
        // 1. Structural Gap Detection (Mocked)
        let mut gaps = Vec::new();
        
        if index.entries.is_empty() {
            gaps.push("general intelligence and system self-awareness".to_string());
        } else if index.entries.len() < 5 {
            gaps.push("advanced Rust systems programming patterns".to_string());
            gaps.push("WASM sandboxing security protocols".to_string());
        }

        // 2. Temporal Forecasting (Predicting future decay)
        let now = chrono::Utc::now();
        for entry in &index.entries {
            let age_days = (now - entry.last_accessed).num_days();
            if age_days > 7 {
                gaps.push(format!("re-verify stale knowledge: {:?}", entry.metadata.get("subject").unwrap_or(&"unknown".to_string())));
            }
        }

        gaps
    }

    /// Predicts future knowledge requirements based on current plan trajectories.
    pub fn forecast_requirements(&self, active_plan: &Option<crate::executive_plan::ExecutivePlan>) -> Vec<String> {
        let mut forecast = Vec::new();
        if let Some(plan) = active_plan {
            println!("[CuriosityEnzyme] Forecasting requirements for plan: {}", plan.goal);
            
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
        
        println!("[CuriosityEnzyme] Formulated aggregate hunger intent: {}", intent);
        Ok(intent)
    }
}
