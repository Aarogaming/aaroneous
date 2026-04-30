/// Integration tests for `SpecialistHost`
///
/// These tests exercise the full lifecycle: start (load), run (checkpoint),
/// shutdown (final save), restart (load again). The Visionary specialist is
/// used as the test subject because its execution path is the simplest.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::federation::specialist::{
        Decision, ExecutionStatus, ResourceRequest, Specialist, SpecialistId,
    };
    use crate::federation::specialists::Visionary;
    use crate::persistence::PersistenceManager;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    fn fresh_persistence() -> SharedPersistence {
        let pm = PersistenceManager::new(":memory:").expect("open in-memory db");
        shared(pm)
    }

    fn make_decision(idx: usize) -> Decision {
        Decision {
            proposal_id: format!("p{}", idx),
            specialist: SpecialistId::Visionary,
            action: "test".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: HashMap::new(),
        }
    }

    // =================================================================
    // Basic lifecycle: NotStarted -> Running -> ShutDown
    // =================================================================
    #[tokio::test]
    async fn test_initial_state_is_not_started() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        assert_eq!(host.state().await, HostState::NotStarted);
    }

    #[tokio::test]
    async fn test_state_after_start_is_running() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        host.start().await.unwrap();
        assert_eq!(host.state().await, HostState::Running);
    }

    #[tokio::test]
    async fn test_state_after_shutdown_is_shut_down() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        host.start().await.unwrap();
        host.shutdown().await.unwrap();
        assert_eq!(host.state().await, HostState::ShutDown);
    }

    #[tokio::test]
    async fn test_double_start_errors() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        host.start().await.unwrap();
        let result = host.start().await;
        assert!(matches!(result, Err(HostError::AlreadyStarted)));
    }

    #[tokio::test]
    async fn test_double_shutdown_errors_on_second() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        host.start().await.unwrap();
        host.shutdown().await.unwrap();
        let result = host.shutdown().await;
        assert!(matches!(result, Err(HostError::AlreadyShutDown)));
    }

    #[tokio::test]
    async fn test_shutdown_without_start_succeeds_as_cleanup() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        // Skipping start() entirely - shutdown should be a no-op cleanup
        host.shutdown().await.unwrap();
        assert_eq!(host.state().await, HostState::ShutDown);
    }

    #[tokio::test]
    async fn test_checkpoint_before_start_errors() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        let result = host.checkpoint_now().await;
        assert!(matches!(result, Err(HostError::NotStarted)));
    }

    // =================================================================
    // Persistence key matches the specialist's constant
    // =================================================================
    #[tokio::test]
    async fn test_persistence_key_matches_specialist() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v, pm, HostConfig::manual_only());
        assert_eq!(host.persistence_key(), Visionary::PERSISTENCE_KEY);
        assert_eq!(host.persistence_key(), "Visionary");
    }

    // =================================================================
    // Start with no prior state: specialist remains neutral
    // =================================================================
    #[tokio::test]
    async fn test_start_with_no_prior_state_keeps_neutral() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v.clone(), pm, HostConfig::manual_only());
        host.start().await.unwrap();

        let learning = v.learning.lock();
        assert_eq!(learning.total_executions, 0);
    }

    // =================================================================
    // Manual checkpoint persists current learning state
    // =================================================================
    #[tokio::test]
    async fn test_manual_checkpoint_persists_state() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v.clone(), pm.clone(), HostConfig::manual_only());
        host.start().await.unwrap();

        // Train the specialist
        for i in 0..3 {
            v.execute(&make_decision(i)).await.unwrap();
        }
        host.checkpoint_now().await.unwrap();

        // Verify by loading directly via the persistence layer
        let pm_guard = pm.lock().await;
        let record = pm_guard
            .load_learning_state(Visionary::PERSISTENCE_KEY)
            .unwrap()
            .expect("checkpoint should have written a row");
        assert_eq!(record.total_executions, 3);
        assert_eq!(record.success_count, 3);
    }

    // =================================================================
    // Auto-checkpoint: learning state is saved automatically
    // =================================================================
    #[tokio::test]
    async fn test_auto_checkpoint_saves_periodically() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(
            v.clone(),
            pm.clone(),
            HostConfig::with_interval(Duration::from_millis(50)),
        );
        host.start().await.unwrap();
        host.spawn_checkpoint_loop().await;

        // Train, then wait for the auto-checkpoint to fire
        for i in 0..2 {
            v.execute(&make_decision(i)).await.unwrap();
        }
        // Sleep longer than the interval so a checkpoint definitely happens
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check that the row exists via direct query
        let pm_guard = pm.lock().await;
        let record = pm_guard
            .load_learning_state(Visionary::PERSISTENCE_KEY)
            .unwrap();
        assert!(record.is_some(), "auto-checkpoint should have written a row");
        assert_eq!(record.unwrap().total_executions, 2);

        drop(pm_guard);
        host.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_zero_interval_does_not_spawn_loop() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v.clone(), pm, HostConfig::manual_only());
        host.start().await.unwrap();
        host.spawn_checkpoint_loop().await; // Should be a no-op
        // No assertion error means the path is taken correctly
        host.shutdown().await.unwrap();
    }

    // =================================================================
    // Full restart: train host A, shut it down, start host B with
    // same DB - host B's specialist has the trained state
    // =================================================================
    #[tokio::test]
    async fn test_full_restart_recovers_learning() {
        let pm = fresh_persistence();

        // === Host A: train and save ===
        let trained_count;
        {
            let v = Arc::new(Visionary::new());
            let host = SpecialistHost::new(v.clone(), pm.clone(), HostConfig::manual_only());
            host.start().await.unwrap();
            for i in 0..7 {
                v.execute(&make_decision(i)).await.unwrap();
            }
            trained_count = v.learning.lock().total_executions;
            host.shutdown().await.unwrap(); // shutdown does final save
        }

        assert_eq!(trained_count, 7);

        // === Host B: cold start, load via host lifecycle ===
        let v_b = Arc::new(Visionary::new());
        let host_b = SpecialistHost::new(v_b.clone(), pm, HostConfig::manual_only());

        // Before start: neutral
        assert_eq!(v_b.learning.lock().total_executions, 0);

        host_b.start().await.unwrap();

        // After start: trained
        assert_eq!(v_b.learning.lock().total_executions, 7);
        assert_eq!(v_b.learning.lock().success_count, 7);

        host_b.shutdown().await.unwrap();
    }

    // =================================================================
    // Shutdown does a final save even if checkpoint loop hasn't fired
    // =================================================================
    #[tokio::test]
    async fn test_shutdown_does_final_save() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(
            v.clone(),
            pm.clone(),
            // 1-hour interval: the loop will never fire during this test
            HostConfig::with_interval(Duration::from_secs(3600)),
        );
        host.start().await.unwrap();
        host.spawn_checkpoint_loop().await;

        // Train but don't manually checkpoint
        for i in 0..4 {
            v.execute(&make_decision(i)).await.unwrap();
        }

        // Shutdown should save before exiting
        host.shutdown().await.unwrap();

        // Verify the trained state was persisted by the final save
        let pm_guard = pm.lock().await;
        let record = pm_guard
            .load_learning_state(Visionary::PERSISTENCE_KEY)
            .unwrap()
            .expect("shutdown should have written a final row");
        assert_eq!(record.total_executions, 4);
    }

    // =================================================================
    // Multiple hosts on same DB: each persists under its own key
    // =================================================================
    #[tokio::test]
    async fn test_two_hosts_share_db_without_collision() {
        use crate::federation::specialists::Symbiotic;

        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let s = Arc::new(Symbiotic::new());

        let host_v = SpecialistHost::new(v.clone(), pm.clone(), HostConfig::manual_only());
        let host_s = SpecialistHost::new(s.clone(), pm.clone(), HostConfig::manual_only());

        host_v.start().await.unwrap();
        host_s.start().await.unwrap();

        // Different number of executions for each
        for i in 0..3 {
            v.execute(&make_decision(i)).await.unwrap();
        }
        for i in 0..5 {
            let d = Decision {
                proposal_id: format!("s{}", i),
                specialist: SpecialistId::Symbiotic,
                action: "test".to_string(),
                allocated_resources: ResourceRequest::default(),
                deadline_ms: 5000,
                context: HashMap::new(),
            };
            s.execute(&d).await.unwrap();
        }

        host_v.shutdown().await.unwrap();
        host_s.shutdown().await.unwrap();

        // Each row should reflect its own host's learning
        let pm_guard = pm.lock().await;
        let v_rec = pm_guard
            .load_learning_state(Visionary::PERSISTENCE_KEY)
            .unwrap()
            .unwrap();
        let s_rec = pm_guard
            .load_learning_state(Symbiotic::PERSISTENCE_KEY)
            .unwrap()
            .unwrap();

        assert_eq!(v_rec.total_executions, 3, "Visionary count");
        assert_eq!(s_rec.total_executions, 5, "Symbiotic count");
        assert_ne!(v_rec.specialist_kind, s_rec.specialist_kind);
    }

    // =================================================================
    // Specialist Arc is shareable: host doesn't take exclusive ownership
    // =================================================================
    #[tokio::test]
    async fn test_specialist_arc_remains_usable() {
        let pm = fresh_persistence();
        let v = Arc::new(Visionary::new());
        let host = SpecialistHost::new(v.clone(), pm, HostConfig::manual_only());

        // Caller still has v
        assert_eq!(Arc::strong_count(&v), 2); // host + caller

        host.start().await.unwrap();

        // Use the specialist directly via the original Arc
        v.execute(&make_decision(0)).await.unwrap();
        assert_eq!(v.learning.lock().total_executions, 1);

        // Host's view via specialist() also reflects the change
        let from_host = host.specialist();
        assert_eq!(from_host.learning.lock().total_executions, 1);

        host.shutdown().await.unwrap();
    }
}
