//! crates/adaptation_engine/src/streaming_adaptation.rs
//! Streaming Online LoRA Adaptation Pipeline with Orthogonal Gradient Projection (OGP).
//! Continuously computes prediction error in continuous-time latent spaces and applies
//! non-interfering LoRA delta updates to Block 2 of `.si` model containers, preventing
//! catastrophic forgetting of protected core task capabilities.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

pub const LATENT_DIM: usize = 256;

/// Report returned from a single streaming adaptation step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingAdaptationReport {
    pub step: u64,
    pub mse_loss: f32,
    pub raw_gradient_norm: f32,
    pub projected_gradient_norm: f32,
    pub protected_subspace_dim: usize,
    pub adaptation_applied: bool,
}

/// Streaming Online LoRA Adaptation Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingLoraAdaptationPipeline {
    pub d_model: usize,
    pub lora_rank: usize,
    pub lora_a: Vec<f32>, // rank x d_model
    pub lora_b: Vec<f32>, // d_model x rank
    pub protected_core_subspace: Vec<Vec<f32>>, // Orthonormal basis vectors in R^256
    pub learning_rate: f32,
    pub adaptation_steps: u64,
    pub max_gradient_norm: f32,
}

impl Default for StreamingLoraAdaptationPipeline {
    fn default() -> Self {
        Self::new(LATENT_DIM, 16, 0.005)
    }
}

impl StreamingLoraAdaptationPipeline {
    /// Initializes a new streaming adaptation pipeline
    pub fn new(d_model: usize, lora_rank: usize, learning_rate: f32) -> Self {
        let size_a = lora_rank * d_model;
        let size_b = d_model * lora_rank;

        // Initialize LoRA A with small Gaussian/uniform random weights
        let mut lora_a = Vec::with_capacity(size_a);
        for i in 0..size_a {
            let val = ((i % 17) as f32 - 8.0) * 0.001;
            lora_a.push(val);
        }

        // Initialize LoRA B to zeros (standard LoRA identity initialization)
        let lora_b = vec![0.0f32; size_b];

        Self {
            d_model,
            lora_rank,
            lora_a,
            lora_b,
            protected_core_subspace: Vec::new(),
            learning_rate,
            adaptation_steps: 0,
            max_gradient_norm: 1.0,
        }
    }

    /// Registers a protected core task gradient vector, adding it as a normalized basis vector
    pub fn register_protected_core_task(&mut self, core_gradient: &[f32]) -> Result<()> {
        if core_gradient.len() != self.d_model {
            return Err(anyhow!(
                "Core gradient length {} does not match d_model {}",
                core_gradient.len(),
                self.d_model
            ));
        }

        // Gram-Schmidt orthogonalization against existing basis
        let mut v = core_gradient.to_vec();
        for u in &self.protected_core_subspace {
            let dot: f32 = v.iter().zip(u).map(|(a, b)| a * b).sum();
            for i in 0..self.d_model {
                v[i] -= dot * u[i];
            }
        }

        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for x in &mut v {
                *x /= norm;
            }
            self.protected_core_subspace.push(v);
        }

