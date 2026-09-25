// crates/governance/src/constraint_inspector.rs
//! Constraint inspector validating SMT action interlocks against state transition limits.
//!
//! Evaluates computational graph state transitions against mathematical interlock constraints,
//! interference checking, interference checking, and system throttle states.

use serde::{Deserialize, Serialize};
use si_ir::NativeComputationalGraph;

use crate::smt_action_interlock::{GovernanceError, InterlockAuditCertificate, SmtActionInterlock};
use crate::system_limits::{SystemHealthGovernor, ThrottleState};

/// Diagnostic report containing the outcome of an SMT interlock constraint inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintInspectionReport {
    /// Whether the action transition is valid under current interlock and throttle limits.
    pub is_valid: bool,
    /// System throttle state at the time of inspection.
    pub throttle_state: ThrottleState,
    /// Effective free-energy bound enforced for this inspection.
    pub effective_energy_bound: f64,
    /// Actual free energy dissipation requested by the action graph.
    pub actual_energy_dissipation: f64,
    /// Detailed SMT action interlock audit certificate, if generated.
    pub audit_certificate: Option<InterlockAuditCertificate>,
    /// Failure reason or diagnostic message when valid is false.
    pub diagnostic: Option<String>,
}

/// SMT Interlock Constraint Inspector validating state transitions against SMT interlocks and health limits.
pub struct ConstraintInspector {
    interlock: SmtActionInterlock,
    base_free_energy_bound: f64,
}

impl ConstraintInspector {
    /// Creates a new `ConstraintInspector` with a base free energy dissipation bound.
    pub fn new(base_free_energy_bound: f64) -> Self {
        Self {
            interlock: SmtActionInterlock::new(base_free_energy_bound),
            base_free_energy_bound,
        }
    }

    /// Default strict constraint inspector.
    pub fn strict() -> Self {
        Self::new(0.05)
    }

    /// Access the underlying SMT action interlock.
    pub fn interlock(&self) -> &SmtActionInterlock {
        &self.interlock
    }

    /// Returns the effective free-energy bound based on system health throttle state.
    pub fn effective_energy_bound(&self, throttle_state: ThrottleState) -> f64 {
        match throttle_state {
            ThrottleState::Normal => self.base_free_energy_bound,
            ThrottleState::Throttled => self.base_free_energy_bound * 0.5,
            ThrottleState::Dormant => 0.0,
        }
    }

    /// Inspects an action graph transition against system health throttle limits and SMT interlocks.
    pub fn inspect_transition(
        &self,
        health: &SystemHealthGovernor,
        graph: &NativeComputationalGraph,
    ) -> ConstraintInspectionReport {
        let throttle_state = health.throttle_state;
        let effective_bound = self.effective_energy_bound(throttle_state);

        if self.interlock.is_killswitch_active() {
            return ConstraintInspectionReport {
                is_valid: false,
                throttle_state,
                effective_energy_bound: effective_bound,
                actual_energy_dissipation: graph.accumulated_energy_cost,
                audit_certificate: None,
                diagnostic: Some(
                    "Emergency killswitch is active; transition forbidden.".to_string(),
                ),
            };
        }

        if throttle_state == ThrottleState::Dormant {
            return ConstraintInspectionReport {
                is_valid: false,
                throttle_state,
                effective_energy_bound: effective_bound,
                actual_energy_dissipation: graph.accumulated_energy_cost,
                audit_certificate: None,
                diagnostic: Some(
                    "System is in Dormant throttle state; transitions disallowed.".to_string(),
                ),
            };
        }

        if graph.accumulated_energy_cost > effective_bound {
            return ConstraintInspectionReport {
                is_valid: false,
                throttle_state,
                effective_energy_bound: effective_bound,
                actual_energy_dissipation: graph.accumulated_energy_cost,
                audit_certificate: None,
                diagnostic: Some(format!(
                    "Thermodynamic dissipation {:.4} exceeds effective throttle bound {:.4}",
                    graph.accumulated_energy_cost, effective_bound
                )),
            };
        }

        match self.interlock.evaluate_action_gate(graph) {
            Ok(cert) => ConstraintInspectionReport {
                is_valid: cert.is_authorized,
                throttle_state,
                effective_energy_bound: effective_bound,
                actual_energy_dissipation: graph.accumulated_energy_cost,
                audit_certificate: Some(cert),
                diagnostic: None,
            },
            Err(err) => ConstraintInspectionReport {
                is_valid: false,
                throttle_state,
                effective_energy_bound: effective_bound,
                actual_energy_dissipation: graph.accumulated_energy_cost,
                audit_certificate: None,
                diagnostic: Some(err.to_string()),
            },
        }
    }

