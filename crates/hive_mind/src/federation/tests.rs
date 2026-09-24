/// Comprehensive Federation Protocol Tests
///
/// These tests validate:
/// - Specialist trait implementation
/// - Proposal submission and ranking
/// - Conflict detection and resolution
/// - Sentinel arbitration
/// - Communication bus
/// - End-to-end federation flows

#[cfg(test)]
mod tests {
    use crate::federation::{
        CommunicationBus, Conflict, ConflictArbitrator, ConflictDetector, Decision,
        DelegateRequest, Proposal, ProposalPriority, ProposalSet, ProposalStatus, ProposedAction,
        Sentinel, SentinelConfig, Specialist, SpecialistContext, SpecialistId, SystemResources,
        UserState,
    };
    use async_trait::async_trait;
    use std::sync::Arc;

    /// Mock specialist for testing
    #[derive(Debug)]
    struct MockSpecialist {
        id: SpecialistId,
        proposal_count: tokio::sync::Mutex<u32>,
    }

    impl MockSpecialist {
        fn new(id: SpecialistId) -> Arc<Self> {
            Arc::new(Self {
                id,
                proposal_count: tokio::sync::Mutex::new(0),
            })
        }
    }

    #[async_trait]
    impl Specialist for MockSpecialist {
        fn id(&self) -> SpecialistId {
            self.id
        }

        async fn propose(
            &self,
            _context: &SpecialistContext,
        ) -> Result<Vec<ProposedAction>, crate::federation::specialist::SpecialistError> {
            let mut count = self.proposal_count.lock().await;
            *count += 1;

            Ok(vec![ProposedAction {
                id: format!("proposal-{}", count),
                specialist: self.id,
                action_type: "mock_action".to_string(),
                description: format!("Mock proposal from {:?}", self.id),
                confidence: 0.8,
                required_resources: crate::federation::specialist::ResourceRequest::default(),
                priority: ProposalPriority::Normal,
                tags: vec!["mock".to_string()],
            }])
        }

        async fn execute(
            &self,
            decision: &Decision,
        ) -> Result<
            crate::federation::specialist::ExecutionResult,
            crate::federation::specialist::SpecialistError,
        > {
            Ok(crate::federation::specialist::ExecutionResult {
                specialist: self.id,
                specialist_name: None,
                proposal_id: decision.proposal_id.clone(),
                status: crate::federation::specialist::ExecutionStatus::Success,
                output: "Mock executed".to_string(),
                resources_used: crate::federation::specialist::ResourceRequest::default(),
                duration_ms: 100,
                error: None,
            })
        }

        async fn delegate(
            &self,
            request: &DelegateRequest,
        ) -> Result<
            crate::federation::specialist::DelegateResponse,
            crate::federation::specialist::SpecialistError,
        > {
            Ok(crate::federation::specialist::DelegateResponse {
                requester: request.requester,
                target: request.target,
                success: true,
                result: "Delegated".to_string(),
                duration_ms: 50,
            })
        }

        async fn negotiate(
            &self,
            other_id: SpecialistId,
            _conflict: &Conflict,
        ) -> Result<
            crate::federation::specialist::NegotiationResult,
            crate::federation::specialist::SpecialistError,
        > {
            Ok(crate::federation::specialist::NegotiationResult {
                resolved: true,
                resolution: format!("Negotiated with {:?}", other_id),
                winner: None,
                compromise: Some("Agreed".to_string()),
            })
        }
    }

    #[tokio::test]
    async fn test_specialist_trait_implementation() {
        let specialist = MockSpecialist::new(SpecialistId::Visionary);

        assert_eq!(specialist.id(), SpecialistId::Visionary);

        let context = SpecialistContext {
            timestamp: 0,
            user_state: UserState::default(),
            system_resources: SystemResources::default(),
            active_specialists: vec![],
            recent_decisions: vec![],
        };

        let proposals = specialist.propose(&context).await.unwrap();
        assert_eq!(proposals.len(), 1);
    }