        Ok(())
    }

    /// Projects an incoming gradient orthogonally to all protected core task vectors (OGP)
    pub fn project_orthogonal(&self, raw_gradient: &[f32]) -> Vec<f32> {
        let mut g_proj = raw_gradient.to_vec();

        for u in &self.protected_core_subspace {
            let dot: f32 = g_proj.iter().zip(u).map(|(a, b)| a * b).sum();
            for i in 0..self.d_model {
                g_proj[i] -= dot * u[i];
            }
        }

        g_proj
    }

    /// Continuous-time streaming prediction error update step
    pub fn update_from_prediction_error(
        &mut self,
        target_latent: &[f32; LATENT_DIM],
        predicted_latent: &[f32; LATENT_DIM],
    ) -> StreamingAdaptationReport {
        self.adaptation_steps += 1;

        // 1. Compute prediction error: delta = target - predicted
        let mut raw_grad = [0.0f32; LATENT_DIM];
        let mut mse_sum = 0.0f32;

        for i in 0..LATENT_DIM {
            let err = target_latent[i] - predicted_latent[i];
            raw_grad[i] = err;
            mse_sum += err * err;
        }
        let mse_loss = mse_sum / LATENT_DIM as f32;

        let raw_norm: f32 = raw_grad.iter().map(|x| x * x).sum::<f32>().sqrt();

        // 2. Apply Orthogonal Gradient Projection (OGP)
        let projected_grad = self.project_orthogonal(&raw_grad);
        let proj_norm_raw: f32 = projected_grad.iter().map(|x| x * x).sum::<f32>().sqrt();

        // 3. Gradient clipping
        let scale = if proj_norm_raw > self.max_gradient_norm {
            self.max_gradient_norm / proj_norm_raw
        } else {
            1.0
        };

        // 4. In-place LoRA rank-1 delta update: B += lr * g_proj * A_mean
        let lr = self.learning_rate * scale;
        for i in 0..self.d_model {
            let g = projected_grad[i];
            for r in 0..self.lora_rank {
                let idx_b = i * self.lora_rank + r;
                let idx_a = r * self.d_model + i;
                let a_val = self.lora_a[idx_a];
                self.lora_b[idx_b] += lr * g * a_val;
            }
        }

        StreamingAdaptationReport {
            step: self.adaptation_steps,
            mse_loss,
            raw_gradient_norm: raw_norm,
            projected_gradient_norm: proj_norm_raw,
            protected_subspace_dim: self.protected_core_subspace.len(),
            adaptation_applied: true,
        }
    }

    /// Evaluates forward LoRA projection: output = input + (B * A) * input
    pub fn forward(&self, input: &[f32; LATENT_DIM]) -> [f32; LATENT_DIM] {
        let mut intermediate = vec![0.0f32; self.lora_rank];

        // A * input (rank x d_model * d_model x 1 -> rank x 1)
        for r in 0..self.lora_rank {
            let mut sum = 0.0f32;
            for i in 0..self.d_model {
                sum += self.lora_a[r * self.d_model + i] * input[i];
            }
            intermediate[r] = sum;
        }

        // B * intermediate (d_model x rank * rank x 1 -> d_model x 1)
        let mut output = *input;
        for i in 0..self.d_model {
            let mut sum = 0.0f32;
            for r in 0..self.lora_rank {
                sum += self.lora_b[i * self.lora_rank + r] * intermediate[r];
            }
            output[i] += sum;
        }

        output
    }

    /// Serializes current LoRA delta into contiguous bytes for Block 2 storage
    pub fn export_delta_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity((self.lora_a.len() + self.lora_b.len()) * 4);
        for &val in &self.lora_a {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        for &val in &self.lora_b {
            bytes.extend_from_slice(&val.to_le_bytes());
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_lora_adaptation_and_ogp_invariance() {
        let mut pipeline = StreamingLoraAdaptationPipeline::new(LATENT_DIM, 16, 0.01);

        // 1. Register a protected core task gradient vector along axis 0
        let mut core_grad = [0.0f32; LATENT_DIM];
        core_grad[0] = 1.0;
        pipeline.register_protected_core_task(&core_grad).unwrap();
        assert_eq!(pipeline.protected_core_subspace.len(), 1);

        // 2. An update that points purely along axis 0 should be fully projected out (norm = 0)
        let mut target = [0.0f32; LATENT_DIM];
        target[0] = 5.0;
        let pred = [0.0f32; LATENT_DIM];

        let report = pipeline.update_from_prediction_error(&target, &pred);
        assert_eq!(report.projected_gradient_norm, 0.0); // OGP completely protects axis 0

        // 3. An update along axis 1 (orthogonal) should pass through untouched
        let mut target2 = [0.0f32; LATENT_DIM];
        target2[1] = 2.0;
        let report2 = pipeline.update_from_prediction_error(&target2, &pred);
        assert!(report2.projected_gradient_norm > 1.9);

        // 4. Test forward evaluation
        let input = [1.0f32; LATENT_DIM];
        let out = pipeline.forward(&input);
        assert_eq!(out.len(), LATENT_DIM);
    }
}
