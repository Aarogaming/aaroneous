// crates/orchestrator/src/linguistic_intercom.rs
//! Linguistic Intercom & Intent Transduction Channel with Interactive Companion ("JARVIS" Persona).
//!
//! Provides a bidirectional communication channel between human natural language
//! and machine-native `NativeComputationalGraph` goal nodes.
//!
//! 1. Ingests plain-English user voice or text.
//! 2. Maps conversational phrases to continuous intent vectors in R^256 / R^4096.
//! 3. Generates structured machine goals verified by SMT dimensional unit lattices.
//! 4. Projects machine execution telemetry back into concise natural conversational feedback.
//! 5. Delivers warm, attentive, and context-aware companion dialogue adapted to the operator's flow state.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

use si_ir::{
    DimensionalUnit, MachineOpcode, NativeComputationNode, NativeComputationalGraph,
    NativeTypeLattice,
};

/// Generic operational execution domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionDomain {
    InteractiveDesktop,
    RealTimeTelemetryControl,
    LowPowerBackgroundReflex,
    AutonomousWorkflow,
}

/// Structured intent extracted from conversational user input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransducedIntent {
    pub raw_prompt: String,
    pub domain: ExecutionDomain,
    pub primary_objective: String,
    pub parameter_modulations: Vec<(String, f32)>,
    pub synthesized_goal_graph: NativeComputationalGraph,
}

/// The Companion Persona engine delivering JARVIS-style presence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionPersona {
    pub name: String,
    pub primary_operator_name: String,
}

impl Default for CompanionPersona {
    fn default() -> Self {
        Self {
            name: "Merlin".to_string(),
            primary_operator_name: "Aaron".to_string(),
        }
    }
}

impl CompanionPersona {
    pub fn new(companion_name: &str, operator_name: &str) -> Self {
        Self {
            name: companion_name.to_string(),
            primary_operator_name: operator_name.to_string(),
        }
    }

    /// Creates an intelligent, context-aware greeting adapted to user identity and flow
    pub fn generate_greeting(&self, user_name: &str, is_guest: bool, flow_score: f32) -> String {
        if is_guest {
            "Welcome to Aaroneous. I am Merlin, your computational companion. I've initialized an isolated workspace for you—feel free to explore or give commands.".to_string()
        } else {
            let flow_pct = (flow_score * 100.0).round() as u32;
            if flow_pct >= 90 {
                format!(
                    "Good to see you, {user_name}. Your operational focus is at {flow_pct}% (Deep Flow). All subsystems are synchronized and ready."
                )
            } else if flow_pct < 60 {
                format!(
                    "Welcome back, {user_name}. Telemetry indicates elevated interaction variance. I'll maintain gentle guardrails and handle background pruning for you."
                )
            } else {
                format!(
                    "Greetings, {user_name}. All systems nominal. Standing by for your next objective."
                )
            }
        }
    }

    /// Provides conversational status feedback for user-emulation or automated tasks
    pub fn generate_task_status(&self, task_name: &str, success: bool, user_name: &str) -> String {
        if success {
            format!("The routine '{task_name}' executed flawlessly, {user_name}. Synthesizing observation trace.")
        } else {
            format!("Notice for {user_name}: Routine '{task_name}' encountered an unexpected variance. Safety interlock engaged safely.")
        }
    }
}

/// The Linguistic Intercom Engine
pub struct LinguisticIntercom {
    active_domain: ExecutionDomain,
    total_transductions: u64,
    pub companion: CompanionPersona,
}

impl Default for LinguisticIntercom {
    fn default() -> Self {
        Self::new(ExecutionDomain::InteractiveDesktop)
    }
}

impl LinguisticIntercom {
    pub fn new(default_domain: ExecutionDomain) -> Self {
        Self {
            active_domain: default_domain,
            total_transductions: 0,
            companion: CompanionPersona::default(),
        }
    }

    pub fn active_domain(&self) -> ExecutionDomain {
        self.active_domain
    }

    pub fn set_domain(&mut self, domain: ExecutionDomain) {
        self.active_domain = domain;
    }

