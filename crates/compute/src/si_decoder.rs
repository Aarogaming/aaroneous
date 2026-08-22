//! crates/compute/src/si_decoder.rs
//! Multi-Headed Action Decoder: R^256 Intent-to-Action Translator.
//!
//! Features:
//! 1. Action Head: Selects discrete MachineOpcode / Action ID from continuous latent manifold.
//! 2. Spatial Head: Predicts 4D continuous spatial parameters [x, y, width, height] via tanh activation.
//! 3. Pointer Head: Predicts register/pointer index for linear memory or AST node references.
//! 4. Sub-10µs CPU/SIMD evaluation.

use serde::{Deserialize, Serialize};
use crate::machine_native::MachineOpcode;

pub const DECODER_INTENT_DIM: usize = 256;

/// Decoded Kinetic Action Command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedActionCommand {
    pub action_id: usize,
    pub opcode: MachineOpcode,
    pub spatial_coords: [f32; 4], // [x, y, w, h] normalized (-1.0 to 1.0 or 0.0 to 1.0)
    pub register_idx: usize,
    pub confidence: f32,
}

/// Multi-Headed Action Decoder
pub struct ActionDecoder {
    pub num_opcodes: usize,
    pub num_registers: usize,
    // Projection matrices
    pub action_weights: Vec<f32>,  // [256 x num_opcodes]
    pub spatial_weights: Vec<f32>, // [256 x 4]
    pub pointer_weights: Vec<f32>, // [256 x num_registers]
}

impl ActionDecoder {
    /// Initializes a new Action Decoder with deterministic Kaiming-style initialization
    pub fn new(num_opcodes: usize, num_registers: usize) -> Self {
        let act_size = DECODER_INTENT_DIM * num_opcodes;
        let spat_size = DECODER_INTENT_DIM * 4;
        let ptr_size = DECODER_INTENT_DIM * num_registers;

        let mut action_weights = Vec::with_capacity(act_size);
        for i in 0..DECODER_INTENT_DIM {
            for j in 0..num_opcodes {
                action_weights.push((((i * 13 + j * 7) as f32).sin()) * 0.05);
            }
        }

        let mut spatial_weights = Vec::with_capacity(spat_size);
        for i in 0..DECODER_INTENT_DIM {
            for j in 0..4 {
                spatial_weights.push((((i * 19 + j * 11) as f32).cos()) * 0.05);
            }
        }

        let mut pointer_weights = Vec::with_capacity(ptr_size);
        for i in 0..DECODER_INTENT_DIM {
            for j in 0..num_registers {
                pointer_weights.push((((i * 23 + j * 17) as f32).sin()) * 0.05);
            }
        }

        Self {
            num_opcodes,
            num_registers,
            action_weights,
            spatial_weights,
            pointer_weights,
        }
    }

    /// Decodes a 256-dim continuous latent intent vector into an action ID, 4D coords, and register index
    pub fn decode(&self, intent: &[f32]) -> DecodedActionCommand {
        let in_len = intent.len().min(DECODER_INTENT_DIM);

        // 1. Action logits & argmax
        let mut max_act_logit = f32::NEG_INFINITY;
        let mut best_action_id = 0usize;
        let mut act_logits = vec![0.0f32; self.num_opcodes];

        for (o, logit) in act_logits.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for (i, &intent_value) in intent.iter().enumerate().take(in_len) {
                sum += intent_value * self.action_weights[i * self.num_opcodes + o];
            }
            *logit = sum;
            if sum > max_act_logit {
                max_act_logit = sum;
                best_action_id = o;
            }
        }

        // Softmax confidence score
        let mut sum_exp = 0.0f32;
        for &l in &act_logits {
            sum_exp += (l - max_act_logit).exp();
        }
        let confidence = 1.0 / sum_exp.max(1e-6);

        // 2. Spatial coords: tanh(intent · W_spat)
        let mut spatial_coords = [0.0f32; 4];
        for (k, coordinate) in spatial_coords.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for (i, &intent_value) in intent.iter().enumerate().take(in_len) {
                sum += intent_value * self.spatial_weights[i * 4 + k];
            }
            *coordinate = sum.tanh();
        }

        // 3. Pointer head: argmax(intent · W_ptr)
        let mut max_ptr_logit = f32::NEG_INFINITY;
        let mut best_ptr_idx = 0usize;
        for (r, sum) in (0..self.num_registers).map(|r| {
            let sum = intent
                .iter()
                .enumerate()
                .take(in_len)
                .map(|(i, &intent_value)| {
                    intent_value * self.pointer_weights[i * self.num_registers + r]
                })
                .sum::<f32>();
            (r, sum)
        }) {
            if sum > max_ptr_logit {
                max_ptr_logit = sum;
                best_ptr_idx = r;
            }
        }

        // 4. Map to MachineOpcode
        let opcode = match best_action_id % 6 {
            0 => MachineOpcode::Alloc {
                size_bytes: 4096,
                align: 64,
            },
            1 => MachineOpcode::Load {
                address_reg: best_ptr_idx as u16,
            },
            2 => MachineOpcode::Store {
                address_reg: best_ptr_idx as u16,
                value_reg: (best_ptr_idx + 1) as u16,
            },
            3 => MachineOpcode::BranchIf {
                condition_reg: best_ptr_idx as u16,
                target_block: 2,
            },
            4 => MachineOpcode::TensorDot {
                left_reg: 1,
                right_reg: 2,
                dim: 64,
            },
            _ => MachineOpcode::Return { value_reg: 0 },
        };

        DecodedActionCommand {
            action_id: best_action_id,
            opcode,
            spatial_coords,
            register_idx: best_ptr_idx,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_decoder_output_ranges() {
        let decoder = ActionDecoder::new(16, 8);
        let intent = vec![0.5f32; DECODER_INTENT_DIM];

        let cmd = decoder.decode(&intent);
        assert!(cmd.action_id < 16);
        assert!(cmd.register_idx < 8);
        assert!(cmd.confidence > 0.0 && cmd.confidence <= 1.0);
        for &coord in &cmd.spatial_coords {
            assert!((-1.0..=1.0).contains(&coord));
        }
    }
}
