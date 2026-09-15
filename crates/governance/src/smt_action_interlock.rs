// crates/governance/src/smt_action_interlock.rs
//! Structural and resource-bound pre-execution action interlock.
//!
//! Provides source-level checks for:
//! 1. Register-footprint interference across graph pairs (separate evaluation).
//! 2. Strict 7-exponent SI dimensional lattice unit checks ([M, L, T, I, Theta, N, J]).
//! 3. Thermodynamic free-energy dissipation bounds (Delta F <= epsilon).
//! 4. Hardware and spatial perimeter containment bounds.
//!
//! If any verification check fails, execution is aborted and the hardware interlock triggers.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;

use crate::lattice_verifier::{LatticeVerifier, VerificationReport};
use crate::z3_prover::{NonInterferenceReport, Z3Prover};
use si_ir::NativeComputationalGraph;

/// Structured errors emitted during formal SMT verification, thermodynamic gating, and interlock checks.
#[derive(Debug, Error, Clone, PartialEq)]
pub enum GovernanceError {
    #[error("Hardware emergency killswitch is active: {0}")]
    KillswitchTripped(String),

    #[error("Thermodynamic free-energy dissipation {actual:.4} exceeds strict bound {max:.4}")]
    ThermodynamicBoundExceeded { actual: f64, max: f64 },

    #[error("Structural lattice dimensional invariant violation: {0}")]
    LatticeViolation(String),

    #[error("SMT formal non-interference conflict on registers: {conflicting_registers:?}")]
    NonInterferenceConflict { conflicting_registers: Vec<u16> },

    #[error("Algebraic invariant violation on state '{state_name}': {reason}")]
    AlgebraicInvariantViolation { state_name: String, reason: String },

    #[error("Memory or register boundary violation: register {register} exceeds valid space")]
    MemorySafetyViolation { register: u16 },

    #[error("Executive plan interlock verification rejected: {0}")]
    PlanVerificationRejected(String),

    #[error("General governance validation error: {0}")]
    ValidationError(String),
}

/// The execution outcome and audit proof produced by the interlock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterlockAuditCertificate {
    pub is_authorized: bool,
    pub graph_id: u64,
    pub timestamp_ms: u64,
    pub free_energy_dissipation: f64,
    pub lattice_report: VerificationReport,
    /// False for single-graph checks; pairwise analysis has its own report.
    pub smt_non_interference_verified: bool,
    pub denial_reason: Option<String>,
}

/// SmtActionInterlock: The hardware-gated mathematical fence
pub struct SmtActionInterlock {
    lattice_verifier: LatticeVerifier,
    z3_prover: Z3Prover,
    max_free_energy_bound: f64,
    emergency_killswitch_tripped: AtomicBool,
    interlock_eval_counter: AtomicU64,
}

impl SmtActionInterlock {
    /// Creates a new mathematical action interlock with strict bounds.
    pub fn new(max_free_energy_bound: f64) -> Self {
        Self {
            lattice_verifier: LatticeVerifier::default().with_epsilon(max_free_energy_bound),
            z3_prover: Z3Prover::new(),
            max_free_energy_bound,
            emergency_killswitch_tripped: AtomicBool::new(false),
            interlock_eval_counter: AtomicU64::new(1),
        }
    }

    /// Access the Z3 SMT prover
    pub fn z3_prover(&self) -> &Z3Prover {
        &self.z3_prover
    }

    /// Default strict configuration (max free energy = 0.05).
    pub fn strict() -> Self {
        Self::new(0.05)
    }

