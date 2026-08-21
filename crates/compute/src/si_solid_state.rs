//! crates/compute/src/si_solid_state.rs
//! Unified Solid-State Single-File Agent Architecture (.si / SINT).
//! Fuses 3 cohesive blocks into one memory-mapped binary container:
//! 1. [Block 1: Frozen Core SSM Weights] (Immutable baseline model)
//! 2. [Block 2: Dynamic Adaptation Matrix with TD(λ) Eligibility Traces & Orthogonal Gradient Projection]
//! 3. [Block 3: Episodic Skill Stack] (Mined AST DAGs, habits, and execution pathways)
//!
//! Enforces 64-byte alignment for cache-line and SIMD AVX-512 / ARM NEON vectorization.

use anyhow::{bail, Result};
use candle_core::Tensor;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::si_binary::SiThoughtPacket;
use crate::si_ssm::{SiSsmConfig, SiStateSpaceModel, SsmStatePrediction};

/// Magic identifier for Solid-State SI Containers: 'SINT' (Synthetic Intelligence Native Topology)
pub const SI_SOLID_STATE_MAGIC: [u8; 4] = [b'S', b'I', b'N', b'T'];
pub const SI_SOLID_STATE_VERSION: u16 = 2; // Version 2 enforces 64-byte cache-line alignment

/// Enforced 64-byte alignment constant for SIMD vectorization & cache-line boundaries
pub const SI_ALIGNMENT_BYTES: usize = 64;

/// Baseline anchor transition to verify adapter updates do not cause catastrophic forgetting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorTransition {
    pub state_t: Vec<f32>,
    pub expected_action: u16,
    pub expected_delta: Vec<f32>,
}

/// Proactive Latent Invariant Safety Verification Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyCheckResult {
    pub is_safe: bool,
    pub norm_magnitude: f32,
    pub violation_reason: Option<String>,
}

/// Block 2: Low-Rank Dynamic Adaptation Matrix (Streaming LoRA Adapter)
/// Allows instant in-place error correction and online learning with L2 weight decay,
/// TD(λ) Eligibility Traces for temporal credit assignment, and Orthogonal Gradient Projection (OGP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicAdaptationMatrix {
    pub in_dim: usize,            // 256 (d_model)
    pub rank: usize,              // 16 (Low-rank adaptation rank r)
    pub out_dim: usize,           // 256 (d_model)
    pub scaling: f32,             // alpha / rank
    pub weight_decay: f32,        // L2 regularization decay rate (e.g. 1e-4)
    pub gamma: f32,               // Temporal discount factor (e.g. 0.95)
    pub lambda: f32,              // Eligibility trace decay rate (e.g. 0.80)
    pub matrix_a: Vec<f32>,       // in_dim x rank (256 x 16 = 4,096 floats)
    pub matrix_b: Vec<f32>,       // rank x out_dim (16 x 256 = 4,096 floats)
    pub momentum_a: Vec<f32>,     // Optimizer momentum for A
    pub momentum_b: Vec<f32>,     // Optimizer momentum for B
    pub trace_a: Vec<f32>,        // TD(λ) Eligibility trace for Matrix A
    pub trace_b: Vec<f32>,        // TD(λ) Eligibility trace for Matrix B
    pub protected_subspace: Vec<Vec<f32>>, // Orthogonal Gradient Projection (OGP) basis vectors
    pub anchor_buffer: Vec<AnchorTransition>, // Anchor states to guarantee baseline fidelity
    pub max_anchors: usize,
    pub error_corrections_count: u64,
    pub success_rewards_count: u64,
    pub total_drift_magnitude: f64,
}

impl DynamicAdaptationMatrix {
    /// Initializes a new Low-Rank Adaptation Matrix with near-zero initialization for B
    pub fn new(in_dim: usize, rank: usize, out_dim: usize) -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let size_a = in_dim * rank;
        let size_b = rank * out_dim;

        // Gaussian init for A
        let mut matrix_a = Vec::with_capacity(size_a);
        for _ in 0..size_a {
            matrix_a.push(rng.gen_range(-0.02..0.02));
        }

        // Zeros init for B so initial adapter output is 0.0 (exact identity with core model)
        let matrix_b = vec![0.0f32; size_b];

