use compute::episodic_memory::{EpisodicMemoryFabric, TrajectoryMetadata, LATENT_VECTOR_DIM};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;

/// SEMANTIC-04: EpisodicMemoryFabric Vector Insertion Pipeline
/// 
/// Takes incoming streams of user interactions and parsed intents, computes a latent
/// vector embedding, and inserts it directly into the high-speed HNSW R^256 fabric.
pub struct EpisodicInsertionPipeline {
    fabric: Arc<EpisodicMemoryFabric>,
}

impl EpisodicInsertionPipeline {
    pub fn new(fabric: Arc<EpisodicMemoryFabric>) -> Self {
        Self { fabric }
    }

    /// Encodes a raw string into a deterministic latent vector and writes it to memory
    pub fn embed_and_insert(&self, raw_text: &str, context_tag: &str) -> Result<()> {
        // Fast zero-copy embedding simulation (until LLaVA/Embedding model is loaded)
        let mut latent = [0.0f32; LATENT_VECTOR_DIM];
        for (i, b) in raw_text.bytes().enumerate().take(LATENT_VECTOR_DIM) {
            latent[i] = (b as f32) / 255.0;
        }

        let metadata = TrajectoryMetadata {
            skill_id: 1,
            trajectory_id: SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64,
            action_summary: format!("context://{}: {}", context_tag, raw_text.chars().take(64).collect::<String>()),
            thermodynamic_free_energy: 1.0,
            crystallized_handle_idx: None,
            timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
        };

        let id = metadata.trajectory_id;
        self.fabric.insert_trajectory(id, &latent, metadata)
    }
}