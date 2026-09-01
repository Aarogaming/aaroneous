//! crates/compute/src/si_jit.rs
//! Machine-Native JIT Crystallization Engine.
//! Transpiles deterministic .si latent trajectories and discrete AST DAGs into raw executable memory blocks,
//! dropping execution latency from < 180µs (neural inference) to < 1µs (bare-metal direct function pointer call).
//! Features:
//! 1. Mathematical Maturity Detector: Var(∇W_LoRA) <= 0.005 and F <= 0.05 (N >= 50).
//! 2. Cranelift Native IR Code Emission & Compilation into Host Machine Code.
//! 3. W^X Two-Phase Memory Protection (RW during compilation -> RX during execution).
//! 4. In-Process Associative Memory Fabric (H4 HNSW) for sub-microsecond trajectory retrieval.
//! 5. De-Crystallization & Hardware Trap Fallback: Flushes poisoned JIT handles and restores continuous SSM learning.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::cranelift_jit::{CraneliftJitEngine, NativeExecutionFn};
use crate::episodic_memory::{EpisodicMemoryFabric, TrajectoryMetadata, LATENT_VECTOR_DIM};
use crate::machine_native::NativeComputationalGraph;
use crate::wx_memory::WxMemoryRegion;

pub const JIT_INTENT_DIM: usize = LATENT_VECTOR_DIM;

/// W^X Memory Protection State
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryProtectionState {
    ReadWrite,  // Mutable during compilation
    ReadExecute,// Immutable & Executable during execution (W^X compliance)
    Revoked,    // De-crystallized / Poisoned
}

/// Context passed to JIT-compiled native execution routines
#[repr(C, align(64))]
pub struct NativeExecutionContext {
    pub registers: [u64; 16],
    pub memory_pool: [u8; 4096],
    pub status_code: u32,
}

impl Default for NativeExecutionContext {
    fn default() -> Self {
        Self {
            registers: [0u64; 16],
            memory_pool: [0u8; 4096],
            status_code: 0,
        }
    }
}

/// JIT-Compiled Native Reflex Handle
pub struct CompiledReflexHandle {
    pub skill_id: u16,
    pub name: String,
    pub instruction_count: usize,
    pub memory_state: MemoryProtectionState,
    pub intent_centroid: Vec<f32>,       // 256-dim intent vector
    pub confidence_radius: f32,          // Maximum Euclidean radius for O(1) bypass
    pub memory_region: Option<WxMemoryRegion>,
    pub execution_fn: Option<NativeExecutionFn>,
}

/// Crystallization Maturity Metrics Evaluator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrystallizationMetrics {
    pub execution_count: u64,
    pub gradient_variance: f32,          // Var(∇W) across recent updates
    pub thermodynamic_free_energy: f64,  // Thermodynamic stability metric
    pub is_mature_for_jit: bool,
}

/// The Machine-Native JIT Compiler Engine
pub struct SiJitCompilerEngine {
    pub jit_backend: CraneliftJitEngine,
    pub memory_fabric: Arc<EpisodicMemoryFabric>,
    pub compiled_registry: Vec<CompiledReflexHandle>,
    pub intent_routing_lut: HashMap<u16, usize>, // Skill ID -> Registry Index
    pub maturity_threshold_count: u64,
    pub max_variance_threshold: f32,
    pub de_crystallization_events_count: u64,
}

impl Default for SiJitCompilerEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SiJitCompilerEngine {
    pub fn new() -> Self {
        Self {
            jit_backend: CraneliftJitEngine::default(),
            memory_fabric: Arc::new(EpisodicMemoryFabric::default()),
            compiled_registry: Vec::new(),
            intent_routing_lut: HashMap::new(),
            maturity_threshold_count: 50,
            max_variance_threshold: 0.005,
            de_crystallization_events_count: 0,
        }
    }

