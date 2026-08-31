//! crates/omni
//! Universal 3D Galaxy semantic data navigation, star-node clustering, and visual search engine for Aaroneous.

pub mod ecs_galaxy;
pub mod galaxy_cluster;
pub mod matrix;
pub mod protocol_bridge;
pub mod query_engine;
pub mod spatial_coord;
pub mod star_node;
pub mod vector_index;

pub use ecs_galaxy::{
    ConstellationLinks, EcsGalaxyEngine, MetabolicEnergy, SemanticEmbedding, SpatialTransform,
};
pub use galaxy_cluster::{GalacticClusteringEngine, GalaxyCluster};
pub use matrix::{
    compute_information_flow, compute_surface_importance, find_redundant_surfaces,
    rate_distortion_analysis, spectral_clustering, SabEmbedding, SabManifest, SabMatrix,
    SabMatrixBuilder, SabMetadata, SabSimilarityMatrix, SabSurface,
};
pub use protocol_bridge::{OmniGalaxySnapshot, OmniProtocolBridge};
pub use query_engine::{OmniQueryEngine, OmniQueryFilter, SpatialFrustum};
pub use spatial_coord::SpatialCoord;
pub use star_node::{LinkType, Priority, StarNode, StarNodeStatus, StarNodeType};
pub use vector_index::{BoundingBox3D, VectorDocument, VectorIndexEngine, VectorSearchResult};

// Backward compatibility type aliases for legacy Constellation callers
pub type Constellation = OmniEngine;
pub type ConstellationNode = StarNode;
pub type NodeType = StarNodeType;
pub type NodeStatus = StarNodeStatus;
pub type RelationshipType = LinkType;
pub type ConstellationQuery = OmniQueryFilter;
pub type ClusteringContext = GalacticClusteringEngine;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Master Omni Engine managing the 3D Galaxy Star-Graph
pub struct OmniEngine {
    nodes: Arc<RwLock<HashMap<String, StarNode>>>,
    galaxies: Arc<RwLock<Vec<GalaxyCluster>>>,
    clustering_engine: GalacticClusteringEngine,
}

impl Default for OmniEngine {
    fn default() -> Self {
        Self::new(200.0)
    }
}

