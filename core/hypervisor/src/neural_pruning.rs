use crate::ConstellationNode;
use crate::NodeType;

/// Archive of pruned nodes, allowing restoration.
#[derive(Debug, Clone, Default)]
pub struct PrunedArchive {
    nodes: Vec<ConstellationNode>,
    prune_reasons: Vec<String>,
}

impl PrunedArchive {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a pruned node with its reason.
    pub fn store(&mut self, node: ConstellationNode, reason: String) {
        self.nodes.push(node);
        self.prune_reasons.push(reason);
    }

    /// Restore nodes by ID. Returns the restored nodes and removes them from the archive.
    pub fn restore(&mut self, ids: &[String]) -> Vec<ConstellationNode> {
        let mut restored = Vec::new();
        let mut remaining_nodes = Vec::new();
        let mut remaining_reasons = Vec::new();

        for (node, reason) in self.nodes.drain(..).zip(self.prune_reasons.drain(..)) {
            if ids.contains(&node.id) {
                println!(
                    "[PrunedArchive] Restoring node: {} (was: {})",
                    node.title, reason
                );
                restored.push(node);
            } else {
                remaining_nodes.push(node);
                remaining_reasons.push(reason);
            }
        }

        self.nodes = remaining_nodes;
        self.prune_reasons = remaining_reasons;
        restored
    }

    /// List all archived node IDs and their prune reasons.
    pub fn list(&self) -> Vec<(&str, &str)> {
        self.nodes
            .iter()
            .zip(self.prune_reasons.iter())
            .map(|(n, r)| (n.id.as_str(), r.as_str()))
            .collect()
    }

    /// Number of archived nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

pub struct NeuralPruningEnzyme {
    min_integrity_threshold: u8,
}

impl NeuralPruningEnzyme {
    pub fn new(threshold: u8) -> Self {
        Self {
            min_integrity_threshold: threshold,
        }
    }

    /// Identifies and archives low-value or redundant nodes to reduce memory pressure.
    /// Pruned nodes are stored in the archive for potential restoration.
    pub fn prune_constellation(
        &self,
        nodes: &mut Vec<ConstellationNode>,
        archive: &mut PrunedArchive,
    ) -> Vec<String> {
        println!("[NeuralPruning] Commencing pruning cycle...");
        let now = chrono::Utc::now();
        let mut pruned_ids = Vec::new();
        let mut keep_indices = Vec::new();

        for (index, node) in nodes.iter().enumerate() {
            let mut should_prune = false;
            let mut reason = String::new();

            // 1. Calculate Information Decay
            let age_days = now.signed_duration_since(node.updated_at).num_days().max(0) as f32;
            let priority_weight = match node.priority {
                crate::Priority::Low => 1.0,
                crate::Priority::Medium => 2.5,
                crate::Priority::High => 5.0,
                crate::Priority::Critical => 10.0,
            };

            // Relevance score decays over time unless priority is high
            let relevance_score = priority_weight / (1.0 + age_days * 0.1);

            if relevance_score < (self.min_integrity_threshold as f32 / 10.0) {
                // Keep completed architecture or core decisions regardless of age
                if node.node_type != NodeType::Architecture && node.node_type != NodeType::Decision
                {
                    should_prune = true;
                    reason = format!("low relevance score: {:.2}", relevance_score);
                }
            }

            // 2. Prune redundant nodes
            if node.metadata.contains_key("redundant") || node.status == crate::NodeStatus::Archived
            {
                should_prune = true;
                if reason.is_empty() {
                    reason = "redundant or archived".to_string();
                }
            }

            if should_prune {
                pruned_ids.push(node.id.clone());
                archive.store(node.clone(), reason);
                println!(
                    "[NeuralPruning] Pruning low-value node: {} (Score: {:.2})",
                    node.title, relevance_score
                );
            } else {
                keep_indices.push(index);
            }
        }

        // Keep only non-pruned nodes
        let mut new_nodes = Vec::new();
        for i in keep_indices {
            new_nodes.push(nodes[i].clone());
        }
        *nodes = new_nodes;

        pruned_ids
    }
}
