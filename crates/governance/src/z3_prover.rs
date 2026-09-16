//! crates/governance/src/z3_prover.rs
//! Formal Verification & SMT Non-Interference Prover Gate for SI Graphs.
//! Evaluates whether two concurrent computational graphs (`NativeComputationalGraph`)
//! have disjoint write/read footprints and can safely execute in parallel or be merged
//! without semantic corruption or race conditions.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::smt_action_interlock::GovernanceError;
use si_ir::{MachineOpcode, NativeComputationalGraph};

/// Maximum allowable hardware register address before triggering memory safety violation
pub const MAX_HARDWARE_REGISTER: u16 = 8192;

/// Verification report output by the SMT Non-Interference Prover
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonInterferenceReport {
    pub is_non_interfering: bool,
    pub shared_read_registers: Vec<u16>,
    pub conflicting_write_registers: Vec<u16>,
    pub dimensional_consistency_verified: bool,
    pub solver_backend: String,
}

/// SMT / Formal Non-Interference Prover
pub struct Z3Prover {
    #[allow(dead_code)]
    timeout_ms: u32,
}

impl Default for Z3Prover {
    fn default() -> Self {
        Self::new()
    }
}

impl Z3Prover {
    pub fn new() -> Self {
        Self { timeout_ms: 1000 }
    }

    pub fn with_timeout_ms(timeout_ms: u32) -> Self {
        Self { timeout_ms }
    }

    /// Mathematically proves single-graph safety invariants:
    /// 1. Physical dimensional unit coherence (7-exponent SI lattice)
    /// 2. Bounded register footprints preventing hardware memory buffer overruns
    pub fn prove_action_safety(
        &self,
        graph: &NativeComputationalGraph,
    ) -> Result<bool, GovernanceError> {
        // 1. Verify physical dimensional unit invariants
        if let Err(e) = graph.verify_dimensional_invariants() {
            return Err(GovernanceError::LatticeViolation(e.to_string()));
        }

        for node in graph.nodes.values() {
            if let MachineOpcode::TensorDot { left_reg, .. } = node.opcode
                && left_reg > MAX_HARDWARE_REGISTER - 1000
            {
                return Err(GovernanceError::MemorySafetyViolation { register: left_reg });
            }
        }
        // 2. Verify register boundaries
        let (reads, writes) = self.extract_register_footprint(graph);
        for &r in &reads {
            if r > MAX_HARDWARE_REGISTER {
                return Err(GovernanceError::MemorySafetyViolation { register: r });
            }
        }
        for &w in &writes {
            if w > MAX_HARDWARE_REGISTER {
                return Err(GovernanceError::MemorySafetyViolation { register: w });
            }
        }

        Ok(true)
    }

    /// Strict non-interference check returning Result<(), GovernanceError>
    pub fn prove_strict_non_interference(
        &self,
        graph_a: &NativeComputationalGraph,
        graph_b: &NativeComputationalGraph,
    ) -> Result<(), GovernanceError> {
        self.prove_action_safety(graph_a)?;
        self.prove_action_safety(graph_b)?;

        let report = self
            .verify_non_interference(graph_a, graph_b)
            .map_err(|e| GovernanceError::ValidationError(e.to_string()))?;

        if !report.is_non_interfering {
            return Err(GovernanceError::NonInterferenceConflict {
                conflicting_registers: report.conflicting_write_registers,
            });
        }

        Ok(())
    }

    /// Proves non-interference between two computational sub-graphs
    pub fn prove_non_interference(
        &self,
        graph_a: &NativeComputationalGraph,
        graph_b: &NativeComputationalGraph,
    ) -> Result<bool> {
        let report = self.verify_non_interference(graph_a, graph_b)?;
        Ok(report.is_non_interfering)
    }

    /// Detailed verification analysis producing full non-interference diagnostics
    pub fn verify_non_interference(
        &self,
        graph_a: &NativeComputationalGraph,
        graph_b: &NativeComputationalGraph,
    ) -> Result<NonInterferenceReport> {
        // 1. Verify physical dimensional unit invariants on both graphs
        graph_a
            .verify_dimensional_invariants()
            .map_err(|e| anyhow!("Graph A dimensional invariant violation: {e}"))?;
        graph_b
            .verify_dimensional_invariants()
            .map_err(|e| anyhow!("Graph B dimensional invariant violation: {e}"))?;

        // 2. Extract Read/Write Register Footprints
        let (reads_a, writes_a) = self.extract_register_footprint(graph_a);
        let (reads_b, writes_b) = self.extract_register_footprint(graph_b);

        // 3. SMT / Semantic Non-Interference Check
        // Condition: (Writes_A ∩ (Reads_B ∪ Writes_B) = ∅) ∧ (Writes_B ∩ Reads_A = ∅)
        let mut conflicting_writes = Vec::new();

        for &w_a in &writes_a {
            if writes_b.contains(&w_a) || reads_b.contains(&w_a) {
                conflicting_writes.push(w_a);
            }
        }

        for &w_b in &writes_b {
            if reads_a.contains(&w_b) && !conflicting_writes.contains(&w_b) {
                conflicting_writes.push(w_b);
            }
        }

        let mut shared_reads = Vec::new();
        for &r_a in &reads_a {
            if reads_b.contains(&r_a) {
                shared_reads.push(r_a);
            }
        }

        let is_non_interfering = conflicting_writes.is_empty();

        let backend = "PureRust-Semantic-Lattice".to_string();

        Ok(NonInterferenceReport {
            is_non_interfering,
            shared_read_registers: shared_reads,
            conflicting_write_registers: conflicting_writes,
            dimensional_consistency_verified: true,
            solver_backend: backend,
        })
    }

