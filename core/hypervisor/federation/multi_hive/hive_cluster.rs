/// HiveCluster: Multi-Hive Coordination System
/// 
/// Manages discovery, health monitoring, and coordination of multiple Aaroneous hives

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Configuration for hive cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub cluster_name: String,
    pub node_id: String,
    pub listen_addr: String,
    pub discovery_interval_ms: u64,
    pub health_check_interval_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub max_cluster_size: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_name: "aaroneous-cluster".to_string(),
            node_id: uuid::Uuid::new_v4().to_string(),
            listen_addr: "127.0.0.1:8001".to_string(),
            discovery_interval_ms: 5000,
            health_check_interval_ms: 1000,
            heartbeat_timeout_ms: 10000,
            max_cluster_size: 100,
        }
    }
}

/// Status of a hive node
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum HiveNodeStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

/// A node in the hive cluster (another Aaroneous hive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveNode {
    pub node_id: String,
    pub address: String,
    pub status: HiveNodeStatus,
    pub last_heartbeat_ms: u64,
    pub specialists_count: u32,
    pub total_models_mb: u32,
    pub uptime_seconds: u64,
    pub version: String,
}

impl HiveNode {
    pub fn new(node_id: String, address: String) -> Self {
        Self {
            node_id,
            address,
            status: HiveNodeStatus::Healthy,
            last_heartbeat_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            specialists_count: 6,
            total_models_mb: 6000,
            uptime_seconds: 0,
            version: "1.0.0".to_string(),
        }
    }

    /// Check if node is considered healthy
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, HiveNodeStatus::Healthy)
    }

    /// Get node utilization percentage
    pub fn utilization(&self) -> f32 {
        ((self.specialists_count as f32 / 6.0) * 100.0).min(100.0)
    }
}

/// The hive cluster coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HiveCluster {
    pub config: ClusterConfig,
    pub nodes: HashMap<String, HiveNode>,
    pub leader_node_id: Option<String>,
    pub total_specialists: u32,
    pub total_capacity_mb: u32,
}

impl HiveCluster {
    pub fn new(config: ClusterConfig) -> Self {
        Self {
            config,
            nodes: HashMap::new(),
            leader_node_id: None,
            total_specialists: 0,
            total_capacity_mb: 0,
        }
    }

    /// Add a node to the cluster
    pub fn add_node(&mut self, node: HiveNode) -> Result<(), String> {
        if self.nodes.len() >= self.config.max_cluster_size {
            return Err(format!("Cluster full (max {})", self.config.max_cluster_size));
        }

        let node_id = node.node_id.clone();
        self.nodes.insert(node_id, node.clone());

        self.total_specialists += node.specialists_count;
        self.total_capacity_mb += node.total_models_mb;

        // First node becomes leader
        if self.leader_node_id.is_none() {
            self.leader_node_id = Some(node.node_id);
        }

        Ok(())
    }

    /// Remove a node from cluster
    pub fn remove_node(&mut self, node_id: &str) -> Option<HiveNode> {
        let node = self.nodes.remove(node_id);

        if let Some(ref n) = node {
            self.total_specialists = self.total_specialists.saturating_sub(n.specialists_count);
            self.total_capacity_mb = self.total_capacity_mb.saturating_sub(n.total_models_mb);

            // Reassign leadership if needed
            if self.leader_node_id.as_ref().map(|id| id.as_str()) == Some(node_id) {
                self.leader_node_id = self.nodes.keys().next().cloned();
            }
        }

        node
    }

    /// Get cluster statistics
    pub fn stats(&self) -> ClusterStats {
        let healthy_count = self.nodes.values().filter(|n| n.is_healthy()).count();
        let total_count = self.nodes.len();
        let avg_utilization = if total_count == 0 {
            0.0
        } else {
            self.nodes.values().map(|n| n.utilization()).sum::<f32>() / total_count as f32
        };

        ClusterStats {
            total_nodes: total_count,
            healthy_nodes: healthy_count,
            degraded_nodes: total_count.saturating_sub(healthy_count),
            total_specialists: self.total_specialists,
            total_capacity_mb: self.total_capacity_mb,
            avg_node_utilization: avg_utilization,
        }
    }

    /// Get best node for assignment
    pub fn select_node_for_specialist(&self) -> Option<String> {
        self.nodes
            .values()
            .filter(|n| n.is_healthy())
            .min_by_key(|n| (n.utilization() * 100.0) as u32)
            .map(|n| n.node_id.clone())
    }

    /// Promote new leader
    pub fn elect_leader(&mut self) -> Option<String> {
        let new_leader = self
            .nodes
            .values()
            .filter(|n| n.is_healthy())
            .max_by_key(|n| n.specialists_count)
            .map(|n| n.node_id.clone());

        if let Some(ref leader_id) = new_leader {
            self.leader_node_id = Some(leader_id.clone());
        }

        new_leader
    }

    /// Update node status
    pub fn update_node_status(&mut self, node_id: &str, status: HiveNodeStatus) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = status;
        }
    }
}

impl Default for HiveCluster {
    fn default() -> Self {
        Self::new(ClusterConfig::default())
    }
}

/// Cluster statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub degraded_nodes: usize,
    pub total_specialists: u32,
    pub total_capacity_mb: u32,
    pub avg_node_utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hive_node_creation() {
        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        assert_eq!(node.node_id, "hive-1");
        assert!(node.is_healthy());
    }

    #[test]
    fn test_hive_node_utilization() {
        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        assert_eq!(node.utilization(), 100.0);
    }

    #[test]
    fn test_hive_cluster_add_node() {
        let config = ClusterConfig::default();
        let mut cluster = HiveCluster::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        let result = cluster.add_node(node);

        assert!(result.is_ok());
        assert_eq!(cluster.nodes.len(), 1);
    }

    #[test]
    fn test_hive_cluster_remove_node() {
        let config = ClusterConfig::default();
        let mut cluster = HiveCluster::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        cluster.add_node(node).ok();

        let removed = cluster.remove_node("hive-1");
        assert!(removed.is_some());
        assert_eq!(cluster.nodes.len(), 0);
    }

    #[test]
    fn test_hive_cluster_stats() {
        let config = ClusterConfig::default();
        let mut cluster = HiveCluster::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        cluster.add_node(node).ok();

        let stats = cluster.stats();
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.healthy_nodes, 1);
    }

    #[test]
    fn test_hive_cluster_leader_election() {
        let config = ClusterConfig::default();
        let mut cluster = HiveCluster::new(config);

        let node1 = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        let node2 = HiveNode::new("hive-2".to_string(), "127.0.0.1:8002".to_string());

        cluster.add_node(node1).ok();
        cluster.add_node(node2).ok();

        let leader = cluster.elect_leader();
        assert!(leader.is_some());
    }

    #[test]
    fn test_hive_cluster_select_node() {
        let config = ClusterConfig::default();
        let mut cluster = HiveCluster::new(config);

        let node = HiveNode::new("hive-1".to_string(), "127.0.0.1:8001".to_string());
        cluster.add_node(node).ok();

        let selected = cluster.select_node_for_specialist();
        assert!(selected.is_some());
    }
}