    /// Checks one graph with typed errors. Pairwise interference is checked separately.
    pub fn evaluate_action_gate(
        &self,
        graph: &NativeComputationalGraph,
    ) -> Result<InterlockAuditCertificate, GovernanceError> {
        if self.emergency_killswitch_tripped.load(Ordering::Acquire) {
            return Err(GovernanceError::KillswitchTripped(
                "Execution forbidden by active emergency killswitch".to_string(),
            ));
        }

        // Prove memory bounds and dimensional consistency via Z3Prover
        self.z3_prover.prove_action_safety(graph)?;

        // Prove thermodynamic dissipation bound
        if graph.thermodynamic_free_energy > self.max_free_energy_bound {
            return Err(GovernanceError::ThermodynamicBoundExceeded {
                actual: graph.thermodynamic_free_energy,
                max: self.max_free_energy_bound,
            });
        }

        let cert = self
            .evaluate_action_graph(graph)
            .map_err(|e| GovernanceError::ValidationError(e.to_string()))?;
        if !cert.is_authorized {
            return Err(GovernanceError::ValidationError(
                cert.denial_reason
                    .clone()
                    .unwrap_or_else(|| "Interlock authorization denied".to_string()),
            ));
        }

        Ok(cert)
    }

    /// Runs structural and resource checks for one graph before dispatch.
    pub fn evaluate_action_graph(
        &self,
        graph: &NativeComputationalGraph,
    ) -> Result<InterlockAuditCertificate> {
        let eval_id = self.interlock_eval_counter.fetch_add(1, Ordering::Relaxed);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // 1. Check physical hardware emergency killswitch
        if self.emergency_killswitch_tripped.load(Ordering::Acquire) {
            bail!("Hardware interlock tripped: Execution forbidden by active emergency killswitch");
        }

        // Every graph is checked in full. Node count and rounded energy do not
        // identify graph content and must never be used to reuse authorization.
        if !self.max_free_energy_bound.is_finite() || self.max_free_energy_bound < 0.0 {
            bail!("Invalid non-finite or negative energy policy bound");
        }
        self.z3_prover.prove_action_safety(graph)?;

        // 2. Perform Structural Lattice and 7-Exponent SI Dimensional Verification
        let lattice_report = match self.lattice_verifier.verify(graph) {
            Ok(rep) => rep,
            Err(e) => {
                return Ok(InterlockAuditCertificate {
                    is_authorized: false,
                    graph_id: eval_id,
                    timestamp_ms: ts,
                    free_energy_dissipation: graph.thermodynamic_free_energy,
                    lattice_report: VerificationReport {
                        is_valid: false,
                        total_nodes: graph.nodes.len(),
                        free_energy: graph.thermodynamic_free_energy,
                        dimensional_checks_passed: 0,
                        spatial_checks_passed: 0,
                        diagnostics: vec![format!("Lattice verification rejected: {e}")],
                    },
                    smt_non_interference_verified: false,
                    denial_reason: Some(format!("Lattice dimensional or energy violation: {e}")),
                });
            }
        };

        // 3. Thermodynamic Free-Energy Dissipation Bound Gate
        if graph.thermodynamic_free_energy > self.max_free_energy_bound {
            return Ok(InterlockAuditCertificate {
                is_authorized: false,
                graph_id: eval_id,
                timestamp_ms: ts,
                free_energy_dissipation: graph.thermodynamic_free_energy,
                lattice_report,
                smt_non_interference_verified: false,
                denial_reason: Some(format!(
                    "Thermodynamic dissipation {:.4} exceeds strict bound {:.4}",
                    graph.thermodynamic_free_energy, self.max_free_energy_bound
                )),
            });
        }

        Ok(InterlockAuditCertificate {
            is_authorized: true,
            graph_id: eval_id,
            timestamp_ms: ts,
            free_energy_dissipation: graph.thermodynamic_free_energy,
            lattice_report,
            smt_non_interference_verified: false,
            denial_reason: None,
        })
    }

    /// Evaluates concurrent execution of two graphs ensuring non-interference before merge or parallel dispatch.
    pub fn evaluate_concurrent_actions(
        &self,
        graph_a: &NativeComputationalGraph,
        graph_b: &NativeComputationalGraph,
    ) -> Result<NonInterferenceReport> {
        if self.emergency_killswitch_tripped.load(Ordering::Acquire) {
            bail!("Hardware interlock tripped: Execution forbidden by active emergency killswitch");
        }

        // Verify individual graphs first
        let cert_a = self.evaluate_action_graph(graph_a)?;
        if !cert_a.is_authorized {
            bail!("Graph A rejected by interlock: {:?}", cert_a.denial_reason);
        }

        let cert_b = self.evaluate_action_graph(graph_b)?;
        if !cert_b.is_authorized {
            bail!("Graph B rejected by interlock: {:?}", cert_b.denial_reason);
        }

        // Run SMT algebraic non-interference solver
        self.z3_prover.verify_non_interference(graph_a, graph_b)
    }

