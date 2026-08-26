//! crates/orchestrator/src/intent_engine.rs
//! User Intent Parsing → Specialist Dispatch Pipeline
//!
//! Converts natural language user intent into CAS commands,
//! routes them through the MDP router to the optimal specialist,
//! and dispatches execution.

use crate::linguistic_transducer::{CasCommand, LinguisticTransducer};
use crate::mdps_router::{RoutableTask, RoutingDecision, Specialist, TaskRoutingEngine, TaskType};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Parsed user intent with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub raw_text: String,
    pub cas_command: CasCommand,
    pub extracted_skills: Vec<String>,
    pub complexity: f64,
    pub urgency: f64,
}

/// Result of dispatching an intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    pub intent: ParsedIntent,
    pub routing: RoutingDecision,
    pub task_id: String,
}

/// The Intent Engine — full pipeline from natural language to specialist dispatch
pub struct IntentEngine {
    transducer: LinguisticTransducer,
    router: TaskRoutingEngine,
}

impl Default for IntentEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl IntentEngine {
    pub fn new() -> Self {
        let specialists = vec![
            Specialist {
                id: "specialist_ariel".to_string(),
                name: "Ariel".to_string(),
                skills: vec![
                    "sensor_node".to_string(),
                    "tensor_forge".to_string(),
                    "ui".to_string(),
                    "visual".to_string(),
                    "render".to_string(),
                ],
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 5.0,
            },
            Specialist {
                id: "specialist_merlin".to_string(),
                name: "Merlin".to_string(),
                skills: vec![
                    "thought_kernel".to_string(),
                    "tensor_forge".to_string(),
                    "analysis".to_string(),
                    "knowledge".to_string(),
                ],
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 8.0,
            },
            Specialist {
                id: "specialist_odin".to_string(),
                name: "Odin".to_string(),
                skills: vec![
                    "thought_kernel".to_string(),
                    "nat_bridge".to_string(),
                    "leadership".to_string(),
                    "orchestration".to_string(),
                ],
                capacity: 1.0,
                success_rate: 0.95,
                avg_completion_time: 3.0,
            },
            Specialist {
                id: "specialist_dionysus".to_string(),
                name: "Dionysus".to_string(),
                skills: vec![
                    "sensor_node".to_string(),
                    "thought_kernel".to_string(),
                    "experience".to_string(),
                    "memory".to_string(),
                ],
                capacity: 1.0,
                success_rate: 0.85,
                avg_completion_time: 6.0,
            },
            Specialist {
                id: "specialist_hephaestus".to_string(),
                name: "Hephaestus".to_string(),
                skills: vec![
                    "tensor_forge".to_string(),
                    "thought_kernel".to_string(),
                    "build".to_string(),
                    "compile".to_string(),
                    "manufacturing".to_string(),
                ],
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 10.0,
            },
            Specialist {
                id: "specialist_argus".to_string(),
                name: "Argus".to_string(),
                skills: vec![
                    "nat_bridge".to_string(),
                    "sensor_node".to_string(),
                    "security".to_string(),
                    "audit".to_string(),
                    "review".to_string(),
                ],
                capacity: 1.0,
                success_rate: 0.95,
                avg_completion_time: 4.0,
            },
        ];

        Self {
            transducer: LinguisticTransducer::new(),
            router: TaskRoutingEngine::new(specialists),
        }
    }

    /// Parse natural language intent into a structured ParsedIntent
    pub fn parse_intent(&self, text: &str) -> ParsedIntent {
        let cas_command = self.transducer.parse_intent(text);
        let extracted_skills = self.extract_skills(text, &cas_command);
        let complexity = self.estimate_complexity(text);
        let urgency = self.estimate_urgency(text);

        ParsedIntent {
            raw_text: text.to_string(),
            cas_command,
            extracted_skills,
            complexity,
            urgency,
        }
    }

    /// Dispatch a parsed intent to the optimal specialist
    pub fn dispatch(&mut self, intent: &ParsedIntent) -> DispatchResult {
        let task_id = format!("task_{}", uuid::Uuid::new_v4());

        let task = RoutableTask {
            id: task_id.clone(),
            task_type: self.cas_to_task_type(&intent.cas_command),
            complexity: intent.complexity,
            urgency: intent.urgency,
            required_skills: intent.extracted_skills.clone(),
            estimated_cost: intent.complexity * 0.3,
        };

        let routing = self.router.find_optimal_specialist(&task);

        // Consume capacity
        self.router
            .consume_capacity(&routing.specialist_id, task.estimated_cost);

        DispatchResult {
            intent: intent.clone(),
            routing,
            task_id,
        }
    }

    /// Parse and dispatch in one call
    pub fn parse_and_dispatch(&mut self, text: &str) -> Result<DispatchResult> {
        let intent = self.parse_intent(text);
        Ok(self.dispatch(&intent))
    }

    /// Record task outcome for learning
    pub fn record_outcome(&mut self, specialist_id: &str, success: bool, completion_time: f64) {
        self.router
            .update_specialist_performance(specialist_id, success, completion_time);
    }