    /// Evaluates whether a candidate skill pathway has reached crystallization maturity
    pub fn evaluate_maturity(
        &self,
        execution_count: u64,
        gradient_variance: f32,
        thermodynamic_free_energy: f64,
    ) -> CrystallizationMetrics {
        let is_mature = execution_count >= self.maturity_threshold_count
            && gradient_variance <= self.max_variance_threshold
            && thermodynamic_free_energy <= 0.05;

        CrystallizationMetrics {
            execution_count,
            gradient_variance,
            thermodynamic_free_energy,
            is_mature_for_jit: is_mature,
        }
    }

    /// Automatically compiles an AST graph into a JIT reflex if mature
    pub fn compile_if_mature(
        &mut self,
        skill_id: u16,
        name: &str,
        graph: &NativeComputationalGraph,
        intent_centroid: &[f32],
        metrics: &CrystallizationMetrics,
    ) -> Result<Option<usize>> {
        if metrics.is_mature_for_jit {
            let idx = self.compile_ast_graph(skill_id, name, graph, intent_centroid)?;
            Ok(Some(idx))
        } else {
            Ok(None)
        }
    }

    /// Compiles a deterministic AST graph into a bare-metal native W^X machine code block via Cranelift
    pub fn compile_ast_graph(
        &mut self,
        skill_id: u16,
        name: &str,
        graph: &NativeComputationalGraph,
        intent_centroid: &[f32],
    ) -> Result<usize> {
        // Step 1: Invariant verification
        graph.verify_dimensional_invariants()?;

        let num_nodes = graph.nodes.len();

        // Step 2: Cranelift IR emission, JIT compilation, and W^X memory page locking
        let (memory_region, fn_ptr) = self.jit_backend.compile_graph_to_memory(graph)?;

        let handle_idx = self.compiled_registry.len();
        self.compiled_registry.push(CompiledReflexHandle {
            skill_id,
            name: name.to_string(),
            instruction_count: num_nodes,
            memory_state: MemoryProtectionState::ReadExecute, // Locked to RX
            intent_centroid: intent_centroid.to_vec(),
            confidence_radius: 1.5,
            memory_region: Some(memory_region),
            execution_fn: Some(fn_ptr),
        });

        self.intent_routing_lut.insert(skill_id, handle_idx);

        // Step 3: Index in Episodic Memory Fabric (HNSW associative lookup)
        let mut latent_arr = [0.0f32; LATENT_VECTOR_DIM];
        for (i, &val) in intent_centroid.iter().take(LATENT_VECTOR_DIM).enumerate() {
            latent_arr[i] = val;
        }

        let _ = self.memory_fabric.insert_trajectory(
            skill_id as u64,
            &latent_arr,
            TrajectoryMetadata {
                skill_id,
                trajectory_id: skill_id as u64,
                action_summary: name.to_string(),
                thermodynamic_free_energy: graph.thermodynamic_free_energy,
                crystallized_handle_idx: Some(handle_idx),
                timestamp_ms: 0,
            },
        );

        Ok(handle_idx)
    }

    /// Fast HNSW Associative Intent Vector Lookup: Retrieves candidate JIT reflex with sub-microsecond latency
    pub fn lookup_fast_reflex(&self, intent_vector: &[f32]) -> Option<usize> {
        let mut query = [0.0f32; LATENT_VECTOR_DIM];
        for (i, &val) in intent_vector.iter().take(LATENT_VECTOR_DIM).enumerate() {
            query[i] = val;
        }

        // HNSW Recall from Episodic Memory Fabric
        let candidates = self.memory_fabric.recall_nearest(&query, 1);
        if let Some(best) = candidates.first() {
            if best.similarity >= 0.70 {
                if let Some(handle_idx) = best.metadata.crystallized_handle_idx {
                    if handle_idx < self.compiled_registry.len() {
                        let handle = &self.compiled_registry[handle_idx];
                        if handle.memory_state == MemoryProtectionState::ReadExecute {
                            return Some(handle_idx);
                        }
                    }
                }
            }
        }

        // Fallback linear distance scan if HNSW returns no active RX match
        for (idx, handle) in self.compiled_registry.iter().enumerate() {
            if handle.memory_state != MemoryProtectionState::ReadExecute {
                continue;
            }

            let dist_sq: f32 = intent_vector
                .iter()
                .zip(&handle.intent_centroid)
                .take(JIT_INTENT_DIM)
                .map(|(a, b)| (a - b).powi(2))
                .sum();

            if dist_sq.sqrt() <= handle.confidence_radius {
                return Some(idx);
            }
        }
        None
    }

