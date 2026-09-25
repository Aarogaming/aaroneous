/// Consensus Engine: Distributed Decision Making
///
/// Implements gossip protocol for distributed consensus among hives
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State of a consensus instance
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConsensusState {
    Proposed,
    Accepted,
    Committed,
    Failed,
}

/// Gossip message for consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipMessage {
    pub message_id: String,
    pub proposal_id: String,
    pub from_node_id: String,
    pub state: ConsensusState,
    pub value: String,
    pub round: u32,
    pub timestamp_ms: u64,
    pub votes: HashMap<String, bool>,
}

impl GossipMessage {
    pub fn new(proposal_id: String, from_node_id: String, value: String) -> Self {
        Self {
            message_id: uuid::Uuid::new_v4().to_string(),
            proposal_id,
            from_node_id,
            state: ConsensusState::Proposed,
            value,
            round: 0,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            votes: HashMap::new(),
        }
    }

    /// Add a vote
    pub fn add_vote(&mut self, node_id: String, vote: bool) {
        self.votes.insert(node_id, vote);
    }

    /// Get vote count
    pub fn vote_count(&self) -> (usize, usize) {
        let yes: usize = self.votes.values().filter(|&&v| v).count();
        let no: usize = self.votes.values().filter(|&&v| !v).count();
        (yes, no)
    }

    /// Check if consensus reached (>66% agreement)
    pub fn consensus_reached(&self, _total_nodes: usize) -> bool {
        let (yes, no) = self.vote_count();
        let total_votes = yes + no;
        if total_votes == 0 {
            return false;
        }
        let yes_percent = (yes as f32 / total_votes as f32) * 100.0;
        yes_percent > 66.0 || no > yes // Either 2/3 yes or all nos
    }
}

/// Consensus instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusInstance {
    pub proposal_id: String,
    pub state: ConsensusState,
    pub messages: Vec<GossipMessage>,
    pub created_at_ms: u64,
    pub decided_value: Option<String>,
    pub final_votes: HashMap<String, bool>,
}

impl ConsensusInstance {
    pub fn new(proposal_id: String) -> Self {
        Self {
            proposal_id,
            state: ConsensusState::Proposed,
            messages: Vec::new(),
            created_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            decided_value: None,
            final_votes: HashMap::new(),
        }
    }

    /// Add gossip message
    pub fn add_message(&mut self, message: GossipMessage) {
        self.messages.push(message);
    }

    /// Get consensus result
    pub fn consensus_result(&self, total_nodes: usize) -> Option<bool> {
        if self.messages.is_empty() {
            return None;
        }

        let latest = &self.messages[self.messages.len() - 1];
        if latest.consensus_reached(total_nodes) {
            let (yes, _) = latest.vote_count();
            Some(yes > 0)
        } else {
            None
        }
    }
}

/// Consensus Engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusEngine {
    pub instances: HashMap<String, ConsensusInstance>,
    pub total_decisions: u64,
    pub successful_consensuses: u64,
}

impl ConsensusEngine {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            total_decisions: 0,
            successful_consensuses: 0,
        }
    }

    /// Start a new consensus instance
    pub fn propose(&mut self, proposal_id: String, _value: String) -> String {
        let instance = ConsensusInstance::new(proposal_id.clone());
        self.instances.insert(proposal_id.clone(), instance);
        self.total_decisions += 1;
        proposal_id
    }

    /// Add vote to consensus instance
    pub fn vote(
        &mut self,
        proposal_id: &str,
        node_id: String,
        vote: bool,
        total_nodes: usize,
    ) -> Option<bool> {
        if let Some(instance) = self.instances.get_mut(proposal_id) {
            instance.final_votes.insert(node_id.clone(), vote);

            // Create gossip message
            let mut msg = GossipMessage::new(
                proposal_id.to_string(),
                node_id.clone(),
                "voted".to_string(),
            );
            msg.add_vote(node_id, vote);
            instance.add_message(msg);

            // Check if consensus reached
            if let Some(result) = instance.consensus_result(total_nodes) {
                instance.state = ConsensusState::Committed;
                instance.decided_value = Some(if result { "yes" } else { "no" }.to_string());
                self.successful_consensuses += 1;
                return Some(result);
            }
        }

        None
    }

    /// Broadcast gossip message
    pub fn broadcast_gossip(&mut self, message: GossipMessage) -> Result<(), String> {
        if let Some(instance) = self.instances.get_mut(&message.proposal_id) {
            instance.add_message(message);
            Ok(())
        } else {
            Err(format!("Unknown proposal: {}", message.proposal_id))
        }
    }

    /// Get consensus statistics
    pub fn stats(&self) -> ConsensusStats {
        ConsensusStats {
            total_decisions: self.total_decisions,
            successful_consensuses: self.successful_consensuses,
            pending_decisions: self.instances.len(),
            consensus_rate: if self.total_decisions == 0 {
                0.0
            } else {
                (self.successful_consensuses as f32 / self.total_decisions as f32) * 100.0
            },
        }
    }

    /// Get status of a consensus instance
    pub fn instance_status(&self, proposal_id: &str) -> Option<ConsensusState> {
        self.instances.get(proposal_id).map(|i| i.state)
    }
}

impl Default for ConsensusEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Consensus statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStats {
    pub total_decisions: u64,
    pub successful_consensuses: u64,
    pub pending_decisions: usize,
    pub consensus_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gossip_message_creation() {
        let msg = GossipMessage::new(
            "prop-1".to_string(),
            "hive-1".to_string(),
            "proposal".to_string(),
        );
        assert_eq!(msg.proposal_id, "prop-1");
        assert_eq!(msg.state, ConsensusState::Proposed);
    }

    #[test]
    fn test_gossip_message_voting() {
        let mut msg = GossipMessage::new(
            "prop-1".to_string(),
            "hive-1".to_string(),
            "proposal".to_string(),
        );

        msg.add_vote("hive-1".to_string(), true);
        msg.add_vote("hive-2".to_string(), true);

        let (yes, no) = msg.vote_count();
        assert_eq!(yes, 2);
        assert_eq!(no, 0);
    }

    #[test]
    fn test_consensus_instance_creation() {
        let instance = ConsensusInstance::new("prop-1".to_string());
        assert_eq!(instance.proposal_id, "prop-1");
        assert_eq!(instance.state, ConsensusState::Proposed);
    }

    #[test]
    fn test_consensus_engine_propose() {
        let mut engine = ConsensusEngine::new();
        let prop_id = engine.propose("prop-1".to_string(), "value".to_string());
        assert_eq!(prop_id, "prop-1");
        assert_eq!(engine.total_decisions, 1);
    }

    #[test]
    fn test_consensus_engine_voting() {
        let mut engine = ConsensusEngine::new();
        engine.propose("prop-1".to_string(), "value".to_string());

        // Get consensus with 3 yes votes out of 3
        engine.vote("prop-1", "hive-1".to_string(), true, 3);
        engine.vote("prop-1", "hive-2".to_string(), true, 3);
        let result = engine.vote("prop-1", "hive-3".to_string(), true, 3);

        assert!(result.is_some());
    }

    #[test]
    fn test_consensus_stats() {
        let mut engine = ConsensusEngine::new();
        engine.propose("prop-1".to_string(), "value".to_string());

        let stats = engine.stats();
        assert_eq!(stats.total_decisions, 1);
        assert_eq!(stats.pending_decisions, 1);
    }
}