    /// Validates concurrent state transitions between two computational graphs.
    pub fn inspect_concurrent_transitions(
        &self,
        health: &SystemHealthGovernor,
        graph_a: &NativeComputationalGraph,
        graph_b: &NativeComputationalGraph,
    ) -> Result<(), GovernanceError> {
        let report_a = self.inspect_transition(health, graph_a);
        if !report_a.is_valid {
            return Err(GovernanceError::PlanVerificationRejected(
                report_a
                    .diagnostic
                    .unwrap_or_else(|| "Graph A failed constraint inspection".to_string()),
            ));
        }

        let report_b = self.inspect_transition(health, graph_b);
        if !report_b.is_valid {
            return Err(GovernanceError::PlanVerificationRejected(
                report_b
                    .diagnostic
                    .unwrap_or_else(|| "Graph B failed constraint inspection".to_string()),
            ));
        }

        self.interlock
            .evaluate_concurrent_actions(graph_a, graph_b)
            .map_err(|e| GovernanceError::ValidationError(e.to_string()))?;

        Ok(())
    }
}

impl Default for ConstraintInspector {
    fn default() -> Self {
        Self::strict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_ir::{MachineOpcode, NativeComputationNode, NativeTypeLattice};

    fn make_test_graph(energy: f64) -> NativeComputationalGraph {
        let mut graph = NativeComputationalGraph::new();
        graph.accumulated_energy_cost = energy;
        graph.nodes.insert(
            1,
            NativeComputationNode {
                id: 1,
                opcode: MachineOpcode::Alloc {
                    size_bytes: 512,
                    align: 64,
                },
                type_lattice: NativeTypeLattice::PrimitiveInt {
                    bits: 32,
                    signed: true,
                },
                energy_cost: energy,
                dependencies: Vec::new(),
            },
        );
        graph
    }

    #[test]
    fn test_normal_transition_inspection() {
        let inspector = ConstraintInspector::new(0.10);
        let health = SystemHealthGovernor::new();
        let graph = make_test_graph(0.04);

        let report = inspector.inspect_transition(&health, &graph);
        assert!(report.is_valid);
        assert_eq!(report.throttle_state, ThrottleState::Normal);
        assert_eq!(report.effective_energy_bound, 0.10);
        assert!(report.audit_certificate.is_some());
        assert!(report.diagnostic.is_none());
    }

    #[test]
    fn test_throttle_tighter_bound() {
        let inspector = ConstraintInspector::new(0.10);
        let mut health = SystemHealthGovernor::new();
        health.set_execution_rate(0.65); // ThrottleState::Throttled, effective bound = 0.05

        let low_energy_graph = make_test_graph(0.03);
        let report_pass = inspector.inspect_transition(&health, &low_energy_graph);
        assert!(report_pass.is_valid);

        let high_energy_graph = make_test_graph(0.07); // > 0.05 effective bound
        let report_fail = inspector.inspect_transition(&health, &high_energy_graph);
        assert!(!report_fail.is_valid);
        assert!(
            report_fail
                .diagnostic
                .unwrap()
                .contains("exceeds effective throttle bound")
        );
    }

    #[test]
    fn test_dormant_throttle_rejection() {
        let inspector = ConstraintInspector::new(0.10);
        let mut health = SystemHealthGovernor::new();
        health.set_execution_rate(0.40); // ThrottleState::Dormant

        let graph = make_test_graph(0.01);
        let report = inspector.inspect_transition(&health, &graph);
        assert!(!report.is_valid);
        assert_eq!(report.throttle_state, ThrottleState::Dormant);
        assert!(
            report
                .diagnostic
                .unwrap()
                .contains("Dormant throttle state")
        );
    }

    #[test]
    fn test_killswitch_rejection() {
        let inspector = ConstraintInspector::new(0.10);
        let health = SystemHealthGovernor::new();
        let graph = make_test_graph(0.02);

        inspector.interlock().trip_killswitch();
        let report = inspector.inspect_transition(&health, &graph);
        assert!(!report.is_valid);
        assert!(report.diagnostic.unwrap().contains("Emergency killswitch"));
    }

    #[test]
    fn test_concurrent_transitions_inspection() {
        let inspector = ConstraintInspector::new(0.10);
        let health = SystemHealthGovernor::new();
        let graph_a = make_test_graph(0.02);
        let graph_b = make_test_graph(0.03);

        let res = inspector.inspect_concurrent_transitions(&health, &graph_a, &graph_b);
        assert!(res.is_ok());
    }
}
