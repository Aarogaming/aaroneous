// Aaroneous Constellation Module
// A 3D spatial knowledge management system for semantic similarity clustering and dynamic querying
// Nodes cluster by semantic similarity, representing the system's evolving understanding

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 3D coordinate in semantic space
/// X: Domain Spectrum (Theory ←→ Execution)
/// Y: Temporal Phase (Past ←→ Future)
/// Z: Priority/Visibility (Hidden ←→ Critical)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpatialCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl SpatialCoord {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Calculate Euclidean distance to another coordinate
    pub fn distance_to(&self, other: &SpatialCoord) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Calculate semantic distance accounting for domain/temporal/priority factors
    pub fn semantic_distance_to(
        &self,
        other: &SpatialCoord,
        _clustering_context: &ClusteringContext,
    ) -> f64 {
        let euclidean = self.distance_to(other);

        // Apply clustering attraction/repulsion based on context
        let domain_proximity = (self.x - other.x).abs() / 2000.0; // Normalized to 0-1
        let temporal_alignment = (self.y - other.y).abs() / 2000.0;
        let priority_alignment = (self.z - other.z).abs() / 2000.0;

        // Weighted combination: lower distance = more related
        let semantic_factor =
            (domain_proximity * 0.25 + temporal_alignment * 0.25 + priority_alignment * 0.25)
                * 0.75;
        euclidean * (1.0 + semantic_factor)
    }
}

/// Node types in the constellation
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Feature,
    Bug,
    Roadmap,
    Decision,
    Lore,
    Architecture,
    Incident,
    Reference,
    Resource,
    TestCase,
    KnowledgeGap,
    NeuralSignal,
    LatentPulse,
}

/// Status of a constellation node
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeStatus {
    Planned,
    InProgress,
    Completed,
    OnHold,
    Discovered,
    Archived,
}

/// Priority level for visibility in the constellation
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3,
}

/// Relationship type between nodes
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RelationshipType {
    DependsOn,
    Blocks,
    Implements,
    Documents,
    RelatesTo,
    ValidatesBy,
    CausedBy,
}

/// A single node in the constellation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstellationNode {
    pub id: String,
    pub node_type: NodeType,
    pub title: String,
    pub description: String,
    pub status: NodeStatus,
    pub priority: Priority,
    pub domain: String, // e.g., "control_plane", "agent_system", "biology_metabolism"
    pub spatial_coord: SpatialCoord,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: HashSet<String>,
    pub relationships: HashMap<String, RelationshipType>, // node_id -> relationship_type
    pub metadata: HashMap<String, String>,                // Key-value pairs for flexibility
    pub version: u32,
    pub is_hidden: bool,                  // For Easter eggs like test_repository
    pub discovery_reward: Option<String>, // Narrative reward for discovering this node
    pub activity_pulse: f32,              // 0.0 to 1.0, for neural signals
    pub latent_vector: Option<[f32; 32]>, // Reduced dimensionality for spatial clustering
}

impl ConstellationNode {
    pub fn new(
        id: String,
        node_type: NodeType,
        title: String,
        description: String,
        domain: String,
        spatial_coord: SpatialCoord,
    ) -> Self {
        Self {
            id,
            node_type,
            title,
            description,
            status: NodeStatus::Planned,
            priority: Priority::Medium,
            domain,
            spatial_coord,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: HashSet::new(),
            relationships: HashMap::new(),
            metadata: HashMap::new(),
            version: 1,
            is_hidden: false,
            discovery_reward: None,
            activity_pulse: 0.0,
            latent_vector: None,
        }
    }

    /// Update node status and reposition in spatial coordinates
    pub fn update_status(&mut self, new_status: NodeStatus) {
        if self.status != new_status {
            self.status = new_status.clone();
            self.updated_at = Utc::now();
            self.version += 1;

            // Reposition based on new status
            match new_status {
                NodeStatus::Planned => self.spatial_coord.y = -300.0,
                NodeStatus::InProgress => self.spatial_coord.y = 100.0,
                NodeStatus::Completed => self.spatial_coord.y = -800.0,
                NodeStatus::OnHold => self.spatial_coord.y = -100.0,
                _ => {}
            }
        }
    }

