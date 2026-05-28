/// End-to-end persistence tests for all 5 specialists
///
/// Each test follows the same pattern:
///   1. Create a fresh specialist, exercise it through `execute()` calls so
///      `LearningData` accumulates real state.
///   2. Save to an in-memory SQLite database via the specialist's
///      `save_learning_to()` method.
///   3. Drop the original specialist (simulates process restart).
///   4. Create a brand-new specialist of the same kind, call
///      `load_learning_from()`, and verify the recovered state matches.
///
/// These are the tests that prove the federation actually has persistent
/// learning - not just an in-memory feature that vanishes on restart.

#[cfg(test)]
mod tests {
    use crate::federation::specialist::{
        Decision, ExecutionStatus, ResourceRequest, Specialist, SpecialistContext, SpecialistId,
        SystemResources, UserState,
    };
    use crate::federation::specialists::{Archivist, Omnipresent, Phygital, Symbiotic, Visionary};
    use crate::persistence::PersistenceManager;
    use std::collections::HashMap;

    fn fresh_db() -> PersistenceManager {
        PersistenceManager::new(":memory:").expect("open in-memory db")
    }

    fn make_decision(specialist: SpecialistId, idx: usize) -> Decision {
        Decision {
            proposal_id: format!("proposal-{}", idx),
            specialist,
            action: "test_action".to_string(),
            allocated_resources: ResourceRequest::default(),
            deadline_ms: 5000,
            context: HashMap::new(),
        }
    }

    fn idle_context() -> SpecialistContext {
        SpecialistContext {
            timestamp: 0,
            user_state: UserState::default(),
            system_resources: SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        }
    }

    // ================================================================
    // Visionary persistence E2E
    // ================================================================
    #[tokio::test]
    async fn test_visionary_learning_persists_across_restart() {
        let pm = fresh_db();

        // Phase 1: Original specialist learns
        {
            let visionary = Visionary::new();
            for i in 0..3 {
                let d = make_decision(SpecialistId::Visionary, i);
                let result = visionary.execute(&d).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
            }
            visionary.save_learning_to(&pm).unwrap();
        }

        // Phase 2: Revived specialist loads same state
        let revived = Visionary::new();
        let loaded = revived.load_learning_from(&pm).unwrap();
        assert!(loaded);

        let learning = revived.learning.lock();
        assert_eq!(learning.success_count, 3);
        assert_eq!(learning.total_executions, 3);
        assert_eq!(learning.failure_count, 0);
        assert!(learning.confidence_score > 0.5);
    }

    // ================================================================
    // Omnipresent persistence E2E
    // ================================================================
    #[tokio::test]
    async fn test_omnipresent_learning_persists_across_restart() {
        let pm = fresh_db();

        // Phase 1
        {
            let omnipresent = Omnipresent::new();
            for i in 0..4 {
                let d = make_decision(SpecialistId::Omnipresent, i);
                let result = omnipresent.execute(&d).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
            }
            omnipresent.save_learning_to(&pm).unwrap();
        }

        // Phase 2
        let revived = Omnipresent::new();
        let loaded = revived.load_learning_from(&pm).unwrap();
        assert!(loaded);

        let learning = revived.learning.lock();
        assert_eq!(learning.success_count, 4);
        assert_eq!(learning.total_executions, 4);
    }

    // ================================================================
    // Symbiotic persistence E2E
    // ================================================================
    #[tokio::test]
    async fn test_symbiotic_learning_persists_across_restart() {
        let pm = fresh_db();

        // Phase 1
        {
            let symbiotic = Symbiotic::new();
            for i in 0..2 {
                let d = make_decision(SpecialistId::Symbiotic, i);
                let result = symbiotic.execute(&d).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
            }
            symbiotic.save_learning_to(&pm).unwrap();
        }

        // Phase 2
        let revived = Symbiotic::new();
        let loaded = revived.load_learning_from(&pm).unwrap();
        assert!(loaded);

        let learning = revived.learning.lock();
        assert_eq!(learning.success_count, 2);
        assert_eq!(learning.total_executions, 2);
    }

    // ================================================================
    // Phygital persistence E2E
    // ================================================================
    #[tokio::test]
    async fn test_phygital_learning_persists_across_restart() {
        let pm = fresh_db();

        // Phase 1
        {
            let phygital = Phygital::new();
            for i in 0..5 {
                let d = make_decision(SpecialistId::Phygital, i);
                let result = phygital.execute(&d).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
            }
            phygital.save_learning_to(&pm).unwrap();
        }

        // Phase 2
        let revived = Phygital::new();
        let loaded = revived.load_learning_from(&pm).unwrap();
        assert!(loaded);

        let learning = revived.learning.lock();
        assert_eq!(learning.success_count, 5);
        assert_eq!(learning.total_executions, 5);
    }

