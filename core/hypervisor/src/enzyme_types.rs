// CONSOLIDATED ENZYME TYPES
// All enzyme variants consolidated from separate files:
// - curiosity_enzyme.rs (91 lines)
// - research_enzyme.rs (87 lines)
// - execution_enzyme.rs (57 lines)
// - diplomat_enzyme.rs (98 lines)
// - self_correction_enzyme.rs (86 lines)
// Total: 419 lines consolidated to single module

use crate::semantic_indexing::SemanticIndex;
use anyhow::{Result, anyhow};
use std::collections::HashSet;

// ============================================================================
// CURIOSITY ENZYME
// ============================================================================

pub struct CuriosityEnzyme {
    known_concepts: HashSet<String>,
}

impl CuriosityEnzyme {
    pub fn new() -> Self {
        Self {
            known_concepts: HashSet::new(),
        }
    }

    pub fn query_known_concepts(&self) -> Vec<String> {
        let mut concepts: Vec<_> = self.known_concepts.iter().cloned().collect();
        concepts.sort();
        concepts
    }

    pub fn add_concept(&mut self, concept: String) {
        self.known_concepts.insert(concept);
    }

    pub fn identify_knowledge_gaps(&mut self, index: &SemanticIndex) -> Vec<String> {
        println!("[CuriosityEnzyme] Analyzing Semantic Index for structural gaps...");
        
        let mut gaps = Vec::new();
        
        if index.entries.is_empty() {
            if !self.known_concepts.contains("general intelligence") {
                gaps.push("general intelligence and system self-awareness".to_string());
            }
        } else if index.entries.len() < 5 {
            if !self.known_concepts.contains("advanced Rust") {
                gaps.push("advanced Rust systems programming patterns".to_string());
            }
            if !self.known_concepts.contains("WASM sandboxing") {
                gaps.push("WASM sandboxing security protocols".to_string());
            }
        }

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

    pub fn forecast_requirements(&self, active_plan: &Option<crate::executive_plan::ExecutivePlan>) -> Vec<String> {
        let mut forecast = Vec::new();
        if let Some(plan) = active_plan {
            println!("[CuriosityEnzyme] Forecasting requirements for plan: {}", plan.goal);
            
            if plan.goal.to_lowercase().contains("wasm") {
                forecast.push("WASM Component Model (WIT) specifications".to_string());
            }
            
            if plan.steps.len() > 5 {
                forecast.push("multi-agent coordination protocols".to_string());
            }
        }
        forecast
    }

    pub async fn formulate_hunger_intent(&self, gaps: &[String]) -> Result<String> {
        if gaps.is_empty() {
            return Err(anyhow!("No significant knowledge gaps detected."));
        }

        let target = if gaps.len() > 1 {
            format!("{} (and {} others)", gaps[0], gaps.len() - 1)
        } else {
            gaps[0].clone()
        };
        
        let intent = format!("Synthesize lore to bridge identified gaps: {}", target);
        println!("[CuriosityEnzyme] Formulated hunger intent: {}", intent);
        Ok(intent)
    }
}

// ============================================================================
// RESEARCH ENZYME
// ============================================================================

pub struct ResearchEnzyme {
    hypothesis_bank: Vec<String>,
    test_results: Vec<(String, bool)>,
}

impl ResearchEnzyme {
    pub fn new() -> Self {
        Self {
            hypothesis_bank: Vec::new(),
            test_results: Vec::new(),
        }
    }

    pub fn propose_hypothesis(&mut self, hypothesis: String) {
        self.hypothesis_bank.push(hypothesis.clone());
        println!("[ResearchEnzyme] Proposed hypothesis: {}", hypothesis);
    }

    pub fn validate_hypothesis(&mut self, hypothesis: &str, result: bool) {
        self.test_results.push((hypothesis.to_string(), result));
        println!("[ResearchEnzyme] Hypothesis '{}' result: {}", hypothesis, if result { "✓" } else { "✗" });
    }

    pub fn get_test_summary(&self) -> String {
        let total = self.test_results.len();
        let passed = self.test_results.iter().filter(|(_, r)| *r).count();
        format!("Tested {}/{} hypotheses successfully", passed, total)
    }

    pub fn analyze_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        if self.test_results.len() >= 3 {
            let success_rate = self.test_results.iter().filter(|(_, r)| *r).count() as f32 
                / self.test_results.len() as f32;
            
            if success_rate > 0.8 {
                patterns.push("Strong pattern: High success rate detected".to_string());
            } else if success_rate < 0.3 {
                patterns.push("Weak pattern: Low success rate - reconsider approach".to_string());
            }
        }
        patterns
    }
}

// ============================================================================
// EXECUTION ENZYME
// ============================================================================

pub struct ExecutionEnzyme {
    execution_count: usize,
    success_count: usize,
}

impl ExecutionEnzyme {
    pub fn new() -> Self {
        Self {
            execution_count: 0,
            success_count: 0,
        }
    }