    /// Update node priority and adjust z-coordinate
    pub fn update_priority(&mut self, new_priority: Priority) {
        if self.priority != new_priority {
            self.priority = new_priority;
            self.updated_at = Utc::now();
            self.version += 1;

            // Adjust z-coordinate based on priority
            match new_priority {
                Priority::Low => self.spatial_coord.z = -200.0,
                Priority::Medium => self.spatial_coord.z = 100.0,
                Priority::High => self.spatial_coord.z = 500.0,
                Priority::Critical => self.spatial_coord.z = 800.0,
            }
        }
    }

    /// Add a relationship to another node
    pub fn add_relationship(&mut self, target_id: String, rel_type: RelationshipType) {
        self.relationships.insert(target_id, rel_type);
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Add a tag for filtering and discovery
    pub fn add_tag(&mut self, tag: String) {
        self.tags.insert(tag);
        self.updated_at = Utc::now();
    }
}

/// Context for clustering calculations
#[derive(Clone, Debug)]
pub struct ClusteringContext {
    pub weight_semantic_similarity: f64,
    pub weight_domain_proximity: f64,
    pub weight_temporal_alignment: f64,
    pub weight_relationship_strength: f64,
    pub clustering_threshold: f64, // Distance threshold for cluster membership
}

impl Default for ClusteringContext {
    fn default() -> Self {
        Self {
            weight_semantic_similarity: 0.3,
            weight_domain_proximity: 0.25,
            weight_temporal_alignment: 0.25,
            weight_relationship_strength: 0.2,
            clustering_threshold: 150.0,
        }
    }
}

/// A cluster of spatially-close nodes representing related concepts
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cluster {
    pub id: String,
    pub center: SpatialCoord,
    pub radius: f64,
    pub node_ids: Vec<String>,
    pub cluster_type: String, // "domain", "temporal", "feature_family", "crisis", "discovery"
}

/// Query filter for multi-dimensional search
#[derive(Clone, Debug, Default)]
pub struct ConstellationQuery {
    pub node_types: Option<Vec<NodeType>>,
    pub statuses: Option<Vec<NodeStatus>>,
    pub priorities: Option<Vec<Priority>>,
    pub domains: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub spatial_bounds: Option<SpatialBounds>,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub include_hidden: bool,
}

/// Spatial bounds for region queries
#[derive(Clone, Debug)]
pub struct SpatialBounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub z_min: f64,
    pub z_max: f64,
}

impl SpatialBounds {
    pub fn contains(&self, coord: &SpatialCoord) -> bool {
        coord.x >= self.x_min
            && coord.x <= self.x_max
            && coord.y >= self.y_min
            && coord.y <= self.y_max
            && coord.z >= self.z_min
            && coord.z <= self.z_max
    }
}

/// The main Constellation system - manages all nodes and queries
pub struct Constellation {
    nodes: HashMap<String, ConstellationNode>,
    clustering_context: ClusteringContext,
    clusters: Vec<Cluster>,
}

impl Constellation {
    pub fn new(clustering_context: ClusteringContext) -> Self {
        Self {
            nodes: HashMap::new(),
            clustering_context,
            clusters: Vec::new(),
        }
    }