    /// Extract relevant skills from text based on CAS command domain
    fn extract_skills(&self, text: &str, cmd: &CasCommand) -> Vec<String> {
        let mut skills = Vec::new();

        // Domain-based skills
        match cmd.domain.as_str() {
            "knowledge" => {
                skills.push("analysis".to_string());
                skills.push("knowledge".to_string());
            }
            "manufacturing" => {
                skills.push("build".to_string());
                skills.push("compile".to_string());
            }
            "security" => {
                skills.push("security".to_string());
                skills.push("audit".to_string());
            }
            "user_interface" => {
                skills.push("ui".to_string());
                skills.push("visual".to_string());
            }
            "experience" => {
                skills.push("experience".to_string());
                skills.push("memory".to_string());
            }
            "leadership" => {
                skills.push("leadership".to_string());
                skills.push("orchestration".to_string());
            }
            _ => {}
        }

        // Keyword-based skill extraction
        let lower = text.to_lowercase();
        if lower.contains("rust") || lower.contains("cargo") {
            skills.push("rust".to_string());
        }
        if lower.contains("python") || lower.contains("pip") {
            skills.push("python".to_string());
        }
        if lower.contains("test") || lower.contains("spec") {
            skills.push("testing".to_string());
        }
        if lower.contains("debug") || lower.contains("fix") || lower.contains("bug") {
            skills.push("debugging".to_string());
        }
        if lower.contains("refactor") || lower.contains("clean") {
            skills.push("refactoring".to_string());
        }
        if lower.contains("document") || lower.contains("readme") {
            skills.push("documentation".to_string());
        }

        // Deduplicate
        skills.sort();
        skills.dedup();
        skills
    }

    /// Estimate task complexity from text heuristics
    fn estimate_complexity(&self, text: &str) -> f64 {
        let word_count = text.split_whitespace().count() as f64;
        let has_code_terms = text.to_lowercase().contains("impl")
            || text.to_lowercase().contains("fn ")
            || text.to_lowercase().contains("struct");
        let has_multi_step = text.to_lowercase().contains("and then")
            || text.to_lowercase().contains("step")
            || text.to_lowercase().contains("first");

        let base = (word_count / 100.0).min(0.5);
        let code_bonus = if has_code_terms { 0.2 } else { 0.0 };
        let step_bonus = if has_multi_step { 0.15 } else { 0.0 };

        (base + code_bonus + step_bonus).clamp(0.1, 1.0)
    }

    /// Estimate urgency from text heuristics
    fn estimate_urgency(&self, text: &str) -> f64 {
        let lower = text.to_lowercase();
        let mut urgency = 0.3; // baseline

        if lower.contains("urgent") || lower.contains("asap") || lower.contains("immediately") {
            urgency = 0.9;
        } else if lower.contains("soon") || lower.contains("quickly") || lower.contains("fast") {
            urgency = 0.7;
        } else if lower.contains("when you can") || lower.contains("low priority") {
            urgency = 0.2;
        }

        urgency
    }

    /// Map CAS command to TaskType
    fn cas_to_task_type(&self, cmd: &CasCommand) -> TaskType {
        match cmd.mnemonic.as_str() {
            "GENERATE" | "BUILD" | "DEPLOY" => TaskType::CodeGeneration,
            "REFINE" => TaskType::Refactor,
            "REVIEW" | "DEFEND" | "TEST" => TaskType::BugFix,
            "ANALYZE" | "RECALL" | "SEARCH" => TaskType::Analysis,
            "REMEMBER" => TaskType::Ingestion,
            "VISUALIZE" => TaskType::Documentation,
            _ => TaskType::Custom(cmd.mnemonic.clone()),
        }
    }

    /// Access the transducer
    pub fn transducer(&self) -> &LinguisticTransducer {
        &self.transducer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_intent_analyze() {
        let engine = IntentEngine::new();
        let intent = engine.parse_intent("Analyze the authentication module");
        assert_eq!(intent.cas_command.mnemonic, "ANALYZE");
        assert!(intent.extracted_skills.contains(&"analysis".to_string()));
        assert!(intent.complexity > 0.0);
    }

    #[test]
    fn test_parse_intent_generate() {
        let engine = IntentEngine::new();
        let intent = engine.parse_intent("Create a new Rust function for sorting");
        assert_eq!(intent.cas_command.mnemonic, "GENERATE");
        assert!(intent.extracted_skills.contains(&"rust".to_string()));
    }

    #[test]
    fn test_parse_intent_urgent() {
        let engine = IntentEngine::new();
        let intent = engine.parse_intent("Fix this bug ASAP");
        assert!(intent.urgency >= 0.8);
    }

    #[test]
    fn test_dispatch() {
        let mut engine = IntentEngine::new();
        let result = engine.parse_and_dispatch("Review the security of this module").unwrap();
        assert!(!result.task_id.is_empty());
        assert!(!result.routing.specialist_id.is_empty());
        assert!(result.routing.confidence > 0.0);
    }

    #[test]
    fn test_complexity_estimation() {
        let engine = IntentEngine::new();
        let simple = engine.estimate_complexity("Fix typo");
        let complex = engine.estimate_complexity(
            "First implement the trait, then refactor the struct, and finally test the integration with the database module",
        );
        assert!(complex > simple);
    }

    #[test]
    fn test_skill_extraction_rust() {
        let engine = IntentEngine::new();
        let cmd = CasCommand {
            opcode: 0x03,
            mnemonic: "GENERATE".to_string(),
            description: "Generate".to_string(),
            domain: "manufacturing".to_string(),
        };
        let skills = engine.extract_skills("Create a Rust function", &cmd);
        assert!(skills.contains(&"rust".to_string()));
        assert!(skills.contains(&"build".to_string()));
    }
}
