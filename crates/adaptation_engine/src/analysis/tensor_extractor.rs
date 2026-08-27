use anyhow::Result;
use candle_core::{Device, Tensor};

pub struct TensorExtractor {
    device: Device,
}

impl TensorExtractor {
    pub fn new() -> Result<Self> {
        let device = Device::Cpu; // Lightweight CPU-first processing
        Ok(Self { device })
    }

    pub fn extract_features(&self, raw_data: &[f32], shape: (usize, usize)) -> Result<Tensor> {
        let t = Tensor::from_slice(raw_data, shape, &self.device)?;
        Ok(t)
    }

    pub fn compute_similarity(&self, a: &Tensor, b: &Tensor) -> Result<f32> {
        // Simple cosine similarity
        let sum_ab = (a * b)?.sum_all()?.to_scalar::<f32>()?;
        let norm_a = (a * a)?.sum_all()?.sqrt()?.to_scalar::<f32>()?;
        let norm_b = (b * b)?.sum_all()?.sqrt()?.to_scalar::<f32>()?;

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(sum_ab / (norm_a * norm_b))
    }
}
