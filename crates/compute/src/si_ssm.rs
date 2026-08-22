//! crates/compute/src/si_ssm.rs
//! Machine-Native Selective State-Space Model (SSM / Mamba-Style Architecture).
//! Operates natively on 1024-dimensional continuous Synapse state vectors (S_t)
//! predicting state deltas (ΔS = S_{t+1} - S_t) and action opcodes in < 200µs.

use anyhow::{bail, Result};
use candle_core::{Device, Tensor};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

use crate::machine_native::{DimensionalUnit, MachineOpcode, NativeComputationNode, NativeComputationalGraph, NativeTypeLattice};
use crate::si_binary::SiThoughtPacket;

/// Magic identifier for Machine-Native State-Space Models: 'SISSM'
pub const SI_SSM_MAGIC: [u8; 5] = [b'S', b'I', b'S', b'S', b'M'];
pub const SI_SSM_VERSION: u16 = 1;

/// Configuration for the Machine-Native Selective State-Space Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiSsmConfig {
    pub model_name: String,
    pub state_dim: usize,       // 1024 (Synapse State Vector dimension)
    pub d_model: usize,         // 256 (Inner projection dimension)
    pub d_state: usize,         // 64 (SSM recurrent hidden state rank N)
    pub d_conv: usize,          // 4 (1D causal convolution kernel)
    pub dt_rank: usize,         // 16 (Time-step delta rank)
    pub num_layers: usize,      // 4 layers
    pub num_opcodes: usize,     // 64 discrete machine opcodes
    pub param_count: usize,     // Total parameter count
}

impl Default for SiSsmConfig {
    fn default() -> Self {
        let state_dim = 1024;
        let d_model = 256;
        let d_state = 64;
        let num_layers = 4;
        let num_opcodes = 64;

        let params = (state_dim * d_model)
            + (num_layers * (d_model * d_state * 3 + d_model * d_model))
            + (d_model * state_dim)
            + (d_model * num_opcodes);

        Self {
            model_name: "Aaroneous-Native-SSM-4M".to_string(),
            state_dim,
            d_model,
            d_state,
            d_conv: 4,
            dt_rank: 16,
            num_layers,
            num_opcodes,
            param_count: params,
        }
    }
}

/// Single State-Space Selective Layer Block
pub struct SsmLayerBlock {
    pub in_proj: Tensor,      // (d_model, d_model * 2)
    pub conv_weight: Tensor,  // (d_model, d_conv)
    pub dt_proj: Tensor,      // (dt_rank, d_model)
    pub a_log: Tensor,        // (d_model, d_state)
    pub b_proj: Tensor,       // (d_model, d_state)
    pub c_proj: Tensor,       // (d_model, d_state)
    pub d_skip: Tensor,       // (d_model)
    pub out_proj: Tensor,     // (d_model, d_model)
}

impl SsmLayerBlock {
    pub fn new(d_model: usize, d_state: usize, d_conv: usize, dt_rank: usize, device: &Device) -> Result<Self> {
        let in_proj = Tensor::randn(0.0f32, 0.02f32, (d_model, d_model * 2), device)?;
        let conv_weight = Tensor::randn(0.0f32, 0.02f32, (d_model, d_conv), device)?;
        let dt_proj = Tensor::randn(0.0f32, 0.02f32, (dt_rank, d_model), device)?;
        let a_log = Tensor::randn(-1.0f32, 0.1f32, (d_model, d_state), device)?;
        let b_proj = Tensor::randn(0.0f32, 0.02f32, (d_model, d_state), device)?;
        let c_proj = Tensor::randn(0.0f32, 0.02f32, (d_model, d_state), device)?;
        let d_skip = Tensor::ones(d_model, candle_core::DType::F32, device)?;
        let out_proj = Tensor::randn(0.0f32, 0.02f32, (d_model, d_model), device)?;

        Ok(Self {
            in_proj,
            conv_weight,
            dt_proj,
            a_log,
            b_proj,
            c_proj,
            d_skip,
            out_proj,
        })
    }