    /// Evaluates custom algebraic constraints and invariant predicates against state vectors
    pub fn verify_algebraic_invariant<F>(
        &self,
        state_name: &str,
        state_vector: &[f64],
        predicate: F,
    ) -> Result<bool>
    where
        F: Fn(&[f64]) -> bool,
    {
        if self.emergency_killswitch_tripped.load(Ordering::Acquire) {
            bail!("Hardware interlock tripped: Execution forbidden by active emergency killswitch");
        }

        if predicate(state_vector) {
            Ok(true)
        } else {
            bail!(
                "Algebraic invariant violation on state '{}': constraint predicate failed",
                state_name
            );
        }
    }

    /// Manually or automatically trip the hardware emergency killswitch
    pub fn trip_killswitch(&self) {
        self.emergency_killswitch_tripped
            .store(true, Ordering::Release);
    }

    /// Reset emergency killswitch after human supervisor audit
    pub fn reset_killswitch(&self) {
        self.emergency_killswitch_tripped
            .store(false, Ordering::Release);
    }

    /// Query killswitch state
    pub fn is_killswitch_active(&self) -> bool {
        self.emergency_killswitch_tripped.load(Ordering::Acquire)
    }

    /// Evaluates a batch of candidate action graphs, returning certificates for all candidates
    pub fn batch_verify_action_graphs(
        &self,
        graphs: &[&NativeComputationalGraph],
    ) -> Result<Vec<InterlockAuditCertificate>> {
        if self.emergency_killswitch_tripped.load(Ordering::Acquire) {
            bail!("Hardware interlock tripped: Execution forbidden by active emergency killswitch");
        }

        let mut certs = Vec::with_capacity(graphs.len());
        for graph in graphs {
            certs.push(self.evaluate_action_graph(graph)?);
        }
        Ok(certs)
    }
}

impl Default for SmtActionInterlock {
    fn default() -> Self {
        Self::strict()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_ir::{MachineOpcode, NativeComputationNode, NativeTypeLattice};

    #[test]
    fn test_smt_action_interlock_valid_authorization() {
        let interlock = SmtActionInterlock::new(0.10);
        let mut graph = NativeComputationalGraph::new();
        graph.thermodynamic_free_energy = 0.04;

        let node = NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc {
                size_bytes: 1024,
                align: 64,
            },
            type_lattice: NativeTypeLattice::PrimitiveInt {
                bits: 64,
                signed: false,
            },
            energy_cost: 0.001,
            dependencies: Vec::new(),
        };
        graph.nodes.insert(1, node);

        let cert = interlock.evaluate_action_graph(&graph).unwrap();
        assert!(cert.is_authorized);
        assert!(!cert.smt_non_interference_verified);
        assert!(cert.denial_reason.is_none());
    }

    #[test]
    fn test_smt_action_interlock_thermodynamic_rejection() {
        let interlock = SmtActionInterlock::new(0.05);
        let mut graph = NativeComputationalGraph::new();
        graph.thermodynamic_free_energy = 0.08; // Exceeds bound of 0.05

        let cert = interlock.evaluate_action_graph(&graph).unwrap();
        assert!(!cert.is_authorized);
        let reason = cert.denial_reason.unwrap();
        assert!(
            reason.contains("Thermodynamic dissipation exceeded")
                || reason.contains("exceeds strict bound")
        );
    }

    #[test]
    fn equal_size_graphs_are_independently_verified() {
        let interlock = SmtActionInterlock::new(0.10);
        let mut graph = NativeComputationalGraph::new();
        graph.thermodynamic_free_energy = 0.02;

        let node = NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc {
                size_bytes: 1024,
                align: 64,
            },
            type_lattice: NativeTypeLattice::PrimitiveInt {
                bits: 64,
                signed: false,
            },
            energy_cost: 0.001,
            dependencies: Vec::new(),
        };
        graph.nodes.insert(1, node);