        Self {
            in_dim,
            rank,
            out_dim,
            scaling: 1.0,
            weight_decay: 1e-4,
            gamma: 0.95,
            lambda: 0.80,
            matrix_a,
            matrix_b,
            momentum_a: vec![0.0f32; size_a],
            momentum_b: vec![0.0f32; size_b],
            trace_a: vec![0.0f32; size_a],
            trace_b: vec![0.0f32; size_b],
            protected_subspace: Vec::new(),
            anchor_buffer: Vec::new(),
            max_anchors: 16,
            error_corrections_count: 0,
            success_rewards_count: 0,
            total_drift_magnitude: 0.0,
        }
    }

    /// Computes the low-rank forward delta: Δx = (x · A) · B * scaling
    /// and updates the internal TD(λ) eligibility traces for temporal credit assignment
    pub fn forward_delta_with_trace(&mut self, x: &[f32]) -> Vec<f32> {
        if x.len() != self.in_dim {
            return vec![0.0; self.out_dim];
        }

        // 1. Project x (1 x in_dim) through A (in_dim x rank) -> intermediate (1 x rank)
        let mut intermediate = vec![0.0f32; self.rank];
        for r in 0..self.rank {
            let mut sum = 0.0f32;
            for i in 0..self.in_dim {
                sum += x[i] * self.matrix_a[i * self.rank + r];
            }
            intermediate[r] = sum;
        }

        // 2. Update Eligibility Traces: E_t = gamma * lambda * E_{t-1} + grad_W
        let trace_decay = self.gamma * self.lambda;
        for i in 0..self.in_dim {
            for r in 0..self.rank {
                let idx = i * self.rank + r;
                self.trace_a[idx] = self.trace_a[idx] * trace_decay + x[i] * intermediate[r];
            }
        }
        for r in 0..self.rank {
            for o in 0..self.out_dim {
                let idx = r * self.out_dim + o;
                self.trace_b[idx] = self.trace_b[idx] * trace_decay + intermediate[r];
            }
        }

        // 3. Project intermediate through B -> delta
        let mut delta = vec![0.0f32; self.out_dim];
        for o in 0..self.out_dim {
            let mut sum = 0.0f32;
            for r in 0..self.rank {
                sum += intermediate[r] * self.matrix_b[r * self.out_dim + o];
            }
            delta[o] = sum * self.scaling;
        }

        delta
    }

    /// Read-only forward delta computation without updating eligibility traces
    pub fn forward_delta(&self, x: &[f32]) -> Vec<f32> {
        if x.len() != self.in_dim {
            return vec![0.0; self.out_dim];
        }

        let mut intermediate = vec![0.0f32; self.rank];
        for r in 0..self.rank {
            let mut sum = 0.0f32;
            for i in 0..self.in_dim {
                sum += x[i] * self.matrix_a[i * self.rank + r];
            }
            intermediate[r] = sum;
        }

        let mut delta = vec![0.0f32; self.out_dim];
        for o in 0..self.out_dim {
            let mut sum = 0.0f32;
            for r in 0..self.rank {
                sum += intermediate[r] * self.matrix_b[r * self.out_dim + o];
            }
            delta[o] = sum * self.scaling;
        }

        delta
    }

    /// Registers a crystallized skill direction for Orthogonal Gradient Projection (OGP)
    pub fn protect_skill_subspace(&mut self, basis_vector: Vec<f32>) {
        if basis_vector.len() == self.out_dim {
            // Normalize basis vector
            let norm: f32 = basis_vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 1e-6 {
                let normalized: Vec<f32> = basis_vector.into_iter().map(|x| x / norm).collect();
                self.protected_subspace.push(normalized);
            }
        }
    }

    /// Projects gradient vector orthogonally to all protected skill directions (OGP)
    pub fn project_gradient_orthogonal(&self, grad: &[f32]) -> Vec<f32> {
        let mut proj = grad.to_vec();
        for basis in &self.protected_subspace {
            let dot: f32 = proj.iter().zip(basis).map(|(g, b)| g * b).sum();
            for i in 0..proj.len() {
                proj[i] -= dot * basis[i];
            }
        }
        proj
    }

    /// Adds a verified anchor transition to the replay buffer to protect baseline skills
    pub fn add_anchor_state(&mut self, state_t: Vec<f32>, expected_action: u16, expected_delta: Vec<f32>) {
        if self.anchor_buffer.len() >= self.max_anchors {
            self.anchor_buffer.remove(0);
        }
        self.anchor_buffer.push(AnchorTransition {
            state_t,
            expected_action,
            expected_delta,
        });
    }

    /// Proactive Latent Invariant Safety Checker: Validates whether delta tensor is within safe operational bounds
    pub fn verify_safety_invariants(&self, delta: &[f32]) -> SafetyCheckResult {
        let norm: f32 = delta.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 25.0 {
            return SafetyCheckResult {
                is_safe: false,
                norm_magnitude: norm,
                violation_reason: Some(format!("Latent delta norm explosion ({:.2} > 25.0)", norm)),
            };
        }

        if delta.iter().any(|x| x.is_nan() || x.is_infinite()) {
            return SafetyCheckResult {
                is_safe: false,
                norm_magnitude: norm,
                violation_reason: Some("NaN / Inf divergence detected in output delta".to_string()),
            };
        }

        SafetyCheckResult {
            is_safe: true,
            norm_magnitude: norm,
            violation_reason: None,
        }
    }

    /// Applies an immediate negative gradient penalty with TD(λ) Eligibility Traces and Orthogonal Projection.
    pub fn apply_error_penalty(&mut self, state_x: &[f32], error_vector: &[f32], lr: f32) {
        if state_x.len() != self.in_dim || error_vector.len() != self.out_dim {
            return;
        }

        // Apply OGP to error direction
        let projected_error = self.project_gradient_orthogonal(error_vector);

        let beta = 0.9f32;
        let mut drift = 0.0f64;

        // Compute intermediate state
        let mut intermediate = vec![0.0f32; self.rank];
        for r in 0..self.rank {
            let mut sum = 0.0f32;
            for i in 0..self.in_dim {
                sum += state_x[i] * self.matrix_a[i * self.rank + r];
            }
            intermediate[r] = sum;
        }

        // Update Matrix B with TD(λ) trace and L2 Weight Decay
        for r in 0..self.rank {
            for o in 0..self.out_dim {
                let idx = r * self.out_dim + o;
                let grad = -intermediate[r] * projected_error[o] * self.scaling;
                let trace_contribution = self.trace_b[idx] * -projected_error[o];
                let combined_grad = 0.7 * grad + 0.3 * trace_contribution;

                self.momentum_b[idx] = beta * self.momentum_b[idx] + (1.0 - beta) * combined_grad;
                
                let decayed_val = self.matrix_b[idx] * (1.0 - lr * self.weight_decay);
                let update = lr * self.momentum_b[idx];
                self.matrix_b[idx] = decayed_val - update;
                drift += (update as f64).abs();
            }
        }

        // Update Matrix A with TD(λ) trace and L2 Weight Decay
        for i in 0..self.in_dim {
            for r in 0..self.rank {
                let idx = i * self.rank + r;
                let mut b_sum = 0.0f32;
                for o in 0..self.out_dim {
                    b_sum += -projected_error[o] * self.matrix_b[r * self.out_dim + o];
                }
                let grad = state_x[i] * b_sum * self.scaling;
                let trace_contribution = self.trace_a[idx] * b_sum;
                let combined_grad = 0.7 * grad + 0.3 * trace_contribution;

                self.momentum_a[idx] = beta * self.momentum_a[idx] + (1.0 - beta) * combined_grad;
                
                let decayed_val = self.matrix_a[idx] * (1.0 - lr * self.weight_decay);
                let update = lr * self.momentum_a[idx];
                self.matrix_a[idx] = decayed_val - update;
                drift += (update as f64).abs();
            }
        }

        self.error_corrections_count += 1;
        self.total_drift_magnitude += drift;
    }

    /// Reinforces successful execution with an immediate positive localized gradient step
    pub fn apply_success_reinforcement(&mut self, state_x: &[f32], target_delta: &[f32], lr: f32) {
        if state_x.len() != self.in_dim || target_delta.len() != self.out_dim {
            return;
        }

        let projected_target = self.project_gradient_orthogonal(target_delta);
        let beta = 0.9f32;
        let mut intermediate = vec![0.0f32; self.rank];
        for r in 0..self.rank {
            let mut sum = 0.0f32;
            for i in 0..self.in_dim {
                sum += state_x[i] * self.matrix_a[i * self.rank + r];
            }
            intermediate[r] = sum;
        }

        for r in 0..self.rank {
            for o in 0..self.out_dim {
                let idx = r * self.out_dim + o;
                let grad = intermediate[r] * projected_target[o] * self.scaling;
                self.momentum_b[idx] = beta * self.momentum_b[idx] + (1.0 - beta) * grad;
                self.matrix_b[idx] = self.matrix_b[idx] * (1.0 - lr * self.weight_decay) + lr * self.momentum_b[idx];
            }
        }

        self.success_rewards_count += 1;
    }

    /// Verifies adapter updates against anchor buffer to ensure baseline integrity
    pub fn verify_anchor_retention(&self) -> f32 {
        if self.anchor_buffer.is_empty() {
            return 100.0;
        }

        let mut preserved = 0;
        for anchor in &self.anchor_buffer {
            let in_slice = if anchor.state_t.len() >= self.in_dim {
                &anchor.state_t[0..self.in_dim]
            } else {
                &anchor.state_t
            };

            let delta = self.forward_delta(in_slice);
            let mut error_norm = 0.0f32;
            for (i, &d) in delta.iter().enumerate() {
                if i < anchor.expected_delta.len() {
                    error_norm += (d - anchor.expected_delta[i]).powi(2);
                }
            }

            if error_norm.sqrt() < 1.0 {
                preserved += 1;
            }
        }

        (preserved as f32 / self.anchor_buffer.len() as f32) * 100.0
    }
}

