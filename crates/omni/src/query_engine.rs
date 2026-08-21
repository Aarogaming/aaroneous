//! query_engine.rs
//! Multi-dimensional spatial frustum and semantic proximity query engine for Omni.

use std::collections::HashMap;

use crate::spatial_coord::SpatialCoord;
use crate::star_node::{StarNode, StarNodeStatus, StarNodeType};

/// 3D Bounding Frustum for spatial region queries
#[derive(Clone, Debug)]
pub struct SpatialFrustum {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub z_min: f64,
    pub z_max: f64,
}

impl SpatialFrustum {
    pub fn contains(&self, coord: &SpatialCoord) -> bool {
        coord.x >= self.x_min
            && coord.x <= self.x_max
            && coord.y >= self.y_min
            && coord.y <= self.y_max
            && coord.z >= self.z_min
            && coord.z <= self.z_max
    }
}

/// Multi-dimensional search query filter
#[derive(Clone, Debug, Default)]
pub struct OmniQueryFilter {
    pub node_types: Option<Vec<StarNodeType>>,
    pub statuses: Option<Vec<StarNodeStatus>>,
    pub domains: Option<Vec<String>>,
    pub spatial_frustum: Option<SpatialFrustum>,
    pub max_results: Option<usize>,
}

/// Omni Search & Query Engine
pub struct OmniQueryEngine;

impl OmniQueryEngine {
    /// Queries the Omni star graph with multi-dimensional filters
    pub fn query<'a>(
        nodes: &'a HashMap<String, StarNode>,
        filter: &OmniQueryFilter,
    ) -> Vec<&'a StarNode> {
        let mut results: Vec<&'a StarNode> = nodes
            .values()
            .filter(|node| {
                // Node type filter
                if let Some(ref types) = filter.node_types {
                    if !types.contains(&node.node_type) {
                        return false;
                    }
                }

                // Status filter
                if let Some(ref statuses) = filter.statuses {
                    if !statuses.contains(&node.status) {
                        return false;
                    }
                }

                // Domain filter
                if let Some(ref domains) = filter.domains {
                    if !domains.iter().any(|d| node.domain.contains(d)) {
                        return false;
                    }
                }

                // Spatial frustum filter
                if let Some(ref frustum) = filter.spatial_frustum {
                    if !frustum.contains(&node.spatial_coord) {
                        return false;
                    }
                }

                true
            })
            .collect();

        if let Some(max) = filter.max_results {
            results.truncate(max);
        }

        results
    }

    /// Finds all star-nodes spatially nearest to a target coordinate
    pub fn find_nearest<'a>(
        nodes: &'a HashMap<String, StarNode>,
        target: &SpatialCoord,
        max_distance: f64,
    ) -> Vec<&'a StarNode> {
        let mut nearest: Vec<(&'a StarNode, f64)> = nodes
            .values()
            .map(|n| (n, n.spatial_coord.distance_to(target)))
            .filter(|(_, d)| *d <= max_distance)
            .collect();

        nearest.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        nearest.into_iter().map(|(n, _)| n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frustum_query() {
        let mut nodes = HashMap::new();
        let star1 = StarNode::new("s1", "Star 1", StarNodeType::Feature, "Core", SpatialCoord::new(10.0, 10.0, 10.0), "uri1");
        let star2 = StarNode::new("s2", "Star 2", StarNodeType::Feature, "Core", SpatialCoord::new(500.0, 500.0, 500.0), "uri2");
        nodes.insert("s1".to_string(), star1);
        nodes.insert("s2".to_string(), star2);

        let filter = OmniQueryFilter {
            spatial_frustum: Some(SpatialFrustum {
                x_min: 0.0,
                x_max: 100.0,
                y_min: 0.0,
                y_max: 100.0,
                z_min: 0.0,
                z_max: 100.0,
            }),
            ..Default::default()
        };

        let res = OmniQueryEngine::query(&nodes, &filter);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].id, "s1");
    }
}
