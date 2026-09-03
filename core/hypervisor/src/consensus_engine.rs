// Consensus Engine for High-Availability
// Enables distributed decision-making across multiple autonomic loops

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A decision that needs consensus approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedDecision {
    pub decision_id: String,
    pub proposer_node: String,
    pub timestamp: DateTime<Utc>,
    pub decision_type: DecisionType,
    pub data: Vec<u8>,
    pub confidence: f32, // 0.0-1.0
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
    pub consensus_score: f32, // 0.0-1.0
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

/// Raft node role in the high-availability cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

/// A replicated log entry for distributed WAL mutations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedWalEntry {
    pub term: u64,
    pub index: u64,
    pub payload: Vec<u8>,
    pub timestamp_ms: u64,
}

/// Request for a leader election vote (Raft RequestVote RPC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

/// Response to a vote request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
    pub voter_id: String,
}

/// Replicated log append request (Raft AppendEntries RPC / Heartbeat)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendEntriesRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<DistributedWalEntry>,
    pub leader_commit: u64,
}

/// Response to append entries request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftAppendEntriesResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
    pub responder_id: String,
}

/// Raft consensus cluster state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftClusterState {
    pub role: RaftRole,
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub leader_id: Option<String>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub log: Vec<DistributedWalEntry>,
}

impl Default for RaftClusterState {
    fn default() -> Self {
        Self {
            role: RaftRole::Follower,
            current_term: 0,
            voted_for: None,
            leader_id: None,
            commit_index: 0,
            last_applied: 0,
            log: Vec::new(),
        }
    }
}

/// Consensus engine for distributed decision making and Raft log replication
pub struct ConsensusEngine {
    pub node_id: String,
    pub peers: Vec<String>,
    pub voting_threshold: f32, // 0.5-1.0, typically 0.6 (60%)
    pub decision_history: Vec<DecisionRecord>,
    pub pending_decisions: HashMap<String, DecisionRecord>,
    pub max_history: usize,
    pub raft_state: RaftClusterState,
    pub election_votes: std::collections::HashSet<String>,
}