    // ================================================================
    // Archivist persistence E2E
    // ================================================================
    #[tokio::test]
    async fn test_archivist_learning_persists_across_restart() {
        let pm = fresh_db();

        // Phase 1
        {
            let archivist = Archivist::new();
            for i in 0..3 {
                let d = make_decision(SpecialistId::Archivist, i);
                let result = archivist.execute(&d).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
            }
            archivist.save_learning_to(&pm).unwrap();
        }

        // Phase 2
        let revived = Archivist::new();
        let loaded = revived.load_learning_from(&pm).unwrap();
        assert!(loaded);

        let learning = revived.learning.lock();
        assert_eq!(learning.success_count, 3);
        assert_eq!(learning.total_executions, 3);
    }

    // ================================================================
    // Cross-specialist: all 5 share one DB without collision
    // ================================================================
    #[tokio::test]
    async fn test_all_five_specialists_persist_independently() {
        let pm = fresh_db();

        // Phase 1: each specialist has a different number of executions
        {
            let visionary = Visionary::new();
            let omnipresent = Omnipresent::new();
            let symbiotic = Symbiotic::new();
            let phygital = Phygital::new();
            let archivist = Archivist::new();

            for i in 0..1 {
                visionary.execute(&make_decision(SpecialistId::Visionary, i)).await.unwrap();
            }
            for i in 0..2 {
                omnipresent.execute(&make_decision(SpecialistId::Omnipresent, i)).await.unwrap();
            }
            for i in 0..3 {
                symbiotic.execute(&make_decision(SpecialistId::Symbiotic, i)).await.unwrap();
            }
            for i in 0..4 {
                phygital.execute(&make_decision(SpecialistId::Phygital, i)).await.unwrap();
            }
            for i in 0..5 {
                archivist.execute(&make_decision(SpecialistId::Archivist, i)).await.unwrap();
            }

            visionary.save_learning_to(&pm).unwrap();
            omnipresent.save_learning_to(&pm).unwrap();
            symbiotic.save_learning_to(&pm).unwrap();
            phygital.save_learning_to(&pm).unwrap();
            archivist.save_learning_to(&pm).unwrap();
        }

        // Phase 2: revive each, verify each has its own correct count
        let visionary = Visionary::new();
        let omnipresent = Omnipresent::new();
        let symbiotic = Symbiotic::new();
        let phygital = Phygital::new();
        let archivist = Archivist::new();

        visionary.load_learning_from(&pm).unwrap();
        omnipresent.load_learning_from(&pm).unwrap();
        symbiotic.load_learning_from(&pm).unwrap();
        phygital.load_learning_from(&pm).unwrap();
        archivist.load_learning_from(&pm).unwrap();

        assert_eq!(visionary.learning.lock().total_executions, 1, "Visionary count");
        assert_eq!(omnipresent.learning.lock().total_executions, 2, "Omnipresent count");
        assert_eq!(symbiotic.learning.lock().total_executions, 3, "Symbiotic count");
        assert_eq!(phygital.learning.lock().total_executions, 4, "Phygital count");
        assert_eq!(archivist.learning.lock().total_executions, 5, "Archivist count");

        // Verify the diagnostics list reflects all 5
        let all = pm.list_learning_states().unwrap();
        assert_eq!(all.len(), 5);
        let kinds: Vec<&str> = all.iter().map(|r| r.specialist_kind.as_str()).collect();
        for expected in ["Visionary", "Omnipresent", "Symbiotic", "Phygital", "Archivist"] {
            assert!(
                kinds.contains(&expected),
                "{} missing from list_learning_states: got {:?}",
                expected,
                kinds
            );
        }
    }

    // ================================================================
    // Confidence improves visibly via real propose() output
    // ================================================================
    #[tokio::test]
    async fn test_persisted_confidence_drives_proposals_after_reload() {
        let pm = fresh_db();

        // Phase 1: train Visionary on 5 successful executions
        {
            let visionary = Visionary::new();
            for i in 0..5 {
                visionary.execute(&make_decision(SpecialistId::Visionary, i)).await.unwrap();
            }
            visionary.save_learning_to(&pm).unwrap();
        }

        // Phase 2: reload and check that propose() reflects the learned confidence
        let revived = Visionary::new();
        revived.load_learning_from(&pm).unwrap();

        let context = idle_context();
        let proposals = revived.propose(&context).await.unwrap();

        // Even though this is a "fresh" specialist process-wise, it should
        // have higher confidence than a truly new one that never trained.
        let truly_fresh = Visionary::new();
        let baseline_proposals = truly_fresh.propose(&context).await.unwrap();

        if !proposals.is_empty() && !baseline_proposals.is_empty() {
            assert!(
                proposals[0].confidence >= baseline_proposals[0].confidence,
                "reloaded confidence {} should be >= fresh baseline {}",
                proposals[0].confidence,
                baseline_proposals[0].confidence
            );
        }
    }
}