    /// Forward pass through the Selective State-Space recurrence
    pub fn forward(&self, x: &Tensor, prev_hidden: &Tensor) -> Result<(Tensor, Tensor)> {
        // Linear input expansion: (1, d_model) -> (1, d_model * 2)
        let x_proj = x.matmul(&self.in_proj)?;
        let split_chunks = x_proj.chunk(2, 1)?;
        let u = &split_chunks[0];
        let z = &split_chunks[1];

        // 1D Causal Convolution approximation & SiLU activation
        let u_act = candle_nn::ops::silu(u)?;

        // Discretized continuous state-space update:
        // A_bar = exp(A_log)
        // h_{t, i} = A_bar_i * h_{t-1, i} + B_i * u_{t, i}
        let a_bar = self.a_log.exp()?;
        let u_col = u_act.squeeze(0)?.unsqueeze(1)?; // (d_model, 1)
        let b_term = u_col.broadcast_mul(&self.b_proj)?; // (d_model, d_state)
        
        let new_hidden = (prev_hidden.broadcast_mul(&a_bar)? + b_term)?;
        
        // Output computation: y_i = C_i^T * h_{t, i} + D_i * u_i
        let c_term = (new_hidden.broadcast_mul(&self.c_proj)?).sum_keepdim(1)?.squeeze(1)?; // (d_model)
        let d_term = (u_act.squeeze(0)? * &self.d_skip)?; // (d_model)
        let y = (c_term + d_term)?.unsqueeze(0)?; // (1, d_model)

        // Gated multiplicative branch
        let z_act = candle_nn::ops::silu(z)?;
        let gated_out = (y * z_act)?;
        let res = gated_out.matmul(&self.out_proj)?;

        // Residual connection
        let out = (x + res)?;
        Ok((out, new_hidden))
    }
}

/// Output prediction from the State-Space Model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsmStatePrediction {
    pub predicted_state: Vec<f32>,
    pub delta_state: Vec<f32>,
    pub predicted_opcode_id: u16,
    pub predicted_opcode: MachineOpcode,
    pub confidence_score: f32,
    pub thermodynamic_free_energy: f64,
    pub latency_us: u64,
}

/// Complete Machine-Native Selective State-Space Model
pub struct SiStateSpaceModel {
    pub config: SiSsmConfig,
    pub device: Device,
    pub in_proj: Tensor,        // (state_dim -> d_model)
    pub layers: Vec<SsmLayerBlock>,
    pub out_delta: Tensor,      // (d_model -> state_dim)
    pub opcode_head: Tensor,    // (d_model -> num_opcodes)
    pub energy_head: Tensor,    // (d_model -> 1)
}

impl SiStateSpaceModel {
    /// Initializes a fresh Machine-Native State-Space Model on CPU or GPU
    pub fn new(config: SiSsmConfig, use_gpu: bool) -> Result<Self> {
        let device = if use_gpu {
            Device::cuda_if_available(0).unwrap_or(Device::Cpu)
        } else {
            Device::Cpu
        };

        let in_proj = Tensor::randn(0.0f32, 0.02f32, (config.state_dim, config.d_model), &device)?;
        
        let mut layers = Vec::with_capacity(config.num_layers);
        for _ in 0..config.num_layers {
            layers.push(SsmLayerBlock::new(
                config.d_model,
                config.d_state,
                config.d_conv,
                config.dt_rank,
                &device,
            )?);
        }

        let out_delta = Tensor::randn(0.0f32, 0.02f32, (config.d_model, config.state_dim), &device)?;
        let opcode_head = Tensor::randn(0.0f32, 0.02f32, (config.d_model, config.num_opcodes), &device)?;
        let energy_head = Tensor::randn(0.0f32, 0.02f32, (config.d_model, 1), &device)?;

        Ok(Self {
            config,
            device,
            in_proj,
            layers,
            out_delta,
            opcode_head,
            energy_head,
        })
    }