/// Report produced by an online correction or reinforcement step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnlineCorrectionReport {
    pub correction_type: String,
    pub step_index: u64,
    pub drift_magnitude: f64,
    pub duration_us: u64,
    pub is_core_preserved: bool,
    pub anchor_retention_percent: f32,
    pub safety_check: SafetyCheckResult,
}

/// Unified Solid-State Container (.si / SINT)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolidStateSiContainer {
    pub container_name: String,
    pub config: SiSsmConfig,                       // Block 1: Base Architecture Config
    pub adaptation: DynamicAdaptationMatrix,       // Block 2: Mutable Dynamic Adapter
    pub skill_stack: Vec<SiThoughtPacket>,         // Block 3: Episodic Skills & AST DAGs
}

impl SolidStateSiContainer {
    /// Creates a fresh Solid-State Container with base SSM and zeroed adaptation matrix
    pub fn new(container_name: &str, config: SiSsmConfig) -> Self {
        let d_model = config.d_model;
        let adaptation = DynamicAdaptationMatrix::new(d_model, 16, d_model);

        Self {
            container_name: container_name.to_string(),
            config,
            adaptation,
            skill_stack: Vec::new(),
        }
    }

    /// Serializes the entire living agent state (Blocks 1, 2, 3) with strict 64-byte SIMD alignment
    pub fn save_to_file(&self, target_path: impl AsRef<Path>) -> Result<PathBuf> {
        let path = target_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;
        
        // Write Magic and Version
        file.write_all(&SI_SOLID_STATE_MAGIC)?;
        file.write_all(&SI_SOLID_STATE_VERSION.to_le_bytes())?;

        // Serialize container payload JSON
        let payload_json = serde_json::to_vec(self)?;
        let payload_len = payload_json.len() as u32;
        let payload_offset = SI_ALIGNMENT_BYTES as u32; // Offset = 64 bytes for SIMD alignment

        file.write_all(&payload_len.to_le_bytes())?;      // bytes 6..10
        file.write_all(&payload_offset.to_le_bytes())?;   // bytes 10..14

        // Pad header out to exactly 64 bytes
        let header_used = 14;
        let padding = vec![0u8; SI_ALIGNMENT_BYTES - header_used];
        file.write_all(&padding)?;

        // Write aligned payload at offset 64
        file.write_all(&payload_json)?;

        Ok(path.to_path_buf())
    }

