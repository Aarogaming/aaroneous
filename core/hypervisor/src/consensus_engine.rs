// Consensus Engine for High-Availability
// Enables distributed decision-making across multiple autonomic loops

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// A decision that needs consensus approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedDecision {
    pub decision_id: String,
    pub proposer_node: String,
    pub timestamp: DateTime<Utc>,
    pub decision_type: DecisionType,
    pub data: Vec<u8>,
    pub confidence: f32,  // 0.0-1.0
}

/// Types of system decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DecisionType {
    /// What plan to execute next
    PlanSelection,
    /// Which specialist to route task to
    TaskRouting,
    /// Update learning model weights
    ModelUpdate,
    /// Adjust thermal throttle level
    ThermalResponse,
    /// Add/remove specialist from pool
    SpecialistManagement,
}

/// Record of a decision and its votes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub decision: ProposedDecision,
    pub votes: HashMap<String, Vote>,
    pub status: DecisionStatus,
    pub consensus_score: f32,  // 0.0-1.0
}

/// Vote from a peer node
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Vote {
    /// Agree with the proposal
    Approve,
    /// Disagree with the proposal
    Reject,
    /// Abstain from voting
    Abstain,
}

/// Status of a decision
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionStatus {
    /// Pending votes from peers
    Pending,
    /// Consensus reached, decision approved
    Approved,
    /// Consensus failed, decision rejected
    Rejected,
    /// Consensus not reachable (deadlock)
    Deadlocked,
}

/// Consensus engine for distributed decision making
pub struct ConsensusEngine {
    pub node_id: String,
    pub peers: Vec<String>,
    pub voting_threshold: f32,  // 0.5-1.0, typically 0.6 (60%)
    pub decision_history: Vec<DecisionRecord>,
    pub pending_decisions: HashMap<String, DecisionRecord>,
    pub max_history: usize,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new(node_id: &str, peers: Vec<String>, voting_threshold: f32) -> Self {
        Self {
            node_id: node_id.to_string(),
            peers,
            voting_threshold: voting_threshold.max(0.5).min(1.0),
            decision_history: Vec::new(),
            pending_decisions: HashMap::new(),
            max_history: 1000,
        }
    }

    /// Propose a new decision (to be voted on by peers)
    pub fn propose_decision(&mut self, decision: ProposedDecision) -> String {
        let decision_id = decision.decision_id.clone();
        
        // Initialize with this node's automatic approval
        let mut votes = HashMap::new();
        votes.insert(self.node_id.clone(), Vote::Approve);
        
        let record = DecisionRecord {
            decision: decision.clone(),
            votes,
            status: DecisionStatus::Pending,
            consensus_score: 0.0,
        };
        
        self.pending_decisions.insert(decision_id.clone(), record);
        
        println!("[ConsensusEngine] Decision {} proposed by {} (type: {:?})", 
            decision_id, decision.proposer_node, decision.decision_type);
        
        decision_id
    }

    /// Cast a vote on a pending decision
    pub fn vote_on_decision(&mut self, decision_id: &str, voter_id: &str, vote: Vote) -> bool {
        if let Some(record) = self.pending_decisions.get_mut(decision_id) {
            record.votes.insert(voter_id.to_string(), vote);
            
            // Check if consensus is reached
            self.check_consensus(decision_id);
            
            println!("[ConsensusEngine] Vote from {} on decision {}: {:?}", 
                voter_id, decision_id, vote);
            
            true
        } else {
            println!("[ConsensusEngine] Decision {} not found", decision_id);
            false
        }
    }

    /// Check if a decision has reached consensus
    fn check_consensus(&mut self, decision_id: &str) {
        if let Some(record) = self.pending_decisions.get_mut(decision_id) {
            let total_votes = record.votes.len();
            let needed_votes = ((self.peers.len() + 1) as f32 * self.voting_threshold).ceil() as usize;
            
            let approve_count = record.votes.values().filter(|v| **v == Vote::Approve).count();
            let reject_count = record.votes.values().filter(|v| **v == Vote::Reject).count();
            
            // Calculate consensus score
            record.consensus_score = approve_count as f32 / total_votes as f32;
            
            // Determine status
            if approve_count >= needed_votes {
                record.status = DecisionStatus::Approved;
                println!("[ConsensusEngine] Decision {} APPROVED ({}/{})", 
                    decision_id, approve_count, needed_votes);
            } else if reject_count > (self.peers.len() + 1 - needed_votes) {
                record.status = DecisionStatus::Rejected;
                println!("[ConsensusEngine] Decision {} REJECTED", decision_id);
            } else if total_votes >= self.peers.len() + 1 {
                // All votes collected but no majority
                record.status = DecisionStatus::Deadlocked;
                println!("[ConsensusEngine] Decision {} DEADLOCKED", decision_id);
            }
        }
    }

    /// Get the status of a decision
    pub fn get_decision_status(&self, decision_id: &str) -> Option<DecisionStatus> {
        self.pending_decisions.get(decision_id).map(|r| r.status)
    }

    /// Finalize a decision (move from pending to history)
    pub fn finalize_decision(&mut self, decision_id: &str) -> Option<DecisionRecord> {
        if let Some(mut record) = self.pending_decisions.remove(decision_id) {
            // Only approved decisions are finalized
            if record.status == DecisionStatus::Approved {
                self.decision_history.push(record.clone());
                
                // Keep history bounded
                if self.decision_history.len() > self.max_history {
                    self.decision_history.remove(0);
                }
                
                Some(record)
            } else {
                // Put it back if not approved
                self.pending_decisions.insert(decision_id.to_string(), record.clone());
                None
            }
        } else {
            None
        }
    }