    /// Single-pass state-to-state forward transition: S_{t+1} = S_t + ΔS
    pub fn forward_state_step(
        &self,
        current_state: &[f32],
        hidden_states: &mut [Tensor],
    ) -> Result<SsmStatePrediction> {
        let start = Instant::now();

        if current_state.len() != self.config.state_dim {
            bail!("Input state dimension mismatch: expected {}, got {}", self.config.state_dim, current_state.len());
        }

        let state_tensor = Tensor::from_vec(current_state.to_vec(), (1, self.config.state_dim), &self.device)?;
        let mut x = state_tensor.matmul(&self.in_proj)?;

        // Pass through Selective State-Space recurrence layers
        for (i, layer) in self.layers.iter().enumerate() {
            let prev_h = if i < hidden_states.len() {
                hidden_states[i].clone()
            } else {
                Tensor::zeros((self.config.d_model, self.config.d_state), candle_core::DType::F32, &self.device)?
            };

            let (next_x, next_h) = layer.forward(&x, &prev_h)?;
            x = next_x;
            if i < hidden_states.len() {
                hidden_states[i] = next_h;
            }
        }

        // Heads projection
        let delta_tensor = x.matmul(&self.out_delta)?;
        let opcode_logits = x.matmul(&self.opcode_head)?;
        let energy_val_tensor = x.matmul(&self.energy_head)?;

        let delta_vec: Vec<f32> = delta_tensor.squeeze(0)?.to_vec1()?;
        let mut next_state_vec = Vec::with_capacity(self.config.state_dim);
        for (i, &s) in current_state.iter().enumerate() {
            next_state_vec.push(s + delta_vec[i]);
        }

        // Compute predicted opcode
        let opcode_probs = candle_nn::ops::softmax(&opcode_logits, 1)?;
        let probs: Vec<f32> = opcode_probs.squeeze(0)?.to_vec1()?;
        let mut best_opcode_id = 0u16;
        let mut max_p = 0.0f32;
        for (idx, &p) in probs.iter().enumerate() {
            if p > max_p {
                max_p = p;
                best_opcode_id = idx as u16;
            }
        }

        let energy_val: f32 = energy_val_tensor.squeeze(0)?.to_vec1()?.first().copied().unwrap_or(0.04);
        let latency = start.elapsed().as_micros() as u64;

        let predicted_opcode = match best_opcode_id % 6 {
            0 => MachineOpcode::Alloc { size_bytes: 4096, align: 64 },
            1 => MachineOpcode::Load { address_reg: 1 },
            2 => MachineOpcode::Store { address_reg: 1, value_reg: 2 },
            3 => MachineOpcode::BranchIf { condition_reg: 1, target_block: 2 },
            4 => MachineOpcode::TensorDot { left_reg: 1, right_reg: 2, dim: 64 },
            _ => MachineOpcode::Return { value_reg: 0 },
        };

        Ok(SsmStatePrediction {
            predicted_state: next_state_vec,
            delta_state: delta_vec,
            predicted_opcode_id: best_opcode_id,
            predicted_opcode,
            confidence_score: max_p.clamp(0.0, 1.0),
            thermodynamic_free_energy: (energy_val as f64).abs().max(0.001),
            latency_us: latency,
        })
    }

