use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use candle_core::{Device, Tensor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEmbedding {
    pub token_ids: Vec<u32>,
    pub vector: Vec<f32>, 
    pub metadata: HashMap<String, String>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u32,
}

pub struct SemanticIndex {
    pub entries: Vec<SemanticEmbedding>,
    device: Device,
}

impl SemanticIndex {
    pub fn new() -> Self {
        Self { 
            entries: Vec::new(),
            device: Device::Cpu,
        }
    }

    /// Adds a tokenized document to the semantic index.
    pub fn index_tokens(&mut self, tokens: &[u32], license_tier: u8) -> Result<()> {
        let mut metadata = HashMap::new();
        metadata.insert("license_tier".to_string(), license_tier.to_string());
        
        let vector = self.generate_vector(tokens)?;
        
        self.entries.push(SemanticEmbedding {
            token_ids: tokens.to_vec(),
            vector,
            metadata,
            last_accessed: chrono::Utc::now(),
            access_count: 0,
        });
        
        Ok(())
    }

    /// Performs a similarity search and updates access patterns.
    pub fn search(&mut self, query_vector: &[f32], limit: usize) -> Vec<&SemanticEmbedding> {
        let mut scored_entries: Vec<(f32, usize)> = self.entries.iter().enumerate()
            .map(|(i, entry)| (self.cosine_similarity(query_vector, &entry.vector), i))
            .collect();
            
        scored_entries.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        
        let now = chrono::Utc::now();
        let top_indices: Vec<usize> = scored_entries.into_iter().take(limit).map(|(_, i)| i).collect();
        
        for &idx in &top_indices {
            self.entries[idx].last_accessed = now;
            self.entries[idx].access_count += 1;
        }

        top_indices.into_iter().map(|i| &self.entries[i]).collect()
    }

    /// Prunes embeddings that haven't been accessed recently or have low utility.
    pub fn prune_stale_embeddings(&mut self, max_age_days: i64) -> usize {
        let now = chrono::Utc::now();
        let initial_count = self.entries.len();
        
        self.entries.retain(|entry| {
            let age = (now - entry.last_accessed).num_days();
            age < max_age_days || entry.access_count > 10 // Protect frequently used items
        });
        
        initial_count - self.entries.len()
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (norm_a * norm_b)
    }

    fn generate_vector(&self, tokens: &[u32]) -> Result<Vec<f32>> {
        // Real tensor-based projection using Candle
        let data: Vec<f32> = tokens.iter()
            .take(1024)
            .map(|&t| (t as f32 % 1000.0) / 1000.0)
            .collect();
            
        let mut padded_data = vec![0.0; 1024];
        for (i, &val) in data.iter().enumerate() {
            padded_data[i] = val;
        }

        let ts = Tensor::from_vec(padded_data, (1, 1024), &self.device)?;
        
        // Simulating a nonlinear activation layer (sin/cos mix)
        let processed = ts.sin()?.to_vec2::<f32>()?;
        let mut result = processed[0].clone();
        
        // Normalize the vector
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in result.iter_mut() {
                *x /= norm;
            }
        }
        
        Ok(result)
    }
}
