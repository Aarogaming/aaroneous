//! galaxy_cluster.rs
//! Dynamic content-based gravitational clustering and Galaxy nebulae formation in Omni.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::spatial_coord::SpatialCoord;
use crate::star_node::StarNode;

/// A Galaxy (Cluster of StarNodes) formed dynamically by content gravity
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GalaxyCluster {
    pub galaxy_id: String,
    pub name: String,
    pub center: SpatialCoord,
    pub radius: f64,
    pub star_ids: Vec<String>,
    pub dominant_domain: String,
}

/// Dynamic Galactic Clustering Engine
pub struct GalacticClusteringEngine {
    pub clustering_threshold: f64,
}

impl Default for GalacticClusteringEngine {
    fn default() -> Self {
        Self {
            clustering_threshold: 200.0,
        }
    }
}

impl GalacticClusteringEngine {
    pub fn new(clustering_threshold: f64) -> Self {
        Self { clustering_threshold }
    }

    /// Clusters a set of star-nodes into galaxies based on spatial and semantic proximity
    pub fn compute_galaxies(&self, nodes: &HashMap<String, StarNode>) -> Vec<GalaxyCluster> {
        let mut galaxies = Vec::new();
        let mut visited = HashSet::new();

        for (id, node) in nodes {
            if visited.contains(id) {
                continue;
            }

            let mut cluster_members = vec![id.clone()];
            visited.insert(id.clone());

            let mut queue = vec![node];
            while let Some(current) = queue.pop() {
                for (other_id, other_node) in nodes {
                    if !visited.contains(other_id) {
                        let dist = current.spatial_coord.semantic_distance_to(&other_node.spatial_coord);
                        if dist <= self.clustering_threshold {
                            cluster_members.push(other_id.clone());
                            visited.insert(other_id.clone());
                            queue.push(other_node);
                        }
                    }
                }
            }

            // Only form a named galaxy if 2 or more stars cluster together
            if cluster_members.len() >= 2 {
                let mut sum_x = 0.0;
                let mut sum_y = 0.0;
                let mut sum_z = 0.0;
                let mut domain_counts = HashMap::new();

                for member_id in &cluster_members {
                    if let Some(m) = nodes.get(member_id) {
                        sum_x += m.spatial_coord.x;
                        sum_y += m.spatial_coord.y;
                        sum_z += m.spatial_coord.z;
                        *domain_counts.entry(m.domain.clone()).or_insert(0) += 1;
                    }
                }

                let count = cluster_members.len() as f64;
                let center = SpatialCoord::new(sum_x / count, sum_y / count, sum_z / count);
                let dominant_domain = domain_counts
                    .into_iter()
                    .max_by_key(|(_, c)| *c)
                    .map(|(d, _)| d)
                    .unwrap_or_else(|| "General".to_string());

                galaxies.push(GalaxyCluster {
                    galaxy_id: format!("galaxy_{}", galaxies.len() + 1),
                    name: format!("{} Nebula", dominant_domain),
                    center,
                    radius: self.clustering_threshold,
                    star_ids: cluster_members,
                    dominant_domain,
                });
            }
        }

        galaxies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::star_node::StarNodeType;

    #[test]
    fn test_galaxy_clustering() {
        let mut nodes = HashMap::new();
        for i in 0..4 {
            let id = format!("star_{}", i);
            let node = StarNode::new(
                &id,
                &format!("Node {}", i),
                StarNodeType::Feature,
                "Compute",
                SpatialCoord::new(10.0 * i as f64, 10.0 * i as f64, 0.0),
                "file:///path",
            );
            nodes.insert(id, node);
        }

        let engine = GalacticClusteringEngine::new(100.0);
        let galaxies = engine.compute_galaxies(&nodes);

        assert_eq!(galaxies.len(), 1);
        assert_eq!(galaxies[0].star_ids.len(), 4);
        assert_eq!(galaxies[0].dominant_domain, "Compute");
    }
}