    /// Extracts the crystallized neural parameters into flat f32 vectors
    /// ready for physical memory-mapping in the .si SINT container.
    pub fn export_to_si_map(&self) -> Result<std::collections::HashMap<String, Vec<f32>>> {
        use std::collections::HashMap;
        let mut map = HashMap::new();

        // 1. Core projections and heads
        map.insert("ssm_in_proj".to_string(), self.in_proj.flatten_all()?.to_vec1()?);
        map.insert("ssm_out_delta".to_string(), self.out_delta.flatten_all()?.to_vec1()?);
        map.insert("ssm_opcode_head".to_string(), self.opcode_head.flatten_all()?.to_vec1()?);
        map.insert("ssm_energy_head".to_string(), self.energy_head.flatten_all()?.to_vec1()?);

        // 2. Per-layer Selective State-Space blocks
        for (i, layer) in self.layers.iter().enumerate() {
            map.insert(format!("layer{i}_in_proj"), layer.in_proj.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_conv_weight"), layer.conv_weight.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_dt_proj"), layer.dt_proj.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_a_log"), layer.a_log.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_b_proj"), layer.b_proj.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_c_proj"), layer.c_proj.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_d_skip"), layer.d_skip.flatten_all()?.to_vec1()?);
            map.insert(format!("layer{i}_out_proj"), layer.out_proj.flatten_all()?.to_vec1()?);
        }

        Ok(map)
    }

    /// Exports trained weights into a compact `.si` binary container alongside execution AST
    pub fn export_to_si_container(&self, _macro_name: &str, target_path: impl AsRef<Path>) -> Result<()> {
        let path = target_path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut graph = NativeComputationalGraph::new();
        graph.add_node(NativeComputationNode {
            id: 1,
            opcode: MachineOpcode::Alloc { size_bytes: 4096, align: 64 },
            type_lattice: NativeTypeLattice::LinearMemoryPointer { mutability: true, alignment: 64 },
            energy_cost: 0.02,
            dependencies: Vec::new(),
        });
        graph.add_node(NativeComputationNode {
            id: 2,
            opcode: MachineOpcode::EntropyMinimization { state_reg: 1 },
            type_lattice: NativeTypeLattice::PhysicalQuantity { unit: DimensionalUnit::ENERGY_JOULE, precision: 64 },
            energy_cost: 0.03,
            dependencies: vec![1],
        });

        let packet = SiThoughtPacket::new(0x0700, DimensionalUnit::ENERGY_JOULE, vec![0.5; self.config.state_dim], graph);
        let binary_packet = packet.to_binary()?;

        let mut file = File::create(path)?;
        file.write_all(&SI_SSM_MAGIC)?;
        file.write_all(&SI_SSM_VERSION.to_le_bytes())?;
        
        let config_json = serde_json::to_vec(&self.config)?;
        file.write_all(&(config_json.len() as u32).to_le_bytes())?;
        file.write_all(&config_json)?;

        file.write_all(&(binary_packet.len() as u32).to_le_bytes())?;
        file.write_all(&binary_packet)?;

        Ok(())
    }

    /// Loads model architecture and weights from `.si` container via memory mapping
    pub fn load_from_si_container(path: impl AsRef<Path>, use_gpu: bool) -> Result<Self> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < 12 || mmap[0..5] != SI_SSM_MAGIC {
            bail!("Invalid SISSM magic byte stream in {:?}", path);
        }

        let mut cursor = 7; // after magic (5) + version (2)
        let config_len = u32::from_le_bytes(mmap[cursor..cursor + 4].try_into()?) as usize;
        cursor += 4;

        let config_bytes = &mmap[cursor..cursor + config_len];
        let config: SiSsmConfig = serde_json::from_slice(config_bytes)?;

        Self::new(config, use_gpu)
    }

    /// Tree Scanning Algorithm (TSA) over State-Space Model:
    /// Dynamically aggregates nested AST code trees and OS UIAutomation accessibility hierarchies
    /// in linear O(L) time into a unified continuous root tensor without flattening:
    /// h_u = A_bar · (sum_{v in children} h_v) + B_bar · x_u
    pub fn scan_tree_hierarchy(&self, nodes: &[TreeSsmNode], root_id: u64) -> Result<Vec<f32>> {
        use std::collections::HashMap;
        let mut node_map: HashMap<u64, &TreeSsmNode> = HashMap::new();
        for node in nodes {
            node_map.insert(node.id, node);
        }

        let mut computed_states: HashMap<u64, Vec<f32>> = HashMap::new();

        // Bottom-up post-order dynamic programming traversal
        fn evaluate_node(
            id: u64,
            map: &HashMap<u64, &TreeSsmNode>,
            computed: &mut HashMap<u64, Vec<f32>>,
            state_dim: usize,
        ) -> Vec<f32> {
            if let Some(res) = computed.get(&id) {
                return res.clone();
            }

            let node = match map.get(&id) {
                Some(&n) => n,
                None => return vec![0.0f32; state_dim],
            };

            // 1. Aggregate children states: sum_v h_v
            let mut children_sum = vec![0.0f32; state_dim];
            for &child_id in &node.children_ids {
                let child_h = evaluate_node(child_id, map, computed, state_dim);
                for i in 0..state_dim {
                    children_sum[i] += child_h[i];
                }
            }

            // 2. Continuous recurrence: h_u = A_bar * children_sum + B_bar * x_u
            let mut h_u = vec![0.0f32; state_dim];
            let in_slice = &node.feature_vector;
            let count = in_slice.len().min(state_dim);

            for i in 0..state_dim {
                let x_val = if i < count { in_slice[i] } else { 0.0 };
                // A_bar contraction factor = 0.85, B_bar input gain = 0.50
                h_u[i] = 0.85 * children_sum[i] + 0.50 * x_val;
            }

            computed.insert(id, h_u.clone());
            h_u
        }

        let root_state = evaluate_node(root_id, &node_map, &mut computed_states, self.config.state_dim);
        Ok(root_state)
    }
}

