use crate::semantic_indexing::SemanticIndex;
use crate::concept_drift::ConceptDriftDetector;
use anyhow::Result;
use std::collections::HashSet;
use crate::ConstellationNode;
use crate::NodeType;

pub struct NeuralPruningEnzyme {
    min_integrity_threshold: u8,
}

impl NeuralPruningEnzyme {
    pub fn new(threshold: u8) -> Self {
        Self { min_integrity_threshold: threshold }
    }

    /// Identifies and archives low-value or redundant nodes to reduce memory pressure.
    pub fn prune_constellation(&self, nodes: &mut Vec<ConstellationNode>) -> Vec<String> {
        println!("[NeuralPruning] Commencing pruning cycle...");
        let now = chrono::Utc::now();
        let mut pruned_ids = Vec::new();
        let mut keep_indices = Vec::new();

        for (index, node) in nodes.iter().enumerate() {
            let mut should_prune = false;

            // 1. Calculate Information Decay
            let age_days = (now - node.updated_at).num_days().max(0) as f32;
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
                if node.node_type != NodeType::Architecture && node.node_type != NodeType::Decision {
                    should_prune = true;
                }
            }

            // 2. Prune redundant nodes
            if node.metadata.contains_key("redundant") || node.status == crate::NodeStatus::Archived {
                should_prune = true;
            }

            if should_prune {
                pruned_ids.push(node.id.clone());
                println!("[NeuralPruning] Pruning low-value node: {} (Score: {:.2})", node.title, relevance_score);
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
