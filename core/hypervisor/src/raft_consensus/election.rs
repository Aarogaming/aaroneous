/// Leader election implementation
///
/// Handles term advancement, election timeout, and distributed voting

use super::types::*;
use super::node::RaftNode;
use std::time::{Duration, Instant};
use rand::RngExt;

/// Election timeout tracker
#[derive(Clone, Debug)]
pub struct ElectionTimeout {
    reset_at: Instant,
    timeout_ms: u64,
}

impl ElectionTimeout {
    pub fn new(min_ms: u64, max_ms: u64) -> Self {
        let timeout = random_election_timeout(min_ms, max_ms);
        Self {
            reset_at: Instant::now() + Duration::from_millis(timeout),
            timeout_ms: timeout,
        }
    }

    pub fn reset(&mut self, min_ms: u64, max_ms: u64) {
        self.timeout_ms = random_election_timeout(min_ms, max_ms);
        self.reset_at = Instant::now() + Duration::from_millis(self.timeout_ms);
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.reset_at
    }
}

/// Randomized election timeout (between min and max)
pub fn random_election_timeout(min_ms: u64, max_ms: u64) -> u64 {
    let mut rng = rand::rng();
    rng.random_range(min_ms..=max_ms)
}

impl ElectionTimeout {
    /// Create new election timeout
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            reset_at: Instant::now() + Duration::from_millis(timeout_ms),
            timeout_ms,
        }
    }

    /// Reset the timeout
    pub fn reset(&mut self) {
        self.reset_at = Instant::now() + Duration::from_millis(self.timeout_ms);
    }

    /// Has timeout elapsed?
    pub fn is_elapsed(&self) -> bool {
        Instant::now() >= self.reset_at
    }

    /// Time until timeout (or 0 if elapsed)
    pub fn time_until_ms(&self) -> u64 {
        let remaining = self.reset_at.saturating_duration_since(Instant::now());
        remaining.as_millis() as u64
    }
}

/// Heartbeat timer for leaders
#[derive(Clone, Debug)]
pub struct HeartbeatTimer {
    next_heartbeat: Instant,
    interval_ms: u64,
}

impl HeartbeatTimer {
    /// Create new heartbeat timer
    pub fn new(interval_ms: u64) -> Self {
        Self {
            next_heartbeat: Instant::now() + Duration::from_millis(interval_ms),
            interval_ms,
        }
    }

    /// Should send heartbeat now?
    pub fn should_heartbeat(&self) -> bool {
        Instant::now() >= self.next_heartbeat
    }

    /// Reset heartbeat timer
    pub fn reset(&mut self) {
        self.next_heartbeat = Instant::now() + Duration::from_millis(self.interval_ms);
    }

    /// Time until next heartbeat
    pub fn time_until_ms(&self) -> u64 {
        let remaining = self.next_heartbeat.saturating_duration_since(Instant::now());
        remaining.as_millis() as u64
    }
}

/// Election result
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElectionOutcome {
    /// Still waiting for votes
    Pending,
    /// Won the election
    Won,
    /// Lost the election (higher term received)
    Lost { higher_term: Term },
    /// Revert to follower (received heartbeat from valid leader)
    Follower,
}

/// Handle a RequestVote RPC
pub fn handle_request_vote(
    node: &RaftNode,
    rpc: &RequestVoteRpc,
) -> Result<RequestVoteResponse, String> {
    let current_term = node.get_term()?;
    let voted_for = node.get_voted_for()?;
    let last_log_index = node.get_log().last_index()?;
    let last_log_term = node.get_log().last_term()?;

    // If request term is less than current term, reject
    if rpc.term < current_term {
        return Ok(RequestVoteResponse {
            term: current_term,
            vote_granted: false,
        });
    }

    // If request term is greater, update our term
    if rpc.term > current_term {
        node.update_term(rpc.term)?;
        node.become_follower(rpc.term, None)?;
    }

    // Check if we've already voted in this term
    if let Some(ref voted) = voted_for {
        if voted != &rpc.candidate_id {
            return Ok(RequestVoteResponse {
                term: rpc.term,
                vote_granted: false,
            });
        }
    }

    // Check log is at least as up-to-date as ours
    let candidate_log_ok = if rpc.last_log_term > last_log_term {
        true
    } else if rpc.last_log_term == last_log_term {
        rpc.last_log_index >= last_log_index
    } else {
        false
    };

    if !candidate_log_ok {
        return Ok(RequestVoteResponse {
            term: rpc.term,
            vote_granted: false,
        });
    }

    // Grant vote
    node.vote_for(rpc.candidate_id.clone())?;

    Ok(RequestVoteResponse {
        term: rpc.term,
        vote_granted: true,
    })
}