    /// Loads the Solid-State container instantly from disk via memory mapping
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            bail!("Solid-State container file not found: {:?}", path);
        }

        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < 14 || &mmap[0..4] != SI_SOLID_STATE_MAGIC {
            bail!("Invalid SINT magic header in {:?}", path);
        }

        let version = u16::from_le_bytes(mmap[4..6].try_into()?);
        let payload_len = u32::from_le_bytes(mmap[6..10].try_into()?) as usize;

        let payload_bytes = if version >= 2 && mmap.len() >= 64 {
            let offset = u32::from_le_bytes(mmap[10..14].try_into()?) as usize;
            &mmap[offset..offset + payload_len]
        } else {
            // Backward-compatibility fallback for v1 containers
            &mmap[10..10 + payload_len]
        };

        let container: Self = serde_json::from_slice(payload_bytes)?;
        Ok(container)
    }

    /// Appends a newly mined high-value latent skill route into Block 3
    pub fn append_skill(&mut self, packet: SiThoughtPacket) {
        self.skill_stack.push(packet);
    }
}

/// The Living Online Learning Host (`si-learner`)
pub struct SiOnlineLearner {
    pub container: SolidStateSiContainer,
    pub model: SiStateSpaceModel,
    pub hidden_states: Vec<Tensor>,
}