    /// Add a node to the constellation
    pub fn add_node(&mut self, node: ConstellationNode) {
        self.nodes.insert(node.id.clone(), node);
        self.recalculate_clusters();
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&ConstellationNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut ConstellationNode> {
        self.nodes.get_mut(id)
    }

    /// Remove a node from the constellation
    pub fn remove_node(&mut self, id: &str) -> Option<ConstellationNode> {
        let removed = self.nodes.remove(id);
        if removed.is_some() {
            self.recalculate_clusters();
        }
        removed
    }

    /// Query nodes using multi-dimensional filters
    pub fn query(&self, query: &ConstellationQuery) -> Vec<&ConstellationNode> {
        self.nodes
            .values()
            .filter(|node| {
                // Apply visibility filter first
                if !query.include_hidden && node.is_hidden {
                    return false;
                }

                // Apply node type filter
                if let Some(ref types) = query.node_types {
                    if !types.contains(&node.node_type) {
                        return false;
                    }
                }

                // Apply status filter
                if let Some(ref statuses) = query.statuses {
                    if !statuses.contains(&node.status) {
                        return false;
                    }
                }

                // Apply priority filter
                if let Some(ref priorities) = query.priorities {
                    if !priorities.contains(&node.priority) {
                        return false;
                    }
                }

                // Apply domain filter
                if let Some(ref domains) = query.domains {
                    if !domains.iter().any(|d| node.domain.contains(d)) {
                        return false;
                    }
                }

                // Apply tag filter (must match all specified tags)
                if let Some(ref tags) = query.tags {
                    if !tags.iter().all(|t| node.tags.contains(t)) {
                        return false;
                    }
                }

                // Apply spatial bounds filter
                if let Some(ref bounds) = query.spatial_bounds {
                    if !bounds.contains(&node.spatial_coord) {
                        return false;
                    }
                }

                // Apply time range filter
                if let Some((start, end)) = query.time_range {
                    if node.created_at < start || node.created_at > end {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Find nodes spatially near a given coordinate
    pub fn find_nearby(&self, coord: &SpatialCoord, max_distance: f64) -> Vec<&ConstellationNode> {
        self.nodes
            .values()
            .filter(|node| coord.distance_to(&node.spatial_coord) <= max_distance)
            .collect()
    }

    /// Find the cluster containing a given node
    pub fn find_cluster(&self, node_id: &str) -> Option<&Cluster> {
        self.clusters
            .iter()
            .find(|c| c.node_ids.contains(&node_id.to_string()))
    }

    /// Get all clusters
    pub fn get_clusters(&self) -> &[Cluster] {
        &self.clusters
    }

    /// Recalculate clusters based on spatial proximity
    pub fn recalculate_clusters(&mut self) {
        self.clusters.clear();

        let mut visited = HashSet::new();
        let threshold = self.clustering_context.clustering_threshold;

        for node_id in self.nodes.keys().cloned().collect::<Vec<_>>() {
            if visited.contains(&node_id) {
                continue;
            }

            let mut cluster_nodes = vec![node_id.clone()];
            visited.insert(node_id.clone());

            // Find all nodes within clustering threshold
            let mut to_check = vec![node_id.clone()];
            while let Some(checking_id) = to_check.pop() {
                let checking_node = &self.nodes[&checking_id];

                for (other_id, other_node) in self.nodes.iter() {
                    if !visited.contains(other_id) {
                        let dist = checking_node.spatial_coord.semantic_distance_to(
                            &other_node.spatial_coord,
                            &self.clustering_context,
                        );

                        if dist <= threshold {
                            cluster_nodes.push(other_id.clone());
                            visited.insert(other_id.clone());
                            to_check.push(other_id.clone());
                        }
                    }
                }
            }

            // Only create cluster if it has minimum nodes
            if cluster_nodes.len() >= 3 {
                let center = self.calculate_cluster_center(&cluster_nodes);
                let cluster_type = self.determine_cluster_type(&cluster_nodes);

                self.clusters.push(Cluster {
                    id: format!("cluster_{}", self.clusters.len()),
                    center,
                    radius: threshold,
                    node_ids: cluster_nodes,
                    cluster_type,
                });
            }
        }
    }

    /// Calculate the spatial center of a cluster
    fn calculate_cluster_center(&self, node_ids: &[String]) -> SpatialCoord {
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_z = 0.0;

        for id in node_ids {
            if let Some(node) = self.nodes.get(id) {
                sum_x += node.spatial_coord.x;
                sum_y += node.spatial_coord.y;
                sum_z += node.spatial_coord.z;
            }
        }

        let count = node_ids.len() as f64;
        SpatialCoord {
            x: sum_x / count,
            y: sum_y / count,
            z: sum_z / count,
        }
    }

    /// Determine cluster type based on member characteristics
    fn determine_cluster_type(&self, node_ids: &[String]) -> String {
        // Analyze cluster composition to assign type
        let mut type_counts = HashMap::new();
        let mut status_counts = HashMap::new();
        let mut domain_counts = HashMap::new();

        for id in node_ids {
            if let Some(node) = self.nodes.get(id) {
                *type_counts.entry(node.node_type.clone()).or_insert(0) += 1;
                *status_counts.entry(node.status.clone()).or_insert(0) += 1;
                *domain_counts.entry(node.domain.clone()).or_insert(0) += 1;
            }
        }

        // --- ENHANCED LABELING ---
        if type_counts.get(&NodeType::Lore).unwrap_or(&0) > &1 {
            return "Research Discovery Hub".to_string();
        }
        if domain_counts.get("rust_optimization").unwrap_or(&0) > &1 {
            return "The Rust Optimization Hub".to_string();
        }

        // Heuristic determination
        if type_counts.get(&NodeType::Bug).unwrap_or(&0) > &2 {
            return "crisis".to_string();
        }

        if status_counts.get(&NodeStatus::InProgress).unwrap_or(&0) > &1 {
            return "feature_family".to_string();
        }

        if type_counts.get(&NodeType::Lore).unwrap_or(&0) > &0 {
            return "discovery".to_string();
        }

        "domain".to_string()
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get all nodes
    pub fn all_nodes(&self) -> Vec<&ConstellationNode> {
        self.nodes.values().collect()
    }

    /// Export constellation state as JSON-serializable structure
    pub fn export_snapshot(&self) -> ConstellationSnapshot {
        ConstellationSnapshot {
            node_count: self.nodes.len(),
            cluster_count: self.clusters.len(),
            nodes: self.nodes.values().cloned().collect(),
            clusters: self.clusters.clone(),
            exported_at: Utc::now(),
        }
    }
}

/// Snapshot of constellation state for persistence/versioning
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConstellationSnapshot {
    pub node_count: usize,
    pub cluster_count: usize,
    pub nodes: Vec<ConstellationNode>,
    pub clusters: Vec<Cluster>,
    pub exported_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_distance() {
        let coord1 = SpatialCoord::new(0.0, 0.0, 0.0);
        let coord2 = SpatialCoord::new(3.0, 4.0, 0.0);
        assert_eq!(coord1.distance_to(&coord2), 5.0);
    }

    #[test]
    fn test_node_creation() {
        let node = ConstellationNode::new(
            "test_node".to_string(),
            NodeType::Feature,
            "Test Feature".to_string(),
            "A test feature".to_string(),
            "test_domain".to_string(),
            SpatialCoord::new(100.0, 50.0, 200.0),
        );

        assert_eq!(node.id, "test_node");
        assert_eq!(node.priority, Priority::Medium);
        assert_eq!(node.status, NodeStatus::Planned);
        assert_eq!(node.version, 1);
    }

    #[test]
    fn test_status_update() {
        let mut node = ConstellationNode::new(
            "test_node".to_string(),
            NodeType::Feature,
            "Test Feature".to_string(),
            "A test feature".to_string(),
            "test_domain".to_string(),
            SpatialCoord::new(100.0, 50.0, 200.0),
        );

        node.update_status(NodeStatus::InProgress);
        assert_eq!(node.status, NodeStatus::InProgress);
        assert_eq!(node.version, 2);
        assert_eq!(node.spatial_coord.y, 100.0); // Repositioned
    }

    #[test]
    fn test_constellation_query() {
        let mut constellation = Constellation::new(ClusteringContext::default());

        let mut node1 = ConstellationNode::new(
            "node1".to_string(),
            NodeType::Feature,
            "Feature 1".to_string(),
            "Description".to_string(),
            "domain1".to_string(),
            SpatialCoord::new(0.0, 0.0, 0.0),
        );
        node1.priority = Priority::High;
        constellation.add_node(node1);

        let mut node2 = ConstellationNode::new(
            "node2".to_string(),
            NodeType::Bug,
            "Bug 1".to_string(),
            "Description".to_string(),
            "domain1".to_string(),
            SpatialCoord::new(0.0, 0.0, 0.0),
        );
        node2.priority = Priority::Low;
        constellation.add_node(node2);

        // Query for high priority nodes
        let query = ConstellationQuery {
            priorities: Some(vec![Priority::High]),
            ..Default::default()
        };

        let results = constellation.query(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "node1");
    }

    #[test]
    fn test_clustering() {
        let mut constellation = Constellation::new(ClusteringContext::default());

        // Add nodes close together
        for i in 0..5 {
            let node = ConstellationNode::new(
                format!("node{}", i),
                NodeType::Feature,
                format!("Feature {}", i),
                "Description".to_string(),
                "domain1".to_string(),
                SpatialCoord::new(100.0 + i as f64 * 10.0, 50.0, 200.0),
            );
            constellation.add_node(node);
        }

        let clusters = constellation.get_clusters();
        assert!(clusters.len() > 0, "Should have at least one cluster");
    }
}