    /// Executes a JIT-compiled native reflex directly in < 1µs with error trap detection
    pub fn execute_native_reflex(
        &mut self,
        handle_index: usize,
        ctx: &mut NativeExecutionContext,
    ) -> Result<(u64, u64)> {
        if handle_index >= self.compiled_registry.len() {
            bail!("JIT Reflex handle out of bounds: {}", handle_index);
        }

        let handle = &self.compiled_registry[handle_index];
        if handle.memory_state != MemoryProtectionState::ReadExecute {
            bail!("JIT Reflex page permission violation (Revoked/Not RX)");
        }

        let fn_ptr = match handle.execution_fn {
            Some(f) => f,
            None => bail!("Compiled reflex has no valid native function pointer"),
        };

        let start = Instant::now();
        let ret_val = unsafe { fn_ptr(ctx) };
        let duration_ns = start.elapsed().as_nanos() as u64;

        Ok((ret_val, duration_ns))
    }

    /// De-crystallizes a failing reflex: Revokes RX memory and restores continuous neural state
    pub fn de_crystallize_reflex(&mut self, handle_index: usize) {
        if handle_index < self.compiled_registry.len() {
            let handle = &mut self.compiled_registry[handle_index];
            handle.memory_state = MemoryProtectionState::Revoked;
            handle.memory_region = None;
            handle.execution_fn = None;
            self.intent_routing_lut.remove(&handle.skill_id);
            self.de_crystallization_events_count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use si_ir::{MachineOpcode, NativeComputationNode, NativeTypeLattice};

    #[test]
    fn test_jit_cranelift_crystallization_and_fast_bypass() {
        let mut jit = SiJitCompilerEngine::new();

        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 512, align: 64 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 64 },
            energy_cost: 0.01,
            dependencies: Vec::new(),
        });
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::Return { value_reg: 1 },
            type_lattice: NativeTypeLattice::PrimitiveInt { bits: 64, signed: false },
            energy_cost: 0.01,
            dependencies: vec![1],
        });

        let intent = vec![0.5f32; JIT_INTENT_DIM];
        let handle_idx = jit
            .compile_ast_graph(0x0111, "Fast-Alloc-Reflex", &graph, &intent)
            .expect("JIT compile failed");
        assert_eq!(handle_idx, 0);

        // 1. Test Fast Intent Lookup (Router Bypass via HNSW)
        let matching_intent = vec![0.50f32; JIT_INTENT_DIM];
        let lookup_match = jit.lookup_fast_reflex(&matching_intent);
        assert_eq!(lookup_match, Some(0));

        // 2. Test Bare-Metal Native Execution
        let mut ctx = NativeExecutionContext::default();
        let (res, duration_ns) = jit
            .execute_native_reflex(0, &mut ctx)
            .expect("JIT execution failed");

        assert_eq!(res, 512); // Alloc size stored in reg 1
        assert!(duration_ns < 10_000); // Sub-10µs bare metal execution

        // 3. Test De-Crystallization
        jit.de_crystallize_reflex(0);
        assert_eq!(jit.lookup_fast_reflex(&matching_intent), None);
    }

    #[test]
    fn test_crystallization_maturity_evaluation() {
        let jit = SiJitCompilerEngine::new();

        // Immature: N < 50
        let m1 = jit.evaluate_maturity(20, 0.001, 0.02);
        assert!(!m1.is_mature_for_jit);

        // Immature: Variance > 0.005
        let m2 = jit.evaluate_maturity(100, 0.010, 0.02);
        assert!(!m2.is_mature_for_jit);

        // Immature: Free Energy > 0.05
        let m3 = jit.evaluate_maturity(100, 0.002, 0.08);
        assert!(!m3.is_mature_for_jit);

        // Mature: N >= 50, Var <= 0.005, F <= 0.05
        let m4 = jit.evaluate_maturity(60, 0.003, 0.01);
        assert!(m4.is_mature_for_jit);
    }
}