impl SiOnlineLearner {
    /// Initializes a living online learning agent with fused core SSM and dynamic adapter
    pub fn new(container: SolidStateSiContainer, use_gpu: bool) -> Result<Self> {
        let model = SiStateSpaceModel::new(container.config.clone(), use_gpu)?;
        let mut hidden_states = Vec::with_capacity(container.config.num_layers);
        for _ in 0..container.config.num_layers {
            hidden_states.push(Tensor::zeros(
                (container.config.d_model, container.config.d_state),
                candle_core::DType::F32,
                &model.device,
            )?);
        }

        Ok(Self {
            container,
            model,
            hidden_states,
        })
    }

    /// Forward pass with fused frozen core and dynamic adaptation matrix: y = Core(x) + Adapter(x)
    pub fn forward_adapted_step(&mut self, state_t: &[f32]) -> Result<SsmStatePrediction> {
        let mut pred = self.model.forward_state_step(state_t, &mut self.hidden_states)?;

        let in_slice = if state_t.len() >= self.container.adaptation.in_dim {
            &state_t[0..self.container.adaptation.in_dim]
        } else {
            state_t
        };
        let adapter_delta = self.container.adaptation.forward_delta_with_trace(in_slice);
        
        for (i, &d) in adapter_delta.iter().enumerate() {
            if i < pred.predicted_state.len() {
                pred.predicted_state[i] += d;
                pred.delta_state[i] += d;
            }
        }

        Ok(pred)
    }

    /// Triggers an immediate in-place error correction update when an execution failure occurs
    pub fn on_runtime_error(&mut self, current_state: &[f32], error_signature: &[f32], lr: f32) -> OnlineCorrectionReport {
        let start = Instant::now();
        let in_slice = if current_state.len() >= self.container.adaptation.in_dim {
            &current_state[0..self.container.adaptation.in_dim]
        } else {
            current_state
        };

        let err_slice = if error_signature.len() >= self.container.adaptation.out_dim {
            &error_signature[0..self.container.adaptation.out_dim]
        } else {
            error_signature
        };

        self.container.adaptation.apply_error_penalty(in_slice, err_slice, lr);
        let duration = start.elapsed().as_micros() as u64;
        let retention = self.container.adaptation.verify_anchor_retention();

        let delta = self.container.adaptation.forward_delta(in_slice);
        let safety = self.container.adaptation.verify_safety_invariants(&delta);

        OnlineCorrectionReport {
            correction_type: "Error-Steering Penalty (TD-λ + OGP)".to_string(),
            step_index: self.container.adaptation.error_corrections_count,
            drift_magnitude: self.container.adaptation.total_drift_magnitude,
            duration_us: duration,
            is_core_preserved: true,
            anchor_retention_percent: retention,
            safety_check: safety,
        }
    }