    fn extract_register_footprint(
        &self,
        graph: &NativeComputationalGraph,
    ) -> (HashSet<u16>, HashSet<u16>) {
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();

        for node in graph.nodes.values() {
            match &node.opcode {
                MachineOpcode::Load { address_reg } => {
                    reads.insert(*address_reg);
                }
                MachineOpcode::Store {
                    address_reg,
                    value_reg,
                } => {
                    writes.insert(*address_reg);
                    reads.insert(*value_reg);
                }
                MachineOpcode::BranchIf { condition_reg, .. } => {
                    reads.insert(*condition_reg);
                }
                MachineOpcode::Call { arg_regs, .. } => {
                    for r in arg_regs {
                        reads.insert(*r);
                    }
                }
                MachineOpcode::TensorDot {
                    left_reg,
                    right_reg,
                    ..
                } => {
                    reads.insert(*left_reg);
                    reads.insert(*right_reg);
                    writes.insert(left_reg.saturating_add(1000)); // Destination register
                }
                MachineOpcode::EntropyMinimization { state_reg } => {
                    reads.insert(*state_reg);
                    writes.insert(*state_reg);
                }
                MachineOpcode::Return { value_reg } => {
                    reads.insert(*value_reg);
                }
                MachineOpcode::Alloc { .. } => {}
            }
        }

        (reads, writes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_ir::{DimensionalUnit, NativeComputationNode, NativeTypeLattice};

    #[test]
    fn test_z3_prover_disjoint_graphs_non_interference() {
        let prover = Z3Prover::new();

        let mut graph_a = NativeComputationalGraph::new();
        graph_a.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Load { address_reg: 10 },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });

        let mut graph_b = NativeComputationalGraph::new();
        graph_b.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Load { address_reg: 20 },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });

        let result = prover.prove_non_interference(&graph_a, &graph_b).unwrap();
        assert!(result, "Disjoint graphs should prove non-interfering");
    }

    #[test]
    fn test_z3_prover_detects_write_conflict() {
        let prover = Z3Prover::new();

        let mut graph_a = NativeComputationalGraph::new();
        graph_a.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Store {
                address_reg: 10,
                value_reg: 5,
            },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });

        let mut graph_b = NativeComputationalGraph::new();
        graph_b.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Store {
                address_reg: 10,
                value_reg: 6,
            }, // Conflict on write reg 10
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });

        let report = prover.verify_non_interference(&graph_a, &graph_b).unwrap();
        assert!(!report.is_non_interfering);
        assert_eq!(report.conflicting_write_registers, vec![10]);
    }

    #[test]
    fn test_z3_prover_memory_safety_bounds() {
        let prover = Z3Prover::new();

        let mut safe_graph = NativeComputationalGraph::new();
        safe_graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Load { address_reg: 100 },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });
        assert!(prover.prove_action_safety(&safe_graph).is_ok());

        // Out-of-bounds register > MAX_HARDWARE_REGISTER
        let mut unsafe_graph = NativeComputationalGraph::new();
        unsafe_graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Load { address_reg: 9000 },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });
        let err = prover.prove_action_safety(&unsafe_graph).unwrap_err();
        assert_eq!(
            err,
            GovernanceError::MemorySafetyViolation { register: 9000 }
        );
    }

    #[test]
    fn test_z3_prover_strict_non_interference() {
        let prover = Z3Prover::new();

        let mut graph_a = NativeComputationalGraph::new();
        graph_a.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Store {
                address_reg: 10,
                value_reg: 5,
            },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });

        let mut graph_b = NativeComputationalGraph::new();
        graph_b.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Store {
                address_reg: 10,
                value_reg: 6,
            },
            type_lattice: NativeTypeLattice::PhysicalQuantity {
                unit: DimensionalUnit::DIMENSIONLESS,
                precision: 32,
            },
            energy_cost: 0.001,
            dependencies: vec![],
        });

        let result = prover.prove_strict_non_interference(&graph_a, &graph_b);
        assert!(result.is_err());
        match result.unwrap_err() {
            GovernanceError::NonInterferenceConflict {
                conflicting_registers,
            } => {
                assert_eq!(conflicting_registers, vec![10]);
            }
            other => panic!("Expected NonInterferenceConflict, got {:?}", other),
        }
    }
}
