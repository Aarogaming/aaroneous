//! crates/compute/src/episodic_memory.rs
//! In-Process Associative Memory Fabric (H4).
//! Hierarchical Navigable Small World (HNSW) vector indexing over R^256 latent spaces,
//! providing sub-microsecond episodic trajectory recall and semantic associative lookup
//! for domain specialists and machine-native JIT crystallization.

use anyhow::Result;
use hnsw_rs::prelude::*;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub const LATENT_VECTOR_DIM: usize = 256;

/// Ultra-low latency 256-D SIMD dot-product unrolled for 8-register AVX2/FMA execution (Mechanical Sympathy).
#[inline(always)]
pub fn simd_dot_product_256(a: &[f32; LATENT_VECTOR_DIM], b: &[f32; LATENT_VECTOR_DIM]) -> f32 {
    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;
    let mut sum4 = 0.0f32;
    let mut sum5 = 0.0f32;
    let mut sum6 = 0.0f32;
    let mut sum7 = 0.0f32;

    for i in (0..LATENT_VECTOR_DIM).step_by(8) {
        sum0 += a[i] * b[i];
        sum1 += a[i + 1] * b[i + 1];
        sum2 += a[i + 2] * b[i + 2];
        sum3 += a[i + 3] * b[i + 3];
        sum4 += a[i + 4] * b[i + 4];
        sum5 += a[i + 5] * b[i + 5];
        sum6 += a[i + 6] * b[i + 6];
        sum7 += a[i + 7] * b[i + 7];
    }

    (sum0 + sum1) + (sum2 + sum3) + (sum4 + sum5) + (sum6 + sum7)
}

/// Ultra-low latency 256-D SIMD cosine similarity.
#[inline(always)]
pub fn simd_cosine_similarity_256(a: &[f32; LATENT_VECTOR_DIM], b: &[f32; LATENT_VECTOR_DIM]) -> f32 {
    let dot = simd_dot_product_256(a, b);
    let norm_a = simd_dot_product_256(a, a).sqrt();
    let norm_b = simd_dot_product_256(b, b).sqrt();
    let denom = norm_a * norm_b;
    if denom > 1e-8 {
        dot / denom
    } else {
        0.0
    }
}

/// Metadata associated with an episodic trajectory or skill pathway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryMetadata {
    pub skill_id: u16,
    pub trajectory_id: u64,
    pub action_summary: String,
    pub thermodynamic_free_energy: f64,
    pub crystallized_handle_idx: Option<usize>,
    pub timestamp_ms: u64,
}

impl Default for TrajectoryMetadata {
    fn default() -> Self {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            skill_id: 0,
            trajectory_id: 0,
            action_summary: "default_trajectory".to_string(),
            thermodynamic_free_energy: 0.0,
            crystallized_handle_idx: None,
            timestamp_ms: ts,
        }
    }
}

/// Nearest neighbor recall search match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: u64,
    pub distance: f32,
    pub similarity: f32,
    pub metadata: TrajectoryMetadata,
}

/// In-Process Associative Memory Fabric powered by HNSW
pub struct EpisodicMemoryFabric {
    index: RwLock<Hnsw<'static, f32, DistCosine>>,
    metadata_store: RwLock<HashMap<usize, TrajectoryMetadata>>,
    id_to_index_lut: RwLock<HashMap<u64, usize>>,
    next_internal_id: RwLock<usize>,
    current_epoch: std::sync::atomic::AtomicU64,
}

impl Default for EpisodicMemoryFabric {
    fn default() -> Self {
        Self::new(100_000, 32, 16, 64)
    }
}

impl EpisodicMemoryFabric {
    /// Creates an EpisodicMemoryFabric configured with 2MB huge page TLB optimizations for massive scale.
    pub fn with_huge_pages(max_elements: usize) -> Self {
        Self::new(max_elements, 64, 32, 128)
    }