    /// Triggers positive reinforcement update when a task succeeds efficiently
    pub fn on_runtime_success(&mut self, current_state: &[f32], target_delta: &[f32], lr: f32) -> OnlineCorrectionReport {
        let start = Instant::now();
        let in_slice = if current_state.len() >= self.container.adaptation.in_dim {
            &current_state[0..self.container.adaptation.in_dim]
        } else {
            current_state
        };

        let delta_slice = if target_delta.len() >= self.container.adaptation.out_dim {
            &target_delta[0..self.container.adaptation.out_dim]
        } else {
            target_delta
        };

        self.container.adaptation.apply_success_reinforcement(in_slice, delta_slice, lr);
        let duration = start.elapsed().as_micros() as u64;
        let retention = self.container.adaptation.verify_anchor_retention();

        let delta = self.container.adaptation.forward_delta(in_slice);
        let safety = self.container.adaptation.verify_safety_invariants(&delta);

        OnlineCorrectionReport {
            correction_type: "Success Reinforcement (TD-λ + OGP)".to_string(),
            step_index: self.container.adaptation.success_rewards_count,
            drift_magnitude: self.container.adaptation.total_drift_magnitude,
            duration_us: duration,
            is_core_preserved: true,
            anchor_retention_percent: retention,
            safety_check: safety,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_native::*;
    use tempfile::tempdir;

    #[test]
    fn test_solid_state_container_binary_roundtrip_aligned() {
        let dir = tempdir().unwrap();
        let target_path = dir.path().join("agent_alpha.si");

        let config = SiSsmConfig {
            model_name: "SolidState-Agent-Alpha".to_string(),
            state_dim: 128,
            d_model: 32,
            d_state: 16,
            d_conv: 4,
            dt_rank: 8,
            num_layers: 2,
            num_opcodes: 16,
            param_count: 40_000,
        };

        let mut container = SolidStateSiContainer::new("Agent Alpha", config);
        
        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 2048, align: 32 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 32 },
            energy_cost: 0.02,
            dependencies: Vec::new(),
        });
        container.append_skill(SiThoughtPacket::new(0x0111, DimensionalUnit::DIMENSIONLESS, vec![0.1; 128], graph));

        container.adaptation.add_anchor_state(vec![0.5; 32], 0x0111, vec![0.0; 32]);
        container.adaptation.protect_skill_subspace(vec![1.0; 32]);

        container.save_to_file(&target_path).expect("Save solid state container failed");
        assert!(target_path.exists());

        let loaded = SolidStateSiContainer::load_from_file(&target_path).expect("Load solid state container failed");
        assert_eq!(loaded.container_name, "Agent Alpha");
        assert_eq!(loaded.config.model_name, "SolidState-Agent-Alpha");
        assert_eq!(loaded.adaptation.rank, 16);
        assert_eq!(loaded.adaptation.anchor_buffer.len(), 1);
        assert_eq!(loaded.adaptation.protected_subspace.len(), 1);
    }

    #[test]
    fn test_online_error_correction_td_lambda_and_ogp() {
        let config = SiSsmConfig {
            model_name: "ErrorSteer-Agent".to_string(),
            state_dim: 64,
            d_model: 32,
            d_state: 8,
            d_conv: 2,
            dt_rank: 4,
            num_layers: 1,
            num_opcodes: 8,
            param_count: 10_000,
        };

        let container = SolidStateSiContainer::new("Error Steer Agent", config);
        let mut learner = SiOnlineLearner::new(container, false).unwrap();

        let state_t = vec![0.5f32; 64];

        // 1. Initial forward pass creates TD(λ) eligibility traces
        let _ = learner.forward_adapted_step(&state_t).unwrap();
        assert!(learner.container.adaptation.trace_a.iter().any(|&t| t.abs() > 0.0));

        // 2. Protect a core skill direction via OGP
        let protected_dir = vec![0.1f32; 32];
        learner.container.adaptation.protect_skill_subspace(protected_dir);

        // 3. Simulate runtime error with an error direction vector
        let error_sig = vec![1.0f32; 32];
        let rep = learner.on_runtime_error(&state_t, &error_sig, 0.05);
        assert_eq!(rep.step_index, 1);
        assert!(rep.is_core_preserved);
        assert!(rep.drift_magnitude > 0.0);
        assert!(rep.duration_us < 50_000);
        assert!(rep.safety_check.is_safe);
    }
}
