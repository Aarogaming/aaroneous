use anyhow::{Result, anyhow};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::LazyLock;

use candle_core::{Device, Tensor};

pub const EMBED_DIM: usize = 384;
const VOCAB_HASH_SIZE: usize = 4096;

/// Fixed random projection matrix: maps 4096 hash-bucket features → 384-dim embedding.
/// Seeded for determinism — same text always produces the same vector.
static PROJECTION_MATRIX: LazyLock<Vec<f32>> = LazyLock::new(|| {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut data = Vec::with_capacity(VOCAB_HASH_SIZE * EMBED_DIM);
    for _ in 0..VOCAB_HASH_SIZE * EMBED_DIM {
        let val: f32 = rand::Rng::sample(&mut rng, rand::distributions::Uniform::new(-0.1f32, 0.1));
        data.push(val);
    }
    data
});

/// Generates a deterministic 384-dim embedding vector from text using candle-core tensor projection.
/// Uses a seeded random projection matrix for semantic feature hashing.
pub fn embed_text(text: &str, device: &Device) -> Result<Vec<f32>> {
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return Ok(vec![0.0f32; EMBED_DIM]);
    }

    // Build hash-bag feature vector
    let mut features = vec![0.0f32; VOCAB_HASH_SIZE];
    for &b in bytes {
        let idx = (b as usize) % VOCAB_HASH_SIZE;
        features[idx] += 1.0;
    }
    // Normalize feature vector
    let feat_norm: f32 = features.iter().map(|x| x * x).sum::<f32>().sqrt();
    if feat_norm > 0.0 {
        for x in features.iter_mut() {
            *x /= feat_norm;
        }
    }

    // Project via candle-core tensor: (1, HASH_SIZE) × (HASH_SIZE, EMBED_DIM) → (1, EMBED_DIM)
    let feat_t = Tensor::from_vec(features, (1, VOCAB_HASH_SIZE), device)
        .map_err(|e| anyhow!("Failed to create feature tensor: {}", e))?;
    let proj_t = Tensor::from_vec(PROJECTION_MATRIX.clone(), (VOCAB_HASH_SIZE, EMBED_DIM), device)
        .map_err(|e| anyhow!("Failed to create projection tensor: {}", e))?;
    let result_t = feat_t.matmul(&proj_t)
        .map_err(|e| anyhow!("Failed to project features: {}", e))?;
    let vec = result_t.squeeze(0)
        .map_err(|e| anyhow!("Failed to squeeze result: {}", e))?
        .to_vec1::<f32>()
        .map_err(|e| anyhow!("Failed to convert to vec: {}", e))?;

    // Unit-normalize
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        Ok(vec.iter().map(|x| x / norm).collect())
    } else {
        Ok(vec)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEmbedding {
    pub text: String,
    pub vector: Vec<f32>, 
    pub metadata: HashMap<String, String>,
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    pub access_count: u32,
    pub id: String,
}

impl SemanticEmbedding {
    pub fn new(text: &str, metadata: HashMap<String, String>, device: &Device) -> Self {
        let vector = embed_text(text, device).unwrap_or_else(|_| {
            let mut v = vec![0.0f32; EMBED_DIM];
            if !text.is_empty() { v[0] = 1.0; }
            v
        });
        Self {
            text: text.to_string(),
            vector,
            metadata,
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            id: uuid::Uuid::new_v4().to_string(),
        }
    }
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

    /// Adds text directly to the semantic index with automatic embedding.
    pub fn index_text(&mut self, text: &str, metadata: HashMap<String, String>) -> String {
        let embedding = SemanticEmbedding::new(text, metadata, &self.device);
        let id = embedding.id.clone();
        self.entries.push(embedding);
        id
    }

    /// Performs a similarity search against stored embeddings.
    pub fn search(&mut self, query: &str, limit: usize) -> Vec<&SemanticEmbedding> {
        let query_vector = match embed_text(query, &self.device) {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };
        let mut scored_entries: Vec<(f32, usize)> = self.entries.iter().enumerate()
            .map(|(i, entry)| (self.cosine_similarity(&query_vector, &entry.vector), i))
            .collect();
            
        scored_entries.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        let now = chrono::Utc::now();
        let top_indices: Vec<usize> = scored_entries.into_iter().take(limit).map(|(_, i)| i).collect();
        
        for &idx in &top_indices {
            self.entries[idx].last_accessed = now;
            self.entries[idx].access_count += 1;
        }

        top_indices.into_iter().map(|i| &self.entries[i]).collect()
    }

    /// Remove an entry by ID and return it.
    pub fn remove(&mut self, id: &str) -> Option<SemanticEmbedding> {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    /// Prunes embeddings that haven't been accessed recently.
    pub fn prune_stale_embeddings(&mut self, max_age_days: i64) -> usize {
        let now = chrono::Utc::now();
        let initial_count = self.entries.len();
        
        self.entries.retain(|entry| {
            let age = (now - entry.last_accessed).num_days();
            age < max_age_days || entry.access_count > 10
        });
        
        initial_count - self.entries.len()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a * norm_b == 0.0 { 0.0 } else { dot_product / (norm_a * norm_b) }
    }
}