        assert!(
            interlock
                .evaluate_action_graph(&graph)
                .unwrap()
                .is_authorized
        );
        graph.nodes.get_mut(&1).unwrap().opcode = MachineOpcode::Alloc {
            size_bytes: 64 * 1024 * 1024 + 1,
            align: 64,
        };
        assert!(
            !interlock
                .evaluate_action_graph(&graph)
                .unwrap()
                .is_authorized
        );
        assert!(interlock.evaluate_action_gate(&graph).is_err());
    }

    #[test]
    fn non_finite_inputs_and_policy_fail_closed() {
        for energy in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
            let mut graph = NativeComputationalGraph::new();
            graph.thermodynamic_free_energy = energy;
            let interlock = SmtActionInterlock::strict();
            assert!(
                !interlock
                    .evaluate_action_graph(&graph)
                    .unwrap()
                    .is_authorized
            );
            assert!(interlock.evaluate_action_gate(&graph).is_err());
            assert!(
                SmtActionInterlock::new(energy)
                    .evaluate_action_graph(&NativeComputationalGraph::new())
                    .is_err()
            );
        }
    }

    #[test]
    fn test_smt_action_interlock_emergency_killswitch() {
        let interlock = SmtActionInterlock::strict();
        let graph = NativeComputationalGraph::new();

        assert!(!interlock.is_killswitch_active());
        interlock.trip_killswitch();
        assert!(interlock.is_killswitch_active());

        // Graph evaluation must error immediately
        assert!(interlock.evaluate_action_graph(&graph).is_err());

        // Reset allows evaluation to proceed
        interlock.reset_killswitch();
        assert!(!interlock.is_killswitch_active());
        assert!(interlock.evaluate_action_graph(&graph).is_ok());
    }

    #[test]
    fn test_algebraic_invariant_evaluation() {
        let interlock = SmtActionInterlock::strict();
        let state = vec![100.0, 50.0, 25.0]; // Positive resource counts
        let valid = interlock.verify_algebraic_invariant("resource_bounds", &state, |s| {
            s.iter().all(|&val| val >= 0.0) && s[0] >= s[1]
        });
        assert!(valid.is_ok());

        let invalid = interlock.verify_algebraic_invariant("negative_budget", &[-1.0, 5.0], |s| {
            s.iter().all(|&val| val >= 0.0)
        });
        assert!(invalid.is_err());
    }

    #[test]
    fn test_batch_verify_action_graphs() {
        let interlock = SmtActionInterlock::new(0.10);
        let mut graph_a = NativeComputationalGraph::new();
        graph_a.thermodynamic_free_energy = 0.02;

        let mut graph_b = NativeComputationalGraph::new();
        graph_b.thermodynamic_free_energy = 0.15; // Exceeds bound

        let results = interlock
            .batch_verify_action_graphs(&[&graph_a, &graph_b])
            .unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].is_authorized);
        assert!(!results[1].is_authorized);
    }

    #[test]
    fn test_evaluate_action_gate_typed_errors() {
        let interlock = SmtActionInterlock::new(0.05);

        // 1. Thermodynamic bound exceeded
        let mut high_energy_graph = NativeComputationalGraph::new();
        high_energy_graph.thermodynamic_free_energy = 0.12;
        let err = interlock
            .evaluate_action_gate(&high_energy_graph)
            .unwrap_err();
        match err {
            GovernanceError::ThermodynamicBoundExceeded { actual, max } => {
                assert!((actual - 0.12).abs() < 1e-6);
                assert!((max - 0.05).abs() < 1e-6);
            }
            other => panic!("Expected ThermodynamicBoundExceeded, got {:?}", other),
        }

        // 2. Killswitch active error
        interlock.trip_killswitch();
        let safe_graph = NativeComputationalGraph::new();
        let err = interlock.evaluate_action_gate(&safe_graph).unwrap_err();
        assert!(matches!(err, GovernanceError::KillswitchTripped(_)));
        interlock.reset_killswitch();

        // 3. Success case
        let cert = interlock.evaluate_action_gate(&safe_graph).unwrap();
        assert!(cert.is_authorized);
    }
}
