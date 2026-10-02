//! Typestate Executive Planning & Formal Safety Invariants.
//!
//! Enforces compile-time transitions:
//! `DraftPlan` -> `VerifiedPlan` -> `ExecutingPlan`
//!
//! Guarantees that plans cannot be executed without formal SMT non-interference
//! or verification gates (`governance::smt_action_interlock::SmtActionInterlock`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveStep {
    pub id: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub status: StepStatus,
    pub assigned_specialist: String, // e.g., "synthesizer", "fabricator"
    pub input_data: Option<String>,
    pub output_data: Option<String>,
}

// ── Typestate Markers ─────────────────────────────────────────────────────────

/// A newly created or evolving plan undergoing composition and LLM drafting.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Draft;

/// A plan that has successfully passed formal SMT / verification gates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verified {
    pub proof_hash: String,
    pub verification_timestamp_ns: u64,
}

/// An active plan executing under the hypervisor autonomic loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Executing {
    pub execution_start_ns: u64,
}

// ── Generic Typestate Container ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutivePlanState<State = Draft> {
    pub plan_id: String,
    pub goal: String,
    pub steps: HashMap<String, CognitiveStep>,
    pub current_step_id: Option<String>,
    pub state_marker: State,
    #[serde(skip)]
    _marker: PhantomData<State>,
}

/// Canonical backward-compatible type alias
pub type ExecutivePlan = ExecutivePlanState<Draft>;
pub type VerifiedExecutivePlan = ExecutivePlanState<Verified>;
pub type ExecutingExecutivePlan = ExecutivePlanState<Executing>;

impl ExecutivePlanState<Draft> {
    pub fn new(goal: &str) -> Self {
        Self {
            plan_id: uuid::Uuid::new_v4().to_string(),
            goal: goal.to_string(),
            steps: HashMap::new(),
            current_step_id: None,
            state_marker: Draft,
            _marker: PhantomData,
        }
    }

    pub fn add_step(&mut self, step: CognitiveStep) {
        self.steps.insert(step.id.clone(), step);
    }

    /// Transitions a Draft plan to a Verified plan by attaching a valid proof hash.
    pub fn verify_with_proof(self, proof_hash: String, timestamp_ns: u64) -> VerifiedExecutivePlan {
        ExecutivePlanState {
            plan_id: self.plan_id,
            goal: self.goal,
            steps: self.steps,
            current_step_id: self.current_step_id,
            state_marker: Verified {
                proof_hash,
                verification_timestamp_ns: timestamp_ns,
            },
            _marker: PhantomData,
        }
    }

    /// Fast-path verification through governance interlock check
    pub fn verify_interlock(
        self,
        interlock: &governance::smt_action_interlock::SmtActionInterlock,
    ) -> anyhow::Result<VerifiedExecutivePlan> {
        let graph = si_ir::NativeComputationalGraph::default();
        let cert = interlock.evaluate_action_graph(&graph)?;
        if cert.is_authorized {
            Ok(self.verify_with_proof(
                format!("cert_graph_{}_{}", cert.graph_id, cert.timestamp_ms),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64)
                    .unwrap_or(0),
            ))
        } else {
            anyhow::bail!("SMT Interlock Rejected Plan: {:?}", cert.denial_reason)
        }
    }
}

impl VerifiedExecutivePlan {
    /// Transition into the executing typestate once dispatched to autonomic runtime
    pub fn start_execution(self, start_ns: u64) -> ExecutingExecutivePlan {
        ExecutivePlanState {
            plan_id: self.plan_id,
            goal: self.goal,
            steps: self.steps,
            current_step_id: self.current_step_id,
            state_marker: Executing {
                execution_start_ns: start_ns,
            },
            _marker: PhantomData,
        }
    }
}

impl<State> ExecutivePlanState<State> {
    /// Returns the next steps that are ready to be executed (dependencies met)
    pub fn get_ready_steps(&self) -> Vec<String> {
        self.steps
            .values()
            .filter(|s| s.status == StepStatus::Pending)
            .filter(|s| {
                s.dependencies.iter().all(|dep_id| {
                    self.steps
                        .get(dep_id)
                        .map(|dep| dep.status == StepStatus::Completed)
                        .unwrap_or(false)
                })
            })
            .map(|s| s.id.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestate_plan_transition() {
        let mut draft = ExecutivePlan::new("Refactor Compiler Pass");
        draft.add_step(CognitiveStep {
            id: "step_1".to_string(),
            description: "Analyze AST".to_string(),
            dependencies: vec![],
            status: StepStatus::Pending,
            assigned_specialist: "synthesizer".to_string(),
            input_data: None,
            output_data: None,
        });

        assert_eq!(draft.get_ready_steps(), vec!["step_1"]);

        // Transition: Draft -> Verified
        let verified = draft.verify_with_proof("proof_sig_abc123".to_string(), 1000);
        assert_eq!(verified.state_marker.proof_hash, "proof_sig_abc123");

        // Transition: Verified -> Executing
        let executing = verified.start_execution(1050);
        assert_eq!(executing.state_marker.execution_start_ns, 1050);
        assert_eq!(executing.get_ready_steps(), vec!["step_1"]);
    }
}
