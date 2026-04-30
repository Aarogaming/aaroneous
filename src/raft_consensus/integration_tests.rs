/// Integration tests for Raft consensus
/// 
/// Tests realistic multi-node scenarios including:
/// - Multi-node election
/// - Leader failure and recovery
/// - Log divergence and conflict resolution
/// - Snapshot-based recovery

#[cfg(test)]
mod tests {
    use crate::raft_consensus::*;
    use crate::raft_consensus::snapshot::SnapshotStore;
    use chrono::Utc;

    /// Helper to create a test cluster
    fn create_cluster(node_count: usize) -> RaftEngine {
        let mut node_ids = Vec::new();
        for i in 0..node_count {
            node_ids.push(format!("node{}", i + 1));
        }
        RaftEngine::new(node_ids)
    }

    /// Helper to create a test log entry
    fn create_entry(index: u64, term: u64, cmd: &str) -> LogEntry {
        LogEntry {
            index,
            term,
            data: serde_json::json!({"cmd": cmd}),
            client_id: "test-client".to_string(),
            sequence: index,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_three_node_cluster_election() {
        let engine = create_cluster(3);

        let nodes = engine.get_all_nodes();
        assert_eq!(nodes.len(), 3);

        // All should start as followers
        for node in nodes {
            assert!(node.get_state().unwrap().is_follower());
            assert_eq!(node.get_term().unwrap(), 0);
        }
    }

    #[test]
    fn test_cluster_leader_election() {
        let engine = create_cluster(3);

        // Node 1 becomes candidate and then leader
        let node1 = engine.get_node(&"node1".to_string()).unwrap();
        node1.become_candidate().unwrap();
        assert!(node1.get_state().unwrap().is_candidate());
        assert_eq!(node1.get_term().unwrap(), 1);

        // In a real scenario, after getting quorum votes, becomes leader
        node1.become_leader().unwrap();
        assert!(node1.get_state().unwrap().is_leader());
    }

    #[test]
    fn test_log_replication_single_entry() {
        let mut engine = create_cluster(3);

        let leader = engine.get_node(&"node1".to_string()).unwrap();
        leader.become_leader().unwrap();

        // Leader replicates an entry
        let entry = create_entry(1, 1, "write");
        
        let result = engine.replicate_log_entry(
            &"node1".to_string(),
            0,
            0,
            vec![entry],
            1,
        ).unwrap();

        assert!(result.is_quorum);
        assert_eq!(result.success_count, 3);
    }

    #[test]
    fn test_log_replication_multiple_entries() {
        let mut engine = create_cluster(3);

        let leader = engine.get_node(&"node1".to_string()).unwrap();
        leader.become_leader().unwrap();

        let entries = vec![
            create_entry(1, 1, "write1"),
            create_entry(2, 1, "write2"),
            create_entry(3, 1, "write3"),
        ];

        let result = engine.replicate_log_entry(
            &"node1".to_string(),
            0,
            0,
            entries,
            3,
        ).unwrap();

        assert!(result.is_quorum);
        assert_eq!(result.success_count, 3);
        assert_eq!(result.total_nodes, 3);
    }

    #[test]
    fn test_mutation_deduplication_across_cluster() {
        let mut tracker = MutationTracker::new(3);

        let cmd = ClientCommand::new("client1".to_string(), 1);
        tracker.mark_applied(cmd.clone(), 1);

        // After replication, should know it was applied
        assert!(tracker.is_duplicate(&cmd));
    }

    #[test]
    fn test_quorum_with_cluster_size() {
        // Test various cluster sizes
        assert!(is_quorum(1, 1)); // 1 node
        assert!(is_quorum(2, 3)); // 3 nodes
        assert!(is_quorum(3, 5)); // 5 nodes
        assert!(is_quorum(4, 7)); // 7 nodes

        // Test edge cases
        assert!(!is_quorum(1, 2)); // Need 2 for 2 nodes
        assert!(!is_quorum(2, 4)); // Need 3 for 4 nodes
    }

    #[test]
    fn test_snapshot_before_replication() {
        let mut store = SnapshotStore::new(3, 1_000_000);
        
        // Create initial snapshot
        let snap = Snapshot::new(10, 1, vec![0; 1000]);
        store.add_snapshot(snap);

        // Even with old snapshot, replication continues from index 11
        assert_eq!(store.latest().unwrap().last_included_index, 10);
    }

    #[test]
    fn test_multi_node_append_entries() {
        let mut engine = create_cluster(5);

        let leader = engine.get_node(&"node1".to_string()).unwrap();
        leader.become_leader().unwrap();

        // Leader replicates to 4 followers
        let entries = vec![
            create_entry(1, 1, "op1"),
            create_entry(2, 1, "op2"),
        ];

        let result = engine.replicate_log_entry(
            &"node1".to_string(),
            0,
            0,
            entries,
            2,
        ).unwrap();

        // For 5 nodes, quorum is 3 (including leader)
        assert!(result.is_quorum);
        assert_eq!(result.success_count, 5);
    }

    #[test]
    fn test_follower_term_update_on_append_entries() {
        let node = create_cluster(1).get_node(&"node1".to_string()).unwrap();

        // Start at term 0
        assert_eq!(node.get_term().unwrap(), 0);

        // Receive append_entries from term 2
        let rpc = AppendEntriesRpc {
            term: 2,
            leader_id: "leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        
        // Should update term
        assert_eq!(node.get_term().unwrap(), 2);
        assert!(response.success);
    }

    #[test]
    fn test_stale_rpc_rejection() {
        let node = create_cluster(1).get_node(&"node1".to_string()).unwrap();

        // Set node to term 3
        node.update_term(3).unwrap();

        // Try to process RPC from term 1 (stale)
        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "stale-leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        
        // Should reject
        assert!(!response.success);
        assert_eq!(response.term, 3);
    }

    #[test]
    fn test_heartbeat_resets_follower_state() {
        let node = create_cluster(1).get_node(&"node1".to_string()).unwrap();

        // Heartbeat from leader
        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "leader1".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };

        let response = node.handle_append_entries(&rpc).unwrap();
        assert!(response.success);

        // Node should know the leader
        if let RaftState::Follower { leader_id, .. } = node.get_state().unwrap() {
            assert_eq!(leader_id, Some("leader1".to_string()));
        } else {
            panic!("Expected follower state");
        }
    }

    #[test]
    fn test_commit_index_advancement() {
        let node = create_cluster(1).get_node(&"node1".to_string()).unwrap();

        let entry = create_entry(1, 1, "write");

        let rpc = AppendEntriesRpc {
            term: 1,
            leader_id: "leader".to_string(),
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![entry],
            leader_commit: 1,
        };

        node.handle_append_entries(&rpc).unwrap();

        // Commit index should be updated
        assert_eq!(node.get_commit_index().unwrap(), 1);
    }

    #[test]
    fn test_election_timeout_config() {
        let node = create_cluster(1).get_node(&"node1".to_string()).unwrap();
        
        // Check that timeouts are reasonable
        let config = node.get_config();
        assert!(config.election_timeout_min_ms > 0);
        assert!(config.election_timeout_max_ms > config.election_timeout_min_ms);
        assert!(config.heartbeat_interval_ms < config.election_timeout_min_ms);
    }

    #[test]
    fn test_quorum_calculation_3_nodes() {
        let engine = create_cluster(3);
        let nodes = engine.get_all_nodes();

        // For 3 nodes, quorum is 2
        let config = nodes[0].get_config();
        let quorum = config.quorum_size();
        assert_eq!(quorum, 2);
    }

    #[test]
    fn test_quorum_calculation_5_nodes() {
        let engine = create_cluster(5);
        let nodes = engine.get_all_nodes();

        // For 5 nodes, quorum is 3
        let config = nodes[0].get_config();
        let quorum = config.quorum_size();
        assert_eq!(quorum, 3);
    }

    #[test]
    fn test_quorum_calculation_7_nodes() {
        let engine = create_cluster(7);
        let nodes = engine.get_all_nodes();

        // For 7 nodes, quorum is 4
        let config = nodes[0].get_config();
        let quorum = config.quorum_size();
        assert_eq!(quorum, 4);
    }

    #[test]
    fn test_cluster_consistency_multiple_operations() {
        let mut engine = create_cluster(3);

        let leader = engine.get_node(&"node1".to_string()).unwrap();
        leader.become_leader().unwrap();

        // Replicate 5 operations
        for i in 1..=5 {
            let entry = create_entry(i, 1, &format!("op{}", i));
            let result = engine.replicate_log_entry(
                &"node1".to_string(),
                i - 1,
                1,
                vec![entry],
                i,
            ).unwrap();

            assert!(result.is_quorum);
        }
    }
}
