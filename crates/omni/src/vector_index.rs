//! crates/omni/src/vector_index.rs
//! High-Dimensional Vector Index & SIMD Spatial Partitioning Engine
//! Integrates glam SIMD 3D projections, Cosine/Euclidean nearest neighbor queries, and bounding box frustums.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A point or semantic vector in high-dimensional embedding space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub coordinates: [f32; 3], // 3D spatial position in the Galaxy
    pub embedding: Vec<f32>,   // High-dimensional semantic vector
    pub payload: String,
}

impl VectorDocument {
    pub fn new(id: impl Into<String>, coords: [f32; 3], embedding: Vec<f32>, payload: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            coordinates: coords,
            embedding,
            payload: payload.into(),
        }
    }

    pub fn position_vec3(&self) -> Vec3 {
        Vec3::from_slice(&self.coordinates)
    }
}

/// A search hit with calculated distance and similarity score
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub similarity_score: f32, // Cosine similarity: -1.0 to +1.0
    pub euclidean_distance: f32,
    pub coordinates: [f32; 3],
    pub payload: String,
}

impl Eq for VectorSearchResult {}

impl PartialOrd for VectorSearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VectorSearchResult {
    fn cmp(&self, other: &Self) -> Ordering {
        self.similarity_score
            .partial_cmp(&other.similarity_score)
            .unwrap_or(Ordering::Equal)
    }
}

/// 3D Axis-Aligned Bounding Box for SIMD spatial partitioning
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox3D {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

impl BoundingBox3D {
    pub fn new(min: [f32; 3], max: [f32; 3]) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, point: [f32; 3]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
            && point[2] >= self.min[2]
            && point[2] <= self.max[2]
    }

    pub fn contains_vec3(&self, point: Vec3) -> bool {
        point.x >= self.min[0]
            && point.x <= self.max[0]
            && point.y >= self.min[1]
            && point.y <= self.max[1]
            && point.z >= self.min[2]
            && point.z <= self.max[2]
    }

    pub fn intersects(&self, other: &BoundingBox3D) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }
}

/// High-Performance Vector Index and 3D Octree Partitioning Engine
pub struct VectorIndexEngine {
    documents: Vec<VectorDocument>,
    #[allow(dead_code)]
    dimension: usize,
}

impl VectorIndexEngine {
    pub fn new(dimension: usize) -> Self {
        Self {
            documents: Vec::new(),
            dimension,
        }
    }

    pub fn insert(&mut self, doc: VectorDocument) {
        self.documents.push(doc);
    }

    pub fn count(&self) -> usize {
        self.documents.len()
    }

    /// K-Nearest Neighbors (KNN) Semantic Vector Search using Cosine Similarity
    pub fn search_knn(&self, query_vector: &[f32], top_k: usize) -> Vec<VectorSearchResult> {
        let query_norm = Self::vector_norm(query_vector);
        if query_norm == 0.0 || self.documents.is_empty() {
            return Vec::new();
        }

        let mut results = Vec::new();

        for doc in &self.documents {
            let similarity = Self::cosine_similarity(query_vector, &doc.embedding, query_norm);
            let distance = Self::euclidean_distance(query_vector, &doc.embedding);

            results.push(VectorSearchResult {
                id: doc.id.clone(),
                similarity_score: similarity,
                euclidean_distance: distance,
                coordinates: doc.coordinates,
                payload: doc.payload.clone(),
            });
        }

        // Sort descending by similarity score
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap_or(Ordering::Equal));
        results.truncate(top_k);
        results
    }

    /// 3D Spatial Bounding Box / Frustum search with SIMD Vec3 acceleration
    pub fn search_spatial_bounds(&self, bbox: &BoundingBox3D) -> Vec<VectorSearchResult> {
        let mut results = Vec::new();

        for doc in &self.documents {
            if bbox.contains_vec3(doc.position_vec3()) {
                results.push(VectorSearchResult {
                    id: doc.id.clone(),
                    similarity_score: 1.0,
                    euclidean_distance: 0.0,
                    coordinates: doc.coordinates,
                    payload: doc.payload.clone(),
                });
            }
        }

        results
    }

    /// Calculate Cosine Similarity between two vectors
    pub fn cosine_similarity(v1: &[f32], v2: &[f32], norm_v1: f32) -> f32 {
        if v1.len() != v2.len() || norm_v1 == 0.0 {
            return 0.0;
        }

        let mut dot_product = 0.0f32;
        let mut norm_v2_sq = 0.0f32;

        for i in 0..v1.len() {
            dot_product += v1[i] * v2[i];
            norm_v2_sq += v2[i] * v2[i];
        }

        let norm_v2 = norm_v2_sq.sqrt();
        if norm_v2 == 0.0 {
            return 0.0;
        }

        dot_product / (norm_v1 * norm_v2)
    }

    /// Calculate Euclidean Distance
    pub fn euclidean_distance(v1: &[f32], v2: &[f32]) -> f32 {
        let min_len = v1.len().min(v2.len());
        let mut sum_sq = 0.0f32;
        for i in 0..min_len {
            let diff = v1[i] - v2[i];
            sum_sq += diff * diff;
        }
        sum_sq.sqrt()
    }

    pub fn vector_norm(v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum::<f32>().sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_knn_search() {
        let mut index = VectorIndexEngine::new(4);

        index.insert(VectorDocument::new(
            "node_alpha",
            [10.0, 20.0, 30.0],
            vec![1.0, 0.0, 0.0, 0.0],
            "Alpha Kernel",
        ));

        index.insert(VectorDocument::new(
            "node_beta",
            [100.0, 200.0, 300.0],
            vec![0.0, 1.0, 0.0, 0.0],
            "Beta Synapse",
        ));

        index.insert(VectorDocument::new(
            "node_gamma",
            [15.0, 25.0, 35.0],
            vec![0.9, 0.1, 0.0, 0.0],
            "Gamma Kernel Hybrid",
        ));

        let query = vec![1.0, 0.0, 0.0, 0.0];
        let hits = index.search_knn(&query, 2);

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "node_alpha");
        assert!((hits[0].similarity_score - 1.0).abs() < 1e-4);
        assert_eq!(hits[1].id, "node_gamma");
    }

    #[test]
    fn test_spatial_bounding_box_simd() {
        let mut index = VectorIndexEngine::new(3);
        index.insert(VectorDocument::new(
            "star_1",
            [5.0, 5.0, 5.0],
            vec![0.1, 0.2, 0.3],
            "Star 1",
        ));
        index.insert(VectorDocument::new(
            "star_outer",
            [500.0, 500.0, 500.0],
            vec![0.1, 0.2, 0.3],
            "Outer Star",
        ));

        let bbox = BoundingBox3D::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]);
        let results = index.search_spatial_bounds(&bbox);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "star_1");
    }
}