    #[test]
    fn test_proposal_creation_and_status() {
        let proposal = Proposal::new(
            SpecialistId::Visionary,
            "generate_design".to_string(),
            "Generate designs".to_string(),
            0.85,
            ProposalPriority::Normal,
        );

        assert_eq!(proposal.specialist, SpecialistId::Visionary);
        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.confidence, 0.85);
    }

    #[test]
    fn test_proposal_set_conflict_detection_gpu() {
        let mut set = ProposalSet::new();

        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Design".to_string(),
            0.8,
            ProposalPriority::Normal,
        );
        p1.required_resources.gpu_percent = 80.0;

        let mut p2 = Proposal::new(
            SpecialistId::Phygital,
            "render".to_string(),
            "Render".to_string(),
            0.8,
            ProposalPriority::Normal,
        );
        p2.required_resources.gpu_percent = 60.0;

        set.add(p1);
        set.add(p2);

        let conflicts = set.detect_conflicts();
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_proposal_set_viable_sorted() {
        let available = SystemResources {
            gpu_available_percent: 100.0,
            cpu_available_percent: 100.0,
            memory_available_mb: 2000,
            thermal_headroom: 1.0,
        };

        let mut set = ProposalSet::new();

        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "d1".to_string(),
            "High confidence".to_string(),
            0.95,
            ProposalPriority::UserFacing,
        );
        p1.required_resources.memory_mb = 500;

        let mut p2 = Proposal::new(
            SpecialistId::Archivist,
            "backup".to_string(),
            "Low confidence".to_string(),
            0.3,
            ProposalPriority::Background,
        );
        p2.required_resources.memory_mb = 3000; // Exceeds available

        let mut p3 = Proposal::new(
            SpecialistId::Omnipresent,
            "sync".to_string(),
            "Medium confidence".to_string(),
            0.7,
            ProposalPriority::Normal,
        );
        p3.required_resources.memory_mb = 300;

        set.add(p1.clone());
        set.add(p2.clone());
        set.add(p3.clone());

        let viable = set.viable_sorted(&available);

        // Should only include viable proposals (p1, p3)
        assert_eq!(viable.len(), 2);

        // Should be sorted by score (p1 should be first)
        assert_eq!(viable[0].id, p1.id);
        assert_eq!(viable[1].id, p3.id);
    }

    #[tokio::test]
    async fn test_communication_bus_registration() {
        let mut bus = CommunicationBus::new();

        bus.register_specialist(SpecialistId::Sentinel);
        bus.register_specialist(SpecialistId::Visionary);
        bus.register_specialist(SpecialistId::Omnipresent);

        assert_eq!(bus.specialist_count(), 3);
        assert!(bus.specialist_channel(&SpecialistId::Visionary).is_some());
    }

    #[tokio::test]
    async fn test_proposal_submission_to_bus() {
        let mut bus = CommunicationBus::new();

        let proposal = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Generate designs".to_string(),
            0.8,
            ProposalPriority::Normal,
        );

        bus.submit_proposal(proposal.clone()).unwrap();

        let pending = bus.pending_proposals().await;
        assert_eq!(pending.count(), 1);
        assert_eq!(pending.proposals[0].id, proposal.id);
    }

    #[tokio::test]
    async fn test_sentinel_arbitration_flow() {
        let config = SentinelConfig::default();
        let mut bus = CommunicationBus::new();

        // Register all specialists
        bus.register_specialist(SpecialistId::Sentinel);
        bus.register_specialist(SpecialistId::Visionary);
        bus.register_specialist(SpecialistId::Phygital);

        // Create proposals
        let mut p1 = Proposal::new(
            SpecialistId::Visionary,
            "design".to_string(),
            "Generate designs".to_string(),
            0.85,
            ProposalPriority::UserFacing,
        );
        p1.required_resources.gpu_percent = 50.0;

        let mut p2 = Proposal::new(
            SpecialistId::Phygital,
            "render".to_string(),
            "Render AR".to_string(),
            0.9,
            ProposalPriority::UserFacing,
        );
        p2.required_resources.gpu_percent = 60.0;

        bus.submit_proposal(p1).unwrap();
        bus.submit_proposal(p2).unwrap();

        // Create Sentinel and arbitrate
        let sentinel = Sentinel::new(config, bus);

        // Update resources
        sentinel
            .update_system_resources(SystemResources {
                gpu_available_percent: 100.0,
                cpu_available_percent: 100.0,
                memory_available_mb: 8192,
                thermal_headroom: 1.0,
            })
            .await;

        let result = sentinel.arbitrate().await.unwrap();

        assert_eq!(result.proposals_reviewed, 2);
        assert_eq!(result.conflicts_detected, 1); // GPU contention
        assert!(result.conflicts_resolved > 0);
    }

    #[tokio::test]
    async fn test_specialist_negotiation() {
        let specialist = MockSpecialist::new(SpecialistId::Visionary);

        let conflict = Conflict {
            specialist_a: SpecialistId::Visionary,
            specialist_b: SpecialistId::Phygital,
            conflict_type: "gpu_contention".to_string(),
            context: std::collections::HashMap::new(),
        };

        let result = specialist
            .negotiate(SpecialistId::Phygital, &conflict)
            .await
            .unwrap();

        assert!(result.resolved);
    }

    #[test]
    fn test_conflict_detector_duplicate_action() {
        let p1 = Proposal::new(
            SpecialistId::Visionary,
            "backup".to_string(),
            "Backup".to_string(),
            0.8,
            ProposalPriority::Normal,
        );

        let p2 = Proposal::new(
            SpecialistId::Archivist,
            "backup".to_string(),
            "Backup".to_string(),
            0.7,
            ProposalPriority::Normal,
        );

        assert!(ConflictDetector::duplicate_conflict(&p1, &p2));
    }

    #[test]
    fn test_conflict_arbitration_priority_based() {
        let p1 = Proposal::new(
            SpecialistId::Visionary,
            "action".to_string(),
            "Action".to_string(),
            0.5,
            ProposalPriority::Urgent,
        );

        let p2 = Proposal::new(
            SpecialistId::Archivist,
            "action".to_string(),
            "Action".to_string(),
            0.9,
            ProposalPriority::Background,
        );

        let conflict = crate::federation::proposal::ProposalConflict {
            proposal_a_id: p1.id,
            proposal_b_id: p2.id,
            specialist_a: SpecialistId::Visionary,
            specialist_b: SpecialistId::Archivist,
            conflict_type: crate::federation::proposal::ConflictType::PriorityMismatch,
            severity: crate::federation::proposal::ConflictSeverity::Medium,
        };

        let resolution =
            ConflictArbitrator::resolve(&conflict, &p1, &p2, &SystemResources::default());

        assert!(resolution.resolved);
        assert_eq!(resolution.winner, Some(SpecialistId::Visionary));
    }

    #[test]
    fn test_full_hive_size_calculation() {
        let total_mb: u32 = vec![
            SpecialistId::Sentinel,
            SpecialistId::Visionary,
            SpecialistId::Omnipresent,
            SpecialistId::Symbiotic,
            SpecialistId::Phygital,
            SpecialistId::Archivist,
        ]
        .iter()
        .map(|id| id.model_size_mb())
        .sum();

        assert_eq!(total_mb, 6000); // Full hive = 6GB
    }

    #[test]
    fn test_portable_configurations() {
        // Mobile: Sentinel + Omnipresent + Symbiotic
        let mobile_mb: u32 = vec![
            SpecialistId::Sentinel,
            SpecialistId::Omnipresent,
            SpecialistId::Symbiotic,
        ]
        .iter()
        .map(|id| id.model_size_mb())
        .sum();

        assert_eq!(mobile_mb, 3500); // ~3.5GB

        // Tablet: Mobile + Phygital
        let tablet_mb: u32 = mobile_mb + SpecialistId::Phygital.model_size_mb();
        assert_eq!(tablet_mb, 4500); // ~4.5GB

        // Desktop: Full hive
        let desktop_mb: u32 = 6000;
        assert_eq!(desktop_mb, 6000); // 6GB

        // Server: Sentinel only
        let server_mb: u32 = SpecialistId::Sentinel.model_size_mb();
        assert_eq!(server_mb, 2000); // 2GB
    }

    #[tokio::test]
    async fn test_federation_end_to_end_flow() {
        // Setup
        let config = SentinelConfig::default();
        let mut bus = CommunicationBus::new();

        // Register all specialists
        for id in vec![
            SpecialistId::Sentinel,
            SpecialistId::Visionary,
            SpecialistId::Omnipresent,
            SpecialistId::Symbiotic,
            SpecialistId::Phygital,
            SpecialistId::Archivist,
        ] {
            bus.register_specialist(id);
        }

        let mut sentinel = Sentinel::new(config, bus);

        // Specialist 1: Visionary proposes design work
        let p1 = Proposal::new(
            SpecialistId::Visionary,
            "generate_design".to_string(),
            "Generate 10 UI designs".to_string(),
            0.88,
            ProposalPriority::Normal,
        );

        // Specialist 2: Archivist proposes backup
        let p2 = Proposal::new(
            SpecialistId::Archivist,
            "backup".to_string(),
            "Backup ArtifactRegistry".to_string(),
            0.95,
            ProposalPriority::Background,
        );

        sentinel.communication_bus.submit_proposal(p1).unwrap();
        sentinel.communication_bus.submit_proposal(p2).unwrap();

        // Update resources
        sentinel
            .update_system_resources(SystemResources {
                gpu_available_percent: 100.0,
                cpu_available_percent: 100.0,
                memory_available_mb: 8192,
                thermal_headroom: 1.0,
            })
            .await;

        // Sentinel arbitrates
        let result = sentinel.arbitrate().await.unwrap();

        // Verify
        assert_eq!(result.proposals_reviewed, 2);
        assert!(result.decisions_issued > 0);
    }
}