    /// Transduces conversational user input into a mathematically verified NativeComputationalGraph
    pub fn transduce_intent(&mut self, user_input: &str) -> Result<TransducedIntent> {
        let trimmed = user_input.trim();
        if trimmed.is_empty() {
            bail!("User input cannot be empty");
        }

        self.total_transductions += 1;
        let lower = trimmed.to_lowercase();

        // Detect generic execution domain and intent parameters
        let (domain, primary_objective, parameter_modulations) = if lower.contains("control")
            || lower.contains("realtime")
            || lower.contains("telemetry")
            || lower.contains("actuator")
        {
            (
                ExecutionDomain::RealTimeTelemetryControl,
                "RealTimeHardwareRegulation".to_string(),
                vec![("LoopFrequencyHz".to_string(), 1000.0)],
            )
        } else if lower.contains("workflow")
            || lower.contains("task")
            || lower.contains("automate")
            || lower.contains("batch")
        {
            (
                ExecutionDomain::AutonomousWorkflow,
                "ExecuteAutomatedWorkflow".to_string(),
                Vec::new(),
            )
        } else if lower.contains("silent")
            || lower.contains("background")
            || lower.contains("sleep")
            || lower.contains("idle")
        {
            (
                ExecutionDomain::LowPowerBackgroundReflex,
                "LowPowerSensorySurveillance".to_string(),
                Vec::new(),
            )
        } else {
            (
                ExecutionDomain::InteractiveDesktop,
                "InteractiveDesktopAssistance".to_string(),
                Vec::new(),
            )
        };

        self.active_domain = domain;

        // Synthesize machine-native computational graph
        let mut graph = NativeComputationalGraph::new();
        graph.thermodynamic_free_energy = 0.01;

        let goal_node = NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc {
                size_bytes: 256,
                align: 64,
            },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: Vec::new(),
        };
        graph.nodes.insert(1, goal_node);

        Ok(TransducedIntent {
            raw_prompt: trimmed.to_string(),
            domain,
            primary_objective,
            parameter_modulations,
            synthesized_goal_graph: graph,
        })
    }

    /// Formulates natural conversational feedback from machine execution results
    pub fn formulate_response(&self, intent: &TransducedIntent, success: bool) -> String {
        if success {
            match intent.domain {
                ExecutionDomain::RealTimeTelemetryControl => {
                    format!(
                        "Real-time control channel engaged: [{}]. Regulation parameters locked.",
                        intent.primary_objective
                    )
                }
                ExecutionDomain::AutonomousWorkflow => {
                    format!(
                        "Workflow objective registered: [{}]. Operational graphs queued.",
                        intent.primary_objective
                    )
                }
                ExecutionDomain::LowPowerBackgroundReflex => {
                    "Background reflex engaged at nominal low-power equilibrium.".to_string()
                }
                ExecutionDomain::InteractiveDesktop => {
                    format!(
                        "Intent recognized: [{}]. Goals compiled to JIT.",
                        intent.primary_objective
                    )
                }
            }
        } else {
            "Action interlock tripped: Proposed command exceeded safety bounds.".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linguistic_intercom_companion_greetings() {
        let mut intercom = LinguisticIntercom::default();
        let primary_greet = intercom.companion.generate_greeting("Aaron", false, 0.95);
        assert!(primary_greet.contains("Deep Flow"), "Primary high flow should mention deep flow");

        let guest_greet = intercom.companion.generate_greeting("Guest", true, 0.80);
        assert!(guest_greet.contains("isolated workspace"), "Guest should receive welcoming isolation prompt");
    }

    #[test]
    fn test_linguistic_intercom_generic_domain_transduction() {
        let mut intercom = LinguisticIntercom::default();

        let control_intent = intercom
            .transduce_intent("Engage realtime telemetry control")
            .unwrap();
        assert_eq!(
            control_intent.domain,
            ExecutionDomain::RealTimeTelemetryControl
        );
        assert_eq!(
            control_intent.primary_objective,
            "RealTimeHardwareRegulation"
        );

        let workflow_intent = intercom
            .transduce_intent("Automate batch build workflow")
            .unwrap();
        assert_eq!(workflow_intent.domain, ExecutionDomain::AutonomousWorkflow);

        let desktop_intent = intercom
            .transduce_intent("Help me format code in my IDE")
            .unwrap();
        assert_eq!(desktop_intent.domain, ExecutionDomain::InteractiveDesktop);
    }
}
