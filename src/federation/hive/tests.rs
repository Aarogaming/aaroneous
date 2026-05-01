/// Integration tests for `Federation`
///
/// Covers the orchestration on top of `SpecialistHost`: building partial
/// federations, starting/shutting down all hosts, restart recovery across
/// the whole hive, and per-host config overrides.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::federation::host::HostConfig;
    use crate::federation::specialist::{
        Decision, ExecutionStatus, ResourceRequest, Specialist, SpecialistId,
    };
    use crate::federation::specialists::{Archivist, Omnipresent, Phygital, Symbiotic, Visionary};
    use crate::persistence::PersistenceManager;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    fn fresh_pm() -> PersistenceManager {
        PersistenceManager::new(":memory:").expect("open in-memory db")
    }

    fn make_decision(specialist: SpecialistId, idx: usize) -> Decision {
        Decision {
            proposal_id: format!("p{}-{}", idx, specialist as u8),
            specialist,
            action: "test".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: HashMap::new(),
        }
    }

    // ===============================================================
    // Construction
    // ===============================================================

    #[tokio::test]
    async fn test_empty_federation_has_zero_enabled() {
        let fed = Federation::builder(fresh_pm()).build();
        assert_eq!(fed.enabled_count(), 0);
        assert!(fed.visionary().is_none());
        assert!(fed.omnipresent().is_none());
        assert!(fed.symbiotic().is_none());
        assert!(fed.phygital().is_none());
        assert!(fed.archivist().is_none());
    }

    #[tokio::test]
    async fn test_with_visionary_only() {
        let fed = Federation::builder(fresh_pm()).with_visionary().build();
        assert_eq!(fed.enabled_count(), 1);
        assert!(fed.visionary().is_some());
        assert!(fed.omnipresent().is_none());
    }

    #[tokio::test]
    async fn test_with_all() {
        let fed = Federation::builder(fresh_pm()).with_all().build();
        assert_eq!(fed.enabled_count(), 5);
        assert!(fed.visionary().is_some());
        assert!(fed.omnipresent().is_some());
        assert!(fed.symbiotic().is_some());
        assert!(fed.phygital().is_some());
        assert!(fed.archivist().is_some());
    }

    #[tokio::test]
    async fn test_partial_subset() {
        let fed = Federation::builder(fresh_pm())
            .with_visionary()
            .with_archivist()
            .build();
        assert_eq!(fed.enabled_count(), 2);
        assert!(fed.visionary().is_some());
        assert!(fed.archivist().is_some());
        assert!(fed.omnipresent().is_none());
        assert!(fed.symbiotic().is_none());
        assert!(fed.phygital().is_none());
    }

    #[tokio::test]
    async fn test_config_carries_through_builder() {
        let fed = Federation::builder(fresh_pm())
            .checkpoint_every(Duration::from_secs(5))
            .with_visionary()
            .build();
        assert_eq!(
            fed.config().default_checkpoint_interval,
            Duration::from_secs(5)
        );
    }

    #[tokio::test]
    async fn test_manual_checkpoints_disables_loop() {
        let fed = Federation::builder(fresh_pm())
            .manual_checkpoints()
            .with_visionary()
            .build();
        assert_eq!(fed.config().default_checkpoint_interval, Duration::ZERO);
    }

    // ===============================================================
    // Specialist instance injection (the with_*_instance variants)
    // ===============================================================

    #[tokio::test]
    async fn test_inject_pre_built_visionary() {
        let v = Arc::new(Visionary::new());
        // Train the instance before adding to the federation
        v.execute(&make_decision(SpecialistId::Visionary, 0))
            .await
            .unwrap();

        let fed = Federation::builder(fresh_pm())
            .with_visionary_instance(v.clone())
            .build();

        let from_fed = fed.visionary().unwrap();
        // The federation holds the same Arc instance
        assert!(Arc::ptr_eq(&v, &from_fed));
        assert_eq!(from_fed.learning.lock().total_executions, 1);
    }

    // ===============================================================
    // Lifecycle: start_all -> shutdown_all
    // ===============================================================

    #[tokio::test]
    async fn test_start_all_succeeds_on_empty_federation() {
        let fed = Federation::builder(fresh_pm()).build();
        fed.start_all().await.expect("empty start should succeed");
    }

    #[tokio::test]
    async fn test_start_all_then_shutdown_all() {
        let fed = Federation::builder(fresh_pm()).with_all().build();
        fed.start_all().await.unwrap();
        fed.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_shutdown_without_start_succeeds() {
        let fed = Federation::builder(fresh_pm()).with_all().build();
        // No start; shutdown should still succeed (each host's shutdown
        // permits being called from NotStarted as a no-op cleanup).
        fed.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_double_start_returns_errors_for_each_host() {
        let fed = Federation::builder(fresh_pm())
            .with_visionary()
            .with_symbiotic()
            .build();
        fed.start_all().await.unwrap();

        let result = fed.start_all().await;
        let err = result.expect_err("second start should fail");
        assert_eq!(err.errors.len(), 2);
        for (kind, _) in &err.errors {
            assert!(kind == "Visionary" || kind == "Symbiotic");
        }
    }

    // ===============================================================
    // Cross-specialist persistence: the whole hive recovers state
    // ===============================================================

    #[tokio::test]
    async fn test_full_federation_restart_recovers_all_specialist_state() {
        let pm = fresh_pm();

        // === Generation 1: build, train, shutdown ===
        let pm_for_gen1 = PersistenceManager::new(":memory:").unwrap();
        // (Note: each PersistenceManager is its own connection. To share
        // state across two builds in this test, we use the same file path.
        // ":memory:" doesn't share - so we need a temp file.)
        drop(pm_for_gen1);

        let tmp_path = std::env::temp_dir().join(format!(
            "aaroneous-test-{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let tmp_path_str = tmp_path.to_string_lossy().to_string();

        // Cleanup helper closure
        let cleanup = || {
            let _ = std::fs::remove_file(&tmp_path);
        };

        // === Generation 1 ===
        {
            let pm_g1 = PersistenceManager::new(&tmp_path_str).unwrap();
            let fed = Federation::builder(pm_g1).with_all().build();
            fed.start_all().await.unwrap();

            // Train each specialist a different number of times
            for i in 0..2 {
                fed.visionary().unwrap()
                    .execute(&make_decision(SpecialistId::Visionary, i))
                    .await
                    .unwrap();
            }
            for i in 0..3 {
                fed.omnipresent().unwrap()
                    .execute(&make_decision(SpecialistId::Omnipresent, i))
                    .await
                    .unwrap();
            }
            for i in 0..4 {
                fed.symbiotic().unwrap()
                    .execute(&make_decision(SpecialistId::Symbiotic, i))
                    .await
                    .unwrap();
            }
            for i in 0..1 {
                fed.phygital().unwrap()
                    .execute(&make_decision(SpecialistId::Phygital, i))
                    .await
                    .unwrap();
            }
            for i in 0..5 {
                fed.archivist().unwrap()
                    .execute(&make_decision(SpecialistId::Archivist, i))
                    .await
                    .unwrap();
            }

            fed.shutdown_all().await.unwrap();
        }

        // Drop pm/fed entirely. Open DB fresh - simulates process exit.
        let _ = pm; // unused in this test (above)

        // === Generation 2 ===
        {
            let pm_g2 = PersistenceManager::new(&tmp_path_str).unwrap();
            let fed = Federation::builder(pm_g2).with_all().build();

            // Each specialist starts neutral
            assert_eq!(fed.visionary().unwrap().learning.lock().total_executions, 0);

            fed.start_all().await.unwrap();

            // After start_all: each is loaded
            assert_eq!(
                fed.visionary().unwrap().learning.lock().total_executions,
                2,
                "Visionary recovered count"
            );
            assert_eq!(
                fed.omnipresent().unwrap().learning.lock().total_executions,
                3,
                "Omnipresent recovered count"
            );
            assert_eq!(
                fed.symbiotic().unwrap().learning.lock().total_executions,
                4,
                "Symbiotic recovered count"
            );
            assert_eq!(
                fed.phygital().unwrap().learning.lock().total_executions,
                1,
                "Phygital recovered count"
            );
            assert_eq!(
                fed.archivist().unwrap().learning.lock().total_executions,
                5,
                "Archivist recovered count"
            );

            fed.shutdown_all().await.unwrap();
        }

        cleanup();
    }

    // ===============================================================
    // checkpoint_all triggers a manual save on every host
    // ===============================================================

    #[tokio::test]
    async fn test_checkpoint_all_persists_every_specialist() {
        let pm = fresh_pm();
        let fed = Federation::builder(pm).with_all().build();
        fed.start_all().await.unwrap();

        // Train one execution each
        fed.visionary().unwrap()
            .execute(&make_decision(SpecialistId::Visionary, 0))
            .await
            .unwrap();
        fed.archivist().unwrap()
            .execute(&make_decision(SpecialistId::Archivist, 0))
            .await
            .unwrap();

        fed.checkpoint_all().await.unwrap();

        // Verify via direct DB query (using the federation's persistence)
        let pm_guard = fed.persistence().lock().await.list_learning_states().unwrap();
        // Five rows total (every host writes a row even with zero executions)
        assert_eq!(pm_guard.len(), 5);

        let v_row = pm_guard.iter().find(|r| r.specialist_kind == "Visionary").unwrap();
        let a_row = pm_guard.iter().find(|r| r.specialist_kind == "Archivist").unwrap();
        assert_eq!(v_row.total_executions, 1);
        assert_eq!(a_row.total_executions, 1);

        fed.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_checkpoint_all_before_start_returns_errors() {
        let fed = Federation::builder(fresh_pm()).with_visionary().build();
        let result = fed.checkpoint_all().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].0, "Visionary");
    }

    // ===============================================================
    // spawn_checkpoint_loops: auto-saving
    // ===============================================================

    #[tokio::test]
    async fn test_spawn_checkpoint_loops_auto_saves() {
        let pm = fresh_pm();
        let fed = Federation::builder(pm)
            .checkpoint_every(Duration::from_millis(50))
            .with_visionary()
            .with_symbiotic()
            .build();

        fed.start_all().await.unwrap();
        fed.spawn_checkpoint_loops().await;

        // Train both specialists
        fed.visionary().unwrap()
            .execute(&make_decision(SpecialistId::Visionary, 0))
            .await
            .unwrap();
        fed.symbiotic().unwrap()
            .execute(&make_decision(SpecialistId::Symbiotic, 0))
            .await
            .unwrap();

        // Wait long enough for auto-save to fire
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Both should be persisted
        let states = fed.persistence().lock().await.list_learning_states().unwrap();
        assert_eq!(states.len(), 2);
        for state in &states {
            assert_eq!(state.total_executions, 1);
        }

        fed.shutdown_all().await.unwrap();
    }

    #[tokio::test]
    async fn test_manual_checkpoints_doesnt_spawn_loops() {
        let fed = Federation::builder(fresh_pm())
            .manual_checkpoints()
            .with_visionary()
            .build();
        fed.start_all().await.unwrap();
        fed.spawn_checkpoint_loops().await; // Should be a no-op for zero interval
        // No assertion of internal state - test passes if it doesn't hang/panic
        fed.shutdown_all().await.unwrap();
    }

    // ===============================================================
    // Per-host config overrides
    // ===============================================================

    #[tokio::test]
    async fn test_per_host_config_override() {
        let custom = HostConfig::with_interval(Duration::from_secs(120));
        let fed = Federation::builder(fresh_pm())
            .checkpoint_every(Duration::from_secs(30))      // federation default
            .with_visionary_host_config(custom)             // host override
            .with_symbiotic()                               // uses default
            .build();

        // Visionary's host has the custom interval; Symbiotic has the default.
        // We can't easily inspect the host config without exposing it, but
        // we can verify the federation builds and starts without issue.
        fed.start_all().await.unwrap();
        fed.shutdown_all().await.unwrap();
    }

    // ===============================================================
    // FederationErrors aggregation
    // ===============================================================

    #[tokio::test]
    async fn test_federation_errors_display_single() {
        let fed = Federation::builder(fresh_pm()).with_visionary().build();
        let result = fed.checkpoint_all().await; // before start: NotStarted error
        let err = result.unwrap_err();
        let display = err.to_string();
        assert!(display.contains("Visionary"), "got: {}", display);
    }

    #[tokio::test]
    async fn test_federation_errors_display_multiple() {
        let fed = Federation::builder(fresh_pm())
            .with_visionary()
            .with_symbiotic()
            .with_archivist()
            .build();
        let result = fed.checkpoint_all().await;
        let err = result.unwrap_err();
        let display = err.to_string();
        assert!(display.contains("3"), "should mention 3 errors: {}", display);
    }
}