impl OmniEngine {
    pub fn new(clustering_threshold: f64) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            galaxies: Arc::new(RwLock::new(Vec::new())),
            clustering_engine: GalacticClusteringEngine::new(clustering_threshold),
        }
    }

    /// Ingest the 9 Specialists into the 3D Omni Galaxy
    pub async fn ingest_standard_specialists(&self) -> usize {
        let specialists = [
            ("orchestrator", "Orchestrator (Strategic Cortex)", -800.0, 500.0, 950.0, 0x0100),
            ("synthesizer", "Synthesizer (Semantic Knowledge)", -600.0, 200.0, 800.0, 0x0200),
            ("presenter", "Presenter (Visual Experience)", -400.0, 400.0, 750.0, 0x0300),
            ("fabricator", "Fabricator (Hardware & Forge)", 200.0, 300.0, 900.0, 0x0400),
            ("sentinel", "Sentinel (Auditor & Safety)", 0.0, 600.0, 1000.0, 0x0500),
            ("archivist", "Archivist (Chaos & Resilience)", 400.0, -200.0, 600.0, 0x0600),
            ("router", "Router (Router & Mesh)", 100.0, 100.0, 850.0, 0x0700),
            ("aligner", "Aligner (Temporal Resonance)", -200.0, -400.0, 700.0, 0x0800),
            ("perceiver", "Perceiver (Sensory Threshold)", 800.0, 300.0, 900.0, 0x0900),
        ];

        let mut count = 0;
        for (id, title, x, y, z, opcode) in specialists {
            let mut latent = [0.0f32; 32];
            // Synthetic latent signature based on opcode
            for (i, val) in latent.iter_mut().enumerate() {
                *val = ((opcode as f32 * 0.1 + i as f32 * 0.3).sin() + 1.0) * 0.5;
            }

            let mut node = StarNode::new(
                id,
                title,
                StarNodeType::Specialist,
                "Specialists",
                SpatialCoord::new(x, y, z),
                &format!("federation://{}", id),
            )
            .with_latent(latent)
            .with_status(StarNodeStatus::Completed)
            .with_priority(Priority::Critical);

            node.pulse(0.9);
            self.insert_node(node).await;
            count += 1;
        }

        // Link core orchestrations
        {
            let mut nodes = self.nodes.write().await;
            if let Some(orchestrator) = nodes.get_mut("orchestrator") {
                orchestrator.link_to("router", LinkType::Synthesizes);
                orchestrator.link_to("sentinel", LinkType::DependsOn);
            }
            if let Some(router) = nodes.get_mut("router") {
                router.link_to("perceiver", LinkType::RelatesTo);
                router.link_to("fabricator", LinkType::RelatesTo);
            }
        }

        count
    }

    /// Ingest workspace crates as architectural Star-Nodes
    pub async fn ingest_workspace_crates(&self, crates: &[&str]) -> usize {
        let mut count = 0;
        for (i, name) in crates.iter().enumerate() {
            let angle = (i as f64 / crates.len() as f64) * 2.0 * std::f64::consts::PI;
            let radius = 350.0;
            let x = radius * angle.cos();
            let y = radius * angle.sin();
            let z = 500.0;

            let mut latent = [0.0f32; 32];
            for (j, val) in latent.iter_mut().enumerate() {
                *val = ((i as f32 * 0.2 + j as f32 * 0.4).cos() + 1.0) * 0.5;
            }

            let node = StarNode::new(
                name,
                &format!("Crate: {}", name),
                StarNodeType::Architecture,
                "Workspace",
                SpatialCoord::new(x, y, z),
                &format!("crate://{}", name),
            )
            .with_latent(latent)
            .with_status(StarNodeStatus::Completed)
            .with_priority(Priority::High);

            self.insert_node(node).await;
            count += 1;
        }
        count
    }

    /// Step N-body gravitational force relaxation simulation
    pub async fn step_gravitational_physics(&self, dt: f64) {
        let mut nodes = self.nodes.write().await;
        let node_ids: Vec<String> = nodes.keys().cloned().collect();
        let n = node_ids.len();
        if n < 2 {
            return;
        }

        let mut forces: HashMap<String, (f64, f64, f64)> = HashMap::new();
        for id in &node_ids {
            forces.insert(id.clone(), (0.0, 0.0, 0.0));
        }

        // 1. Pairwise N-body repulsion + semantic attraction
        for i in 0..n {
            for j in (i + 1)..n {
                let id_a = &node_ids[i];
                let id_b = &node_ids[j];

                let (pos_a, latent_a, links_a) = {
                    let a = &nodes[id_a];
                    (a.spatial_coord, a.latent_vector, a.links.clone())
                };
                let (pos_b, latent_b) = {
                    let b = &nodes[id_b];
                    (b.spatial_coord, b.latent_vector)
                };

                let dx = pos_b.x - pos_a.x;
                let dy = pos_b.y - pos_a.y;
                let dz = pos_b.z - pos_a.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(10.0);

                // Coulomb repulsion
                let k_rep = 50000.0;
                let f_rep = k_rep / (dist * dist);
                let fx_rep = f_rep * (dx / dist);
                let fy_rep = f_rep * (dy / dist);
                let fz_rep = f_rep * (dz / dist);

                // Semantic & link attraction
                let is_linked = links_a.contains_key(id_b);
                let sem_sim = if let (Some(la), Some(lb)) = (latent_a, latent_b) {
                    let mut dot = 0.0f32;
                    let mut na = 0.0f32;
                    let mut nb = 0.0f32;
                    for k in 0..32 {
                        dot += la[k] * lb[k];
                        na += la[k] * la[k];
                        nb += lb[k] * lb[k];
                    }
                    if na > 1e-6 && nb > 1e-6 { dot / (na.sqrt() * nb.sqrt()) } else { 0.0 }
                } else {
                    0.0
                };

                let k_att = if is_linked { 0.08 } else { 0.02 * sem_sim.max(0.0) as f64 };
                let fx_att = k_att * dx;
                let fy_att = k_att * dy;
                let fz_att = k_att * dz;

                let total_fx = fx_att - fx_rep;
                let total_fy = fy_att - fy_rep;
                let total_fz = fz_att - fz_rep;

                if let Some(f) = forces.get_mut(id_a) {
                    f.0 += total_fx;
                    f.1 += total_fy;
                    f.2 += total_fz;
                }
                if let Some(f) = forces.get_mut(id_b) {
                    f.0 -= total_fx;
                    f.1 -= total_fy;
                    f.2 -= total_fz;
                }
            }
        }

        // 2. Apply forces to update positions
        for (id, (fx, fy, fz)) in forces {
            if let Some(node) = nodes.get_mut(&id) {
                node.spatial_coord.x += fx * dt;
                node.spatial_coord.y += fy * dt;
                node.spatial_coord.z += fz * dt;
            }
        }

        // 3. Recompute galactic clusters
        let updated_galaxies = self.clustering_engine.compute_galaxies(&nodes);
        let mut galaxies = self.galaxies.write().await;
        *galaxies = updated_galaxies;
    }

    /// Search nearest star-nodes by 32-dim latent embedding cosine similarity
    pub async fn search_semantic(&self, query_latent: &[f32; 32], top_k: usize) -> Vec<(StarNode, f32)> {
        let nodes = self.nodes.read().await;
        let mut scored: Vec<(StarNode, f32)> = nodes
            .values()
            .map(|node| {
                let score = node.cosine_similarity(query_latent);
                (node.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Insert or update a star-node in the galaxy
    pub async fn insert_node(&self, node: StarNode) {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.id.clone(), node);
        let updated_galaxies = self.clustering_engine.compute_galaxies(&nodes);
        let mut galaxies = self.galaxies.write().await;
        *galaxies = updated_galaxies;
    }

    /// Query the galaxy with spatial frustum and multi-dimensional filters
    pub async fn query(&self, filter: &OmniQueryFilter) -> Vec<StarNode> {
        let nodes = self.nodes.read().await;
        OmniQueryEngine::query(&nodes, filter).into_iter().cloned().collect()
    }

    /// Export a full snapshot of the Omni Galaxy
    pub async fn export_snapshot(&self) -> Result<OmniGalaxySnapshot> {
        let nodes = self.nodes.read().await;
        let galaxies = self.galaxies.read().await;
        Ok(OmniProtocolBridge::create_snapshot(&nodes, &galaxies))
    }

    /// Returns all registered star nodes in the galaxy
    pub async fn get_all_nodes(&self) -> HashMap<String, StarNode> {
        self.nodes.read().await.clone()
    }

    /// Returns the total number of registered star nodes
    pub async fn total_stars(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Returns the list of discovered galaxy clusters
    pub async fn get_galaxy_clusters(&self) -> Vec<GalaxyCluster> {
        self.galaxies.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_omni_engine_e2e() {
        let engine = OmniEngine::default();
        let star1 = StarNode::new("star_1", "Feature A", StarNodeType::Feature, "Core", SpatialCoord::new(0.0, 0.0, 0.0), "uri1");
        let star2 = StarNode::new("star_2", "Feature B", StarNodeType::Feature, "Core", SpatialCoord::new(10.0, 10.0, 10.0), "uri2");

        engine.insert_node(star1).await;
        engine.insert_node(star2).await;

        let snapshot = engine.export_snapshot().await.unwrap();
        assert_eq!(snapshot.total_stars, 2);
        assert_eq!(snapshot.total_galaxies, 1);
    }
}