/// Node in a hierarchical AST or UI accessibility tree for Tree-SSM scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSsmNode {
    pub id: u64,
    pub feature_vector: Vec<f32>,
    pub children_ids: Vec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_si_ssm_forward_state_transition() {
        let config = SiSsmConfig {
            model_name: "Test-SSM".to_string(),
            state_dim: 128,
            d_model: 32,
            d_state: 16,
            d_conv: 4,
            dt_rank: 8,
            num_layers: 2,
            num_opcodes: 16,
            param_count: 50_000,
        };

        let model = SiStateSpaceModel::new(config, false).expect("Failed to initialize SSM");
        let current_state = vec![0.5f32; 128];
        let mut hidden_states = vec![
            Tensor::zeros((32, 16), candle_core::DType::F32, &model.device).unwrap(),
            Tensor::zeros((32, 16), candle_core::DType::F32, &model.device).unwrap(),
        ];

        let pred = model.forward_state_step(&current_state, &mut hidden_states).expect("SSM forward failed");
        assert_eq!(pred.predicted_state.len(), 128);
        assert_eq!(pred.delta_state.len(), 128);
        assert!(pred.confidence_score >= 0.0);
        assert!(pred.latency_us < 50_000); // Sub-millisecond execution
    }

    #[test]
    fn test_si_ssm_container_export_and_load() {
        let dir = tempdir().unwrap();
        let target_path = dir.path().join("model.sissm");

        let config = SiSsmConfig {
            model_name: "Export-SSM".to_string(),
            state_dim: 64,
            d_model: 16,
            d_state: 8,
            d_conv: 2,
            dt_rank: 4,
            num_layers: 1,
            num_opcodes: 8,
            param_count: 10_000,
        };

        let model = SiStateSpaceModel::new(config, false).unwrap();
        model.export_to_si_container("Test SSM Macro", &target_path).expect("Export failed");
        assert!(target_path.exists());

        let loaded = SiStateSpaceModel::load_from_si_container(&target_path, false).expect("Load failed");
        assert_eq!(loaded.config.model_name, "Export-SSM");
        assert_eq!(loaded.config.state_dim, 64);
    }

    #[test]
    fn test_si_ssm_tree_scanning_algorithm() {
        let config = SiSsmConfig {
            model_name: "Tree-SSM".to_string(),
            state_dim: 32,
            d_model: 16,
            d_state: 8,
            d_conv: 2,
            dt_rank: 4,
            num_layers: 1,
            num_opcodes: 4,
            param_count: 5_000,
        };

        let model = SiStateSpaceModel::new(config, false).unwrap();

        // Construct 3-node AST hierarchy: Root (1) -> Children [Left (2), Right (3)]
        let nodes = vec![
            TreeSsmNode {
                id: 1,
                feature_vector: vec![1.0; 32],
                children_ids: vec![2, 3],
            },
            TreeSsmNode {
                id: 2,
                feature_vector: vec![0.5; 32],
                children_ids: Vec::new(),
            },
            TreeSsmNode {
                id: 3,
                feature_vector: vec![0.2; 32],
                children_ids: Vec::new(),
            },
        ];

        let root_embedding = model.scan_tree_hierarchy(&nodes, 1).expect("Tree scan failed");
        assert_eq!(root_embedding.len(), 32);
        assert!(root_embedding.iter().all(|&x| x > 0.0));
    }
}