/// Check if candidate won the election
pub fn check_election_won(
    votes_received: u32,
    votes_needed: u32,
) -> bool {
    votes_received >= votes_needed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_election_timeout_creation() {
        let timeout = ElectionTimeout::new(200);
        assert!(!timeout.is_elapsed()); // Just created, shouldn't be elapsed
    }

    #[test]
    fn test_election_timeout_elapsed() {
        let mut timeout = ElectionTimeout::new(1);
        std::thread::sleep(Duration::from_millis(10));
        assert!(timeout.is_elapsed()); // Should be elapsed after 10ms
    }

    #[test]
    fn test_election_timeout_reset() {
        let mut timeout = ElectionTimeout::new(200);
        std::thread::sleep(Duration::from_millis(50));
        timeout.reset();
        assert!(!timeout.is_elapsed()); // After reset, shouldn't be elapsed
    }

    #[test]
    fn test_random_election_timeout_range() {
        let timeout = random_election_timeout(150, 300);
        assert!(timeout >= 150 && timeout <= 300);
    }

    #[test]
    fn test_heartbeat_timer() {
        let mut timer = HeartbeatTimer::new(50);
        assert!(!timer.should_heartbeat());
        std::thread::sleep(Duration::from_millis(60));
        assert!(timer.should_heartbeat());
        timer.reset();
        assert!(!timer.should_heartbeat());
    }

    #[test]
    fn test_request_vote_reject_old_term() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );
        let node = RaftNode::new(config);

        // Set current term to 5
        node.update_term(5).unwrap();

        let rpc = RequestVoteRpc {
            term: 3, // Old term
            candidate_id: "node2".to_string(),
            last_log_index: 10,
            last_log_term: 2,
        };

        let response = handle_request_vote(&node, &rpc).unwrap();
        assert!(!response.vote_granted);
        assert_eq!(response.term, 5);
    }

    #[test]
    fn test_request_vote_grant() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );
        let node = RaftNode::new(config);

        let rpc = RequestVoteRpc {
            term: 1,
            candidate_id: "node2".to_string(),
            last_log_index: 0,
            last_log_term: 0,
        };

        let response = handle_request_vote(&node, &rpc).unwrap();
        assert!(response.vote_granted);
        assert_eq!(response.term, 1);
    }

    #[test]
    fn test_request_vote_duplicate_candidate() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string(), "node3".to_string()],
        );
        let node = RaftNode::new(config);

        let rpc = RequestVoteRpc {
            term: 1,
            candidate_id: "node2".to_string(),
            last_log_index: 0,
            last_log_term: 0,
        };

        // Vote for node2
        let response1 = handle_request_vote(&node, &rpc).unwrap();
        assert!(response1.vote_granted);

        // Vote again for node2 in same term (should grant)
        let response2 = handle_request_vote(&node, &rpc).unwrap();
        assert!(response2.vote_granted);

        // Vote for different candidate (should reject)
        let rpc3 = RequestVoteRpc {
            term: 1,
            candidate_id: "node3".to_string(),
            last_log_index: 0,
            last_log_term: 0,
        };
        let response3 = handle_request_vote(&node, &rpc3).unwrap();
        assert!(!response3.vote_granted);
    }

    #[test]
    fn test_request_vote_log_up_to_date_by_term() {
        let config = RaftConfig::new(
            "node1".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );
        let node = RaftNode::new(config);

        // Candidate has higher term in log
        let rpc = RequestVoteRpc {
            term: 1,
            candidate_id: "node2".to_string(),
            last_log_index: 5,
            last_log_term: 2, // Higher term
        };

        let response = handle_request_vote(&node, &rpc).unwrap();
        assert!(response.vote_granted);
    }

    #[test]
    fn test_check_election_won() {
        assert!(check_election_won(2, 2)); // Exactly needed
        assert!(check_election_won(3, 2)); // More than needed
        assert!(!check_election_won(1, 2)); // Not enough
    }
}