    /// Reach consensus by merging divergent proposals
    pub fn merge_proposals(&self, proposals: Vec<ProposedDecision>) -> Option<ProposedDecision> {
        if proposals.is_empty() {
            return None;
        }
        
        if proposals.len() == 1 {
            return Some(proposals[0].clone());
        }
        
        // Find proposal with highest confidence
        let best = proposals.iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .cloned()?;
        
        println!("[ConsensusEngine] Merged {} proposals into highest-confidence proposal", proposals.len());
        Some(best)
    }

    /// Get pending decisions for a specific type
    pub fn get_pending_by_type(&self, decision_type: &DecisionType) -> Vec<DecisionRecord> {
        self.pending_decisions.values()
            .filter(|r| r.decision.decision_type == *decision_type)
            .cloned()
            .collect()
    }

    /// Get decision history for analysis
    pub fn get_history(&self) -> &[DecisionRecord] {
        &self.decision_history
    }

    /// Get consensus statistics
    pub fn get_statistics(&self) -> ConsensusStatistics {
        let total_decisions = self.decision_history.len();
        let approved = self.decision_history.iter()
            .filter(|r| r.status == DecisionStatus::Approved)
            .count();
        
        let avg_consensus = if !self.decision_history.is_empty() {
            self.decision_history.iter()
                .map(|r| r.consensus_score)
                .sum::<f32>() / self.decision_history.len() as f32
        } else {
            0.0
        };
        
        ConsensusStatistics {
            total_decisions,
            approved,
            rejected: total_decisions - approved,
            avg_consensus_score: avg_consensus,
            pending_count: self.pending_decisions.len(),
        }
    }
}

/// Statistics about consensus decisions
#[derive(Debug, Clone)]
pub struct ConsensusStatistics {
    pub total_decisions: usize,
    pub approved: usize,
    pub rejected: usize,
    pub avg_consensus_score: f32,
    pub pending_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_proposal() {
        let peers = vec!["node_2".to_string(), "node_3".to_string()];
        let mut engine = ConsensusEngine::new("node_1", peers, 0.6);
        
        let decision = ProposedDecision {
            decision_id: "dec_001".to_string(),
            proposer_node: "node_1".to_string(),
            timestamp: Utc::now(),
            decision_type: DecisionType::PlanSelection,
            data: vec![1, 2, 3],
            confidence: 0.95,
        };
        
        let id = engine.propose_decision(decision);
        assert_eq!(id, "dec_001");
        assert!(engine.pending_decisions.contains_key(&id));
    }

    #[test]
    fn test_voting() {
        let peers = vec!["node_2".to_string(), "node_3".to_string()];
        let mut engine = ConsensusEngine::new("node_1", peers, 0.6);
        
        let decision = ProposedDecision {
            decision_id: "dec_002".to_string(),
            proposer_node: "node_1".to_string(),
            timestamp: Utc::now(),
            decision_type: DecisionType::TaskRouting,
            data: vec![],
            confidence: 0.8,
        };
        
        let id = engine.propose_decision(decision);
        
        // Votes: node_1 (auto-approve) + node_2 (approve) = 2/3
        engine.vote_on_decision(&id, "node_2", Vote::Approve);
        
        let status = engine.get_decision_status(&id);
        assert_eq!(status, Some(DecisionStatus::Approved));
    }

    #[test]
    fn test_rejection() {
        let peers = vec!["node_2".to_string(), "node_3".to_string()];
        let mut engine = ConsensusEngine::new("node_1", peers, 0.6);
        
        let decision = ProposedDecision {
            decision_id: "dec_003".to_string(),
            proposer_node: "node_1".to_string(),
            timestamp: Utc::now(),
            decision_type: DecisionType::ModelUpdate,
            data: vec![],
            confidence: 0.5,
        };
        
        let id = engine.propose_decision(decision);
        
        // Votes: node_1 (auto-approve), node_2 (reject), node_3 (reject)
        engine.vote_on_decision(&id, "node_2", Vote::Reject);
        engine.vote_on_decision(&id, "node_3", Vote::Reject);
        
        let status = engine.get_decision_status(&id);
        assert_eq!(status, Some(DecisionStatus::Rejected));
    }

    #[test]
    fn test_proposal_merge() {
        let engine = ConsensusEngine::new("node_1", vec![], 0.6);
        
        let proposals = vec![
            ProposedDecision {
                decision_id: "p1".to_string(),
                proposer_node: "node_1".to_string(),
                timestamp: Utc::now(),
                decision_type: DecisionType::PlanSelection,
                data: vec![1],
                confidence: 0.7,
            },
            ProposedDecision {
                decision_id: "p2".to_string(),
                proposer_node: "node_2".to_string(),
                timestamp: Utc::now(),
                decision_type: DecisionType::PlanSelection,
                data: vec![2],
                confidence: 0.9,  // Higher confidence
            },
        ];
        
        let merged = engine.merge_proposals(proposals);
        assert!(merged.is_some());
        assert_eq!(merged.unwrap().decision_id, "p2");
    }

    #[test]
    fn test_statistics() {
        let peers = vec!["node_2".to_string()];
        let mut engine = ConsensusEngine::new("node_1", peers, 0.5);
        
        for i in 0..5 {
            let decision = ProposedDecision {
                decision_id: format!("dec_{}", i),
                proposer_node: "node_1".to_string(),
                timestamp: Utc::now(),
                decision_type: DecisionType::PlanSelection,
                data: vec![],
                confidence: 0.8,
            };
            
            let id = engine.propose_decision(decision);
            engine.vote_on_decision(&id, "node_2", Vote::Approve);
            engine.finalize_decision(&id);
        }
        
        let stats = engine.get_statistics();
        assert_eq!(stats.total_decisions, 5);
        assert_eq!(stats.approved, 5);
        assert_eq!(stats.rejected, 0);
    }
}