impl ConsensusEngine {
    /// Create a new consensus engine
    pub fn new(node_id: &str, peers: Vec<String>, voting_threshold: f32) -> Self {
        Self {
            node_id: node_id.to_string(),
            peers,
            voting_threshold: voting_threshold.clamp(0.5, 1.0),
            decision_history: Vec::new(),
            pending_decisions: HashMap::new(),
            max_history: 1000,
            raft_state: RaftClusterState::default(),
            election_votes: std::collections::HashSet::new(),
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

        println!(
            "[ConsensusEngine] Decision {} proposed by {} (type: {:?})",
            decision_id, decision.proposer_node, decision.decision_type
        );

        decision_id
    }

    /// Cast a vote on a pending decision
    pub fn vote_on_decision(&mut self, decision_id: &str, voter_id: &str, vote: Vote) -> bool {
        if let Some(record) = self.pending_decisions.get_mut(decision_id) {
            record.votes.insert(voter_id.to_string(), vote);

            // Check if consensus is reached
            self.check_consensus(decision_id);

            println!(
                "[ConsensusEngine] Vote from {} on decision {}: {:?}",
                voter_id, decision_id, vote
            );

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
            let needed_votes =
                ((self.peers.len() + 1) as f32 * self.voting_threshold).ceil() as usize;

            let approve_count = record
                .votes
                .values()
                .filter(|v| **v == Vote::Approve)
                .count();
            let reject_count = record
                .votes
                .values()
                .filter(|v| **v == Vote::Reject)
                .count();

            // Calculate consensus score
            record.consensus_score = approve_count as f32 / total_votes as f32;

            // Determine status
            if approve_count >= needed_votes {
                record.status = DecisionStatus::Approved;
                println!(
                    "[ConsensusEngine] Decision {} APPROVED ({}/{})",
                    decision_id, approve_count, needed_votes
                );
            } else if reject_count > (self.peers.len() + 1 - needed_votes) {
                record.status = DecisionStatus::Rejected;
                println!("[ConsensusEngine] Decision {} REJECTED", decision_id);
            } else if total_votes > self.peers.len() {
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
        if let Some(record) = self.pending_decisions.remove(decision_id) {
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
                self.pending_decisions
                    .insert(decision_id.to_string(), record.clone());
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
        let best = proposals
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .cloned()?;

        println!(
            "[ConsensusEngine] Merged {} proposals into highest-confidence proposal",
            proposals.len()
        );
        Some(best)
    }

    /// Get pending decisions for a specific type
    pub fn get_pending_by_type(&self, decision_type: &DecisionType) -> Vec<DecisionRecord> {
        self.pending_decisions
            .values()
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
        let approved = self
            .decision_history
            .iter()
            .filter(|r| r.status == DecisionStatus::Approved)
            .count();

        let avg_consensus = if !self.decision_history.is_empty() {
            self.decision_history
                .iter()
                .map(|r| r.consensus_score)
                .sum::<f32>()
                / self.decision_history.len() as f32
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

    /// Returns the current Raft cluster role
    pub fn role(&self) -> RaftRole {
        self.raft_state.role
    }

    /// Returns the current Raft consensus term
    pub fn current_term(&self) -> u64 {
        self.raft_state.current_term
    }

    /// Returns whether this node is the active elected cluster Leader
    pub fn is_leader(&self) -> bool {
        self.raft_state.role == RaftRole::Leader
    }

    /// Transitions to Candidate and initiates a leader election round
    pub fn start_election(&mut self) -> RaftVoteRequest {
        self.raft_state.role = RaftRole::Candidate;
        self.raft_state.current_term += 1;
        self.raft_state.voted_for = Some(self.node_id.clone());
        self.raft_state.leader_id = None;
        self.election_votes.clear();
        self.election_votes.insert(self.node_id.clone());

        let last_log_index = self.raft_state.log.len() as u64;
        let last_log_term = self.raft_state.log.last().map(|e| e.term).unwrap_or(0);

        // In a single-node cluster, candidate immediately becomes leader
        if self.peers.is_empty() {
            self.raft_state.role = RaftRole::Leader;
            self.raft_state.leader_id = Some(self.node_id.clone());
        }

        RaftVoteRequest {
            term: self.raft_state.current_term,
            candidate_id: self.node_id.clone(),
            last_log_index,
            last_log_term,
        }
    }

    /// Handles an incoming RequestVote RPC from a peer candidate
    pub fn handle_vote_request(&mut self, req: &RaftVoteRequest) -> RaftVoteResponse {
        if req.term > self.raft_state.current_term {
            self.raft_state.current_term = req.term;
            self.raft_state.role = RaftRole::Follower;
            self.raft_state.voted_for = None;
            self.raft_state.leader_id = None;
        }

        let my_last_index = self.raft_state.log.len() as u64;
        let my_last_term = self.raft_state.log.last().map(|e| e.term).unwrap_or(0);

        let log_up_to_date = req.last_log_term > my_last_term
            || (req.last_log_term == my_last_term && req.last_log_index >= my_last_index);

        let can_vote = (self.raft_state.voted_for.is_none()
            || self.raft_state.voted_for.as_ref() == Some(&req.candidate_id))
            && req.term == self.raft_state.current_term
            && log_up_to_date;

        if can_vote {
            self.raft_state.voted_for = Some(req.candidate_id.clone());
            RaftVoteResponse {
                term: self.raft_state.current_term,
                vote_granted: true,
                voter_id: self.node_id.clone(),
            }
        } else {
            RaftVoteResponse {
                term: self.raft_state.current_term,
                vote_granted: false,
                voter_id: self.node_id.clone(),
            }
        }
    }

    /// Processes a vote response. Returns true if candidate won the election and became Leader.
    pub fn handle_vote_response(&mut self, res: &RaftVoteResponse) -> bool {
        if res.term > self.raft_state.current_term {
            self.raft_state.current_term = res.term;
            self.raft_state.role = RaftRole::Follower;
            self.raft_state.voted_for = None;
            self.raft_state.leader_id = None;
            self.election_votes.clear();
            return false;
        }

        if self.raft_state.role == RaftRole::Candidate
            && res.term == self.raft_state.current_term
            && res.vote_granted
        {
            self.election_votes.insert(res.voter_id.clone());
            let quorum = (self.peers.len() + 2).div_ceil(2);
            if self.election_votes.len() >= quorum {
                self.raft_state.role = RaftRole::Leader;
                self.raft_state.leader_id = Some(self.node_id.clone());
                return true;
            }
        }
        false
    }

    /// Appends a new WAL mutation to the leader log and returns an AppendEntries request for peers
    pub fn append_wal_mutation(
        &mut self,
        payload: Vec<u8>,
    ) -> Result<RaftAppendEntriesRequest, String> {
        if self.raft_state.role != RaftRole::Leader {
            return Err("Only the elected Raft leader can append WAL mutations".to_string());
        }

        let prev_log_index = self.raft_state.log.len() as u64;
        let prev_log_term = self.raft_state.log.last().map(|e| e.term).unwrap_or(0);
        let new_index = prev_log_index + 1;

        let entry = DistributedWalEntry {
            term: self.raft_state.current_term,
            index: new_index,
            payload,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        self.raft_state.log.push(entry.clone());

        Ok(RaftAppendEntriesRequest {
            term: self.raft_state.current_term,
            leader_id: self.node_id.clone(),
            prev_log_index,
            prev_log_term,
            entries: vec![entry],
            leader_commit: self.raft_state.commit_index,
        })
    }

    /// Handles AppendEntries RPC (replicates log entries & updates commit index)
    pub fn handle_append_entries(
        &mut self,
        req: &RaftAppendEntriesRequest,
    ) -> RaftAppendEntriesResponse {
        if req.term < self.raft_state.current_term {
            return RaftAppendEntriesResponse {
                term: self.raft_state.current_term,
                success: false,
                match_index: self.raft_state.log.len() as u64,
                responder_id: self.node_id.clone(),
            };
        }

        if req.term > self.raft_state.current_term || self.raft_state.role == RaftRole::Candidate {
            self.raft_state.current_term = req.term;
            self.raft_state.role = RaftRole::Follower;
            self.raft_state.voted_for = None;
        }

        self.raft_state.leader_id = Some(req.leader_id.clone());

        // Verify prev_log_index and prev_log_term
        if req.prev_log_index > 0 {
            let idx = (req.prev_log_index - 1) as usize;
            if idx >= self.raft_state.log.len() || self.raft_state.log[idx].term != req.prev_log_term
            {
                return RaftAppendEntriesResponse {
                    term: self.raft_state.current_term,
                    success: false,
                    match_index: self.raft_state.log.len() as u64,
                    responder_id: self.node_id.clone(),
                };
            }
        }

        // Append new entries
        for entry in &req.entries {
            let idx = (entry.index - 1) as usize;
            if idx < self.raft_state.log.len() {
                self.raft_state.log[idx] = entry.clone();
            } else {
                self.raft_state.log.push(entry.clone());
            }
        }

        // Advance commit index
        if req.leader_commit > self.raft_state.commit_index {
            self.raft_state.commit_index =
                req.leader_commit.min(self.raft_state.log.len() as u64);
        }

        RaftAppendEntriesResponse {
            term: self.raft_state.current_term,
            success: true,
            match_index: self.raft_state.log.len() as u64,
            responder_id: self.node_id.clone(),
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
                confidence: 0.9, // Higher confidence
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

    #[test]
    fn test_raft_election_single_node() {
        let mut engine = ConsensusEngine::new("node_1", vec![], 0.6);
        assert_eq!(engine.role(), RaftRole::Follower);
        assert_eq!(engine.current_term(), 0);

        let vote_req = engine.start_election();
        assert_eq!(vote_req.term, 1);
        assert_eq!(vote_req.candidate_id, "node_1");
        assert_eq!(engine.role(), RaftRole::Leader);
        assert!(engine.is_leader());
    }

    #[test]
    fn test_raft_election_multi_node_quorum() {
        let peers = vec!["node_2".to_string(), "node_3".to_string()];
        let mut node1 = ConsensusEngine::new("node_1", peers, 0.6);

        let req = node1.start_election();
        assert_eq!(node1.role(), RaftRole::Candidate);
        assert_eq!(req.term, 1);

        let mut node2 = ConsensusEngine::new("node_2", vec!["node_1".to_string(), "node_3".to_string()], 0.6);
        let resp2 = node2.handle_vote_request(&req);
        assert!(resp2.vote_granted);

        // Process vote response on candidate -> 2/3 votes -> Leader
        let won = node1.handle_vote_response(&resp2);
        assert!(won);
        assert_eq!(node1.role(), RaftRole::Leader);
        assert!(node1.is_leader());
    }

    #[test]
    fn test_raft_wal_entry_replication_and_commit() {
        let peers = vec!["node_2".to_string()];
        let mut leader = ConsensusEngine::new("node_1", peers, 0.5);
        leader.start_election();
        leader.raft_state.role = RaftRole::Leader;

        let append_req = leader.append_wal_mutation(b"WAL_RECORD_TX_001".to_vec()).unwrap();
        assert_eq!(append_req.entries.len(), 1);
        assert_eq!(append_req.entries[0].index, 1);

        let mut follower = ConsensusEngine::new("node_2", vec!["node_1".to_string()], 0.5);
        let append_resp = follower.handle_append_entries(&append_req);
        assert!(append_resp.success);
        assert_eq!(append_resp.match_index, 1);
        assert_eq!(follower.raft_state.log.len(), 1);
        assert_eq!(follower.raft_state.log[0].payload, b"WAL_RECORD_TX_001");
    }
}
