use crate::executive_plan::{CognitiveStep, ExecutivePlan, StepStatus};
use anyhow::Result;

pub struct PrefrontalCortex;

/// Architectural Terminology Standard: ExecutivePlanner
pub type ExecutivePlanner = PrefrontalCortex;
pub type StrategicPlanner = PrefrontalCortex;

impl PrefrontalCortex {
    /// Generates a multi-step execution plan based on a high-level intent.
    pub async fn draft_plan(&self, intent: &str) -> Result<ExecutivePlan> {
        println!("[PrefrontalCortex] Drafting plan for: {}", intent);

        let mut plan = ExecutivePlan::new(intent);
        let tokens: Vec<&str> = intent.split_whitespace().collect();

        // Heuristic-based planning (replacing the hardcoded 'research' block)
        if tokens
            .iter()
            .any(|&t| t.contains("research") || t.contains("find") || t.contains("search"))
        {
            plan.add_step(CognitiveStep {
                id: "research_phase".to_string(),
                description: format!("Perform deep research into: {}", intent),
                dependencies: vec![],
                status: StepStatus::Pending,
                assigned_specialist: "synthesizer".to_string(),
                input_data: Some(intent.to_string()),
                output_data: None,
            });
        }

        if tokens.iter().any(|&t| {
            t.contains("implement")
                || t.contains("build")
                || t.contains("write")
                || t.contains("create")
        }) {
            let deps = if plan.steps.contains_key("research_phase") {
                vec!["research_phase".to_string()]
            } else {
                vec![]
            };
            plan.add_step(CognitiveStep {
                id: "implementation_phase".to_string(),
                description: "Execute technical implementation based on research or direct intent"
                    .to_string(),
                dependencies: deps,
                status: StepStatus::Pending,
                assigned_specialist: "fabricator".to_string(),
                input_data: None,
                output_data: None,
            });
        }

        if plan.steps.is_empty() {
            // Default single-step for unknown intents
            plan.add_step(CognitiveStep {
                id: "general_execution".to_string(),
                description: "Direct execution of intent".to_string(),
                dependencies: vec![],
                status: StepStatus::Pending,
                assigned_specialist: "orchestrator".to_string(),
                input_data: Some(intent.to_string()),
                output_data: None,
            });
        }

        Ok(plan)
    }

    /// Critiques an existing plan and refines it (Plan-and-Solve)
    pub async fn critique_plan(&self, plan: &mut ExecutivePlan) -> Result<()> {
        println!("[PrefrontalCortex] Critiquing plan: {}", plan.plan_id);
        // Logic to verify if dependencies are logical or if steps are missing
        Ok(())
    }
}