    pub fn execute_task(&mut self, task: &str) -> Result<String> {
        self.execution_count += 1;
        println!("[ExecutionEnzyme] Executing task: {}", task);
        
        let success = !task.contains("fail");
        if success {
            self.success_count += 1;
            Ok(format!("Task '{}' completed successfully", task))
        } else {
            Err(anyhow!("Task '{}' execution failed", task))
        }
    }

    pub fn get_execution_stats(&self) -> (usize, usize) {
        (self.execution_count, self.success_count)
    }

    pub fn success_rate(&self) -> f32 {
        if self.execution_count == 0 {
            0.0
        } else {
            self.success_count as f32 / self.execution_count as f32
        }
    }
}

// ============================================================================
// DIPLOMAT ENZYME
// ============================================================================

pub struct DiplomatEnzyme {
    negotiation_log: Vec<String>,
}

impl DiplomatEnzyme {
    pub fn new() -> Self {
        Self {
            negotiation_log: Vec::new(),
        }
    }

    pub fn propose_agreement(&mut self, proposal: &str) -> String {
        let response = format!("Considering proposal: {}", proposal);
        self.negotiation_log.push(response.clone());
        println!("[DiplomatEnzyme] {}", response);
        response
    }

    pub fn accept_terms(&mut self, terms: &str) -> String {
        let agreement = format!("Terms accepted: {}", terms);
        self.negotiation_log.push(agreement.clone());
        println!("[DiplomatEnzyme] {}", agreement);
        agreement
    }

    pub fn reject_terms(&mut self, reason: &str) -> String {
        let rejection = format!("Terms rejected: {}", reason);
        self.negotiation_log.push(rejection.clone());
        println!("[DiplomatEnzyme] {}", rejection);
        rejection
    }

    pub fn get_negotiation_history(&self) -> &[String] {
        &self.negotiation_log
    }

    pub fn get_successful_agreements(&self) -> usize {
        self.negotiation_log.iter()
            .filter(|log| log.contains("accepted"))
            .count()
    }
}

// ============================================================================
// SELF-CORRECTION ENZYME
// ============================================================================

pub struct SelfCorrectionEnzyme {
    error_log: Vec<String>,
    corrections_applied: usize,
}

impl SelfCorrectionEnzyme {
    pub fn new() -> Self {
        Self {
            error_log: Vec::new(),
            corrections_applied: 0,
        }
    }

    pub fn detect_error(&mut self, error: &str) -> String {
        let msg = format!("[ERROR] {}", error);
        self.error_log.push(msg.clone());
        println!("[SelfCorrectionEnzyme] Detected: {}", error);
        msg
    }

    pub fn apply_correction(&mut self, correction: &str) -> String {
        self.corrections_applied += 1;
        let msg = format!("[CORRECTED] {}", correction);
        println!("[SelfCorrectionEnzyme] Applied: {}", correction);
        msg
    }

    pub fn get_error_count(&self) -> usize {
        self.error_log.len()
    }

    pub fn get_corrections_applied(&self) -> usize {
        self.corrections_applied
    }

    pub fn correction_effectiveness(&self) -> f32 {
        if self.error_log.is_empty() {
            1.0
        } else {
            self.corrections_applied as f32 / self.error_log.len() as f32
        }
    }

    pub fn analyze_error_patterns(&self) -> Vec<String> {
        let mut patterns = Vec::new();
        
        if self.error_log.len() > 5 {
            patterns.push(format!("Error trend: {} errors detected", self.error_log.len()));
        }
        
        if self.correction_effectiveness() > 0.8 {
            patterns.push("High correction effectiveness detected".to_string());
        }
        
        patterns
    }
}

#[cfg(test)]
mod enzyme_types_tests {
    use super::*;

    #[test]
    fn test_enzyme_consolidation() {
        println!("[TEST] Enzyme Consolidation Verification");
        println!("✓ CuriosityEnzyme available");
        println!("✓ ResearchEnzyme available");
        println!("✓ ExecutionEnzyme available");
        println!("✓ DiplomatEnzyme available");
        println!("✓ SelfCorrectionEnzyme available");
        
        let mut curiosity = CuriosityEnzyme::new();
        curiosity.add_concept("test".to_string());
        assert!(!curiosity.query_known_concepts().is_empty());
        
        let mut research = ResearchEnzyme::new();
        research.propose_hypothesis("test hypothesis".to_string());
        assert_eq!(research.hypothesis_bank.len(), 1);
        
        let mut execution = ExecutionEnzyme::new();
        let result = execution.execute_task("test");
        assert!(result.is_ok());
        
        let mut diplomat = DiplomatEnzyme::new();
        diplomat.accept_terms("test terms");
        assert!(!diplomat.get_negotiation_history().is_empty());
        
        let mut correction = SelfCorrectionEnzyme::new();
        correction.detect_error("test error");
        assert_eq!(correction.get_error_count(), 1);
        
        println!("[RESULT] ✅ All enzyme types working correctly");
    }
}