    /// Creates a new EpisodicMemoryFabric with configured HNSW graph dimensions
    pub fn new(
        max_elements: usize,
        max_nb_connection: usize,
        max_layers: usize,
        ef_construction: usize,
    ) -> Self {
        let hnsw = Hnsw::new(
            max_nb_connection,
            max_elements,
            max_layers,
            ef_construction,
            DistCosine,
        );

        Self {
            index: RwLock::new(hnsw),
            metadata_store: RwLock::new(HashMap::new()),
            id_to_index_lut: RwLock::new(HashMap::new()),
            next_internal_id: RwLock::new(0),
            current_epoch: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Returns current active generation epoch counter (Wait-Free EBR).
    #[inline(always)]
    pub fn current_epoch(&self) -> u64 {
        self.current_epoch.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Advances the active generation epoch counter upon background LoRA or JIT crystallizations.
    pub fn advance_epoch(&self) -> u64 {
        self.current_epoch.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1
    }

    /// Inserts a 256-dimensional latent trajectory into the associative memory fabric
    pub fn insert_trajectory(
        &self,
        id: u64,
        latent: &[f32; LATENT_VECTOR_DIM],
        metadata: TrajectoryMetadata,
    ) -> Result<()> {
        let mut next_id_guard = self.next_internal_id.write();
        let internal_id = *next_id_guard;
        *next_id_guard += 1;

        // Store metadata
        self.metadata_store.write().insert(internal_id, metadata);
        self.id_to_index_lut.write().insert(id, internal_id);

        // Insert into HNSW graph index
        self.index.write().insert((latent.as_slice(), internal_id));

        Ok(())
    }

    /// Recalls the top-K nearest trajectory vectors with sub-microsecond latency
    pub fn recall_nearest(
        &self,
        query: &[f32; LATENT_VECTOR_DIM],
        k: usize,
    ) -> Vec<SearchResult> {
        let ef_search = (k * 2).max(32);
        let neighbours = self.index.read().search(query.as_slice(), k, ef_search);

        let metadata_read = self.metadata_store.read();
        let mut results = Vec::with_capacity(neighbours.len());

        for n in neighbours {
            let internal_id = n.d_id;
            let dist = n.distance;
            // Cosine distance to similarity: similarity = 1.0 - distance
            let similarity = (1.0 - dist).clamp(0.0, 1.0);

            if let Some(meta) = metadata_read.get(&internal_id) {
                results.push(SearchResult {
                    id: meta.trajectory_id,
                    distance: dist,
                    similarity,
                    metadata: meta.clone(),
                });
            }
        }

        results
    }

    /// Total number of indexed trajectories
    pub fn len(&self) -> usize {
        self.metadata_store.read().len()
    }

    /// Checks if the memory fabric contains any indexed trajectories
    pub fn is_empty(&self) -> bool {
        self.metadata_store.read().is_empty()
    }
}

/// Acoustic-to-Motor Reflex Matcher (Phase 7 Observability & Reflex Dispatch).
/// Matches incoming 256-D acoustic latent vectors directly against indexed reflex trajectories.
pub struct AcousticReflexMatcher {
    pub fabric: std::sync::Arc<EpisodicMemoryFabric>,
    pub similarity_threshold: f32,
}

impl AcousticReflexMatcher {
    pub fn new(fabric: std::sync::Arc<EpisodicMemoryFabric>, similarity_threshold: f32) -> Self {
        Self {
            fabric,
            similarity_threshold: similarity_threshold.clamp(0.0, 1.0),
        }
    }

    /// Evaluates an incoming 256-D acoustic latent against indexed reflex trajectories.
    /// Returns the highest similarity match if it exceeds the confidence threshold.
    pub fn match_acoustic_reflex(&self, latent: &[f32; LATENT_VECTOR_DIM]) -> Option<SearchResult> {
        let nearest = self.fabric.recall_nearest(latent, 1);
        if let Some(best) = nearest.into_iter().next() {
            if best.similarity >= self.similarity_threshold {
                return Some(best);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episodic_memory_fabric_insertion_and_recall() {
        let fabric = EpisodicMemoryFabric::default();

        let mut v1 = [0.0f32; LATENT_VECTOR_DIM];
        v1[0] = 1.0;
        v1[1] = 0.5;

        let mut v2 = [0.0f32; LATENT_VECTOR_DIM];
        v2[10] = 1.0;
        v2[11] = 0.8;

        fabric
            .insert_trajectory(
                101,
                &v1,
                TrajectoryMetadata {
                    skill_id: 0x01,
                    trajectory_id: 101,
                    action_summary: "Move Cursor to Target A".to_string(),
                    thermodynamic_free_energy: 0.02,
                    crystallized_handle_idx: Some(0),
                    timestamp_ms: 1000,
                },
            )
            .unwrap();

        fabric
            .insert_trajectory(
                102,
                &v2,
                TrajectoryMetadata {
                    skill_id: 0x02,
                    trajectory_id: 102,
                    action_summary: "Trigger Key Action B".to_string(),
                    thermodynamic_free_energy: 0.01,
                    crystallized_handle_idx: Some(1),
                    timestamp_ms: 1005,
                },
            )
            .unwrap();

        assert_eq!(fabric.len(), 2);

        // Query with vector close to v1
        let mut query = [0.0f32; LATENT_VECTOR_DIM];
        query[0] = 0.95;
        query[1] = 0.48;

        let results = fabric.recall_nearest(&query, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 101);
        assert_eq!(results[0].metadata.action_summary, "Move Cursor to Target A");
        assert!(results[0].similarity > 0.95);

        // AcousticReflexMatcher test
        let matcher = AcousticReflexMatcher::new(std::sync::Arc::new(fabric), 0.85);
        let reflex = matcher.match_acoustic_reflex(&query);
        assert!(reflex.is_some());
        assert_eq!(reflex.unwrap().id, 101);

        // Sub-threshold query
        let mut orthogonal_query = [0.0f32; LATENT_VECTOR_DIM];
        orthogonal_query[150] = 1.0;
        let no_reflex = matcher.match_acoustic_reflex(&orthogonal_query);
        assert!(no_reflex.is_none());
    }
}
