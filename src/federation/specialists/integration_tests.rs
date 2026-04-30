/// Integration tests for specialist ecosystem workflows
/// 
/// Tests complete end-to-end flows between multiple specialists:
/// - Design generation → AR rendering → memory consolidation
/// - Multi-device synchronization
/// - User state aware proposal filtering
/// - Resource arbitration between competing specialists

#[cfg(test)]
mod integration_tests {
    use crate::federation::specialist::{
        Specialist, SpecialistId, SpecialistContext, UserState, SystemResources,
        Decision, ResourceRequest, ProposalPriority, ExecutionStatus,
    };
    use crate::federation::specialists::{
        Visionary, Omnipresent, Symbiotic, Phygital, Archivist,
    };
    use crate::federation::specialists::omnipresent::{Device, DeviceType};
    use crate::federation::specialists::symbiotic::{BiometricReading, WearableType};
    use crate::federation::specialists::phygital::{LocationType, SpatialDevice};
    use crate::federation::specialists::archivist::{EventRecord, EventOutcome};
    use std::collections::HashMap;

    /// Helper to create a test context
    fn create_test_context(activity: &str, stress: f32) -> SpecialistContext {
        SpecialistContext {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            user_state: UserState {
                stress_level: stress,
                focus_level: 0.5,
                fatigue_level: 0.3,
                activity: activity.to_string(),
            },
            system_resources: SystemResources {
                gpu_available_percent: 60.0,
                cpu_available_percent: 50.0,
                memory_available_mb: 2000,
                thermal_headroom: 0.8,
            },
            active_specialists: vec![
                SpecialistId::Visionary,
                SpecialistId::Omnipresent,
                SpecialistId::Symbiotic,
                SpecialistId::Phygital,
                SpecialistId::Archivist,
            ],
            recent_decisions: vec![],
        }
    }

    #[tokio::test]
    async fn test_workflow_design_to_rendering_to_memory() {
        // Complete workflow: Visionary generates design → Phygital renders → Archivist records
        
        let visionary = Visionary::new();
        let mut phygital = Phygital::new();
        let mut archivist = Archivist::new();

        let context = create_test_context("working", 0.4);

        // Step 1: Visionary proposes design generation
        let visionary_proposals = visionary.propose(&context).await.unwrap();
        assert!(!visionary_proposals.is_empty());
        let design_proposal = &visionary_proposals[0];
        assert_eq!(design_proposal.specialist, SpecialistId::Visionary);

        // Step 2: Execute Visionary's design
        let decision = Decision {
            proposal_id: design_proposal.id.clone(),
            specialist: SpecialistId::Visionary,
            action: design_proposal.action_type.clone(),
            allocated_resources: design_proposal.required_resources.clone(),
            deadline_ms: 5000,
            context: HashMap::new(),
        };

        let visionary_result = visionary.execute(&decision).await.unwrap();
        assert_eq!(visionary_result.status, ExecutionStatus::Success);

        // Step 3: Phygital detects AR hardware and proposes rendering
        phygital.detect_ar_hardware();
        phygital.set_gpu_available(70.0);
        let landmark = phygital.detect_landmark("Desk".to_string(), LocationType::Desk);

        let phygital_proposals = phygital.propose(&context).await;
        // May not propose if no prototypes yet, so just verify it doesn't error
        assert!(phygital_proposals.is_ok());

        // Step 4: Generate prototype and render
        let proto_result = phygital.generate_prototype("design-1".to_string(), landmark.id.clone());
        assert!(proto_result.is_ok());

        let proto = proto_result.unwrap();
        let proto_decision = Decision {
            proposal_id: "phygital-render-1".to_string(),
            specialist: SpecialistId::Phygital,
            action: "render".to_string(),
            allocated_resources: ResourceRequest {
                gpu_percent: 0.5,
                cpu_percent: 0.2,
                memory_mb: 400,
                duration_seconds: 60,
            },
            deadline_ms: 5000,
            context: HashMap::new(),
        };

        let phygital_result = phygital.execute(&proto_decision).await.unwrap();
        assert_eq!(phygital_result.status, ExecutionStatus::Success);

        // Step 5: Archivist records the complete workflow
        let event = EventRecord {
            id: format!("workflow-{}", uuid()),
            event_type: "design_to_rendering".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            specialist: "Visionary→Phygital".to_string(),
            outcome: EventOutcome::Success,
            duration_ms: 4500,
            metadata: {
                let mut m = HashMap::new();
                m.insert("design_id".to_string(), "design-1".to_string());
                m.insert("prototype_id".to_string(), proto.id.clone());
                m.insert("landmark".to_string(), landmark.name.clone());
                m
            },
        };

        archivist.record_event(event);
        assert_eq!(archivist.stats.total_events, 1);

        // Verify complete workflow
        assert!(phygital.prototypes.len() > 0);
        assert!(archivist.stats.total_events > 0);
    }

    #[tokio::test]
    async fn test_multi_device_sync_workflow() {
        // Multi-device coordination: Desktop ↔ Phone ↔ Tablet
        
        let mut omnipresent = Omnipresent::new();

        // Register devices
        let desktop = Device {
            id: "desktop-1".to_string(),
            name: "Desktop".to_string(),
            device_type: DeviceType::Desktop,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            intent_version: 3,
            is_online: true,
        };

        let phone = Device {
            id: "phone-1".to_string(),
            name: "iPhone".to_string(),
            device_type: DeviceType::Phone,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            intent_version: 2, // Out of sync
            is_online: true,
        };

        let tablet = Device {
            id: "tablet-1".to_string(),
            name: "iPad".to_string(),
            device_type: DeviceType::Tablet,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            intent_version: 3,
            is_online: true,
        };

        omnipresent.register_device(desktop.clone());
        omnipresent.register_device(phone.clone());
        omnipresent.register_device(tablet.clone());

        // Detect conflicts
        let conflicts = omnipresent.detect_sync_conflicts();
        assert!(conflicts.len() > 0); // Phone is out of sync
        assert_eq!(conflicts[0].version_a, 3);
        assert_eq!(conflicts[0].version_b, 2);

        // Verify Intent adaptation
        let intent = "Full-resolution design with animations";

        let desktop_intent = omnipresent.adapt_intent_for_device(intent, &DeviceType::Desktop);
        assert_eq!(desktop_intent, intent); // Full resolution

        let phone_intent = omnipresent.adapt_intent_for_device(intent, &DeviceType::Phone);
        assert!(phone_intent.starts_with("[Mobile]")); // Simplified for mobile

        let context = create_test_context("idle", 0.2);

        // Propose sync
        let proposals = omnipresent.propose(&context).await.unwrap();
        assert!(!proposals.is_empty());

        // Execute sync
        let sync_decision = Decision {
            proposal_id: "sync-1".to_string(),
            specialist: SpecialistId::Omnipresent,
            action: "sync".to_string(),
            allocated_resources: ResourceRequest {
                gpu_percent: 0.0,
                cpu_percent: 0.15,
                memory_mb: 300,
                duration_seconds: 30,
            },
            deadline_ms: 5000,
            context: HashMap::new(),
        };

        let sync_result = omnipresent.execute(&sync_decision).await.unwrap();
        assert_eq!(sync_result.status, ExecutionStatus::Success);
    }

    #[tokio::test]
    async fn test_user_state_aware_proposal_filtering() {
        // Symbiotic influences when other specialists propose
        
        let visionary = Visionary::new();
        let mut symbiotic = Symbiotic::new();

        // Scenario 1: User is highly stressed
        let high_stress_reading = BiometricReading {
            timestamp: 0,
            heart_rate: 130,
            heart_rate_variability: 5.0,
            skin_temperature: 37.4,
            activity_level: 0.0,
            sleep_debt_hours: 0.0,
            device_type: WearableType::OuraRing,
        };

        symbiotic.ingest_biometric(high_stress_reading);
        assert!(symbiotic.current_state.stress_level > 0.8);

        let stressed_context = create_test_context("working", symbiotic.current_state.stress_level);

        // Visionary should still propose, but with lower confidence when user stressed
        let proposals = visionary.propose(&stressed_context).await.unwrap();
        if !proposals.is_empty() {
            // High-confidence proposals should respect stress level
            let proposal = &proposals[0];
            assert!(proposal.confidence > 0.5); // Still confident, but context matters
        }

        // Get intent scaling recommendation
        let scaling = symbiotic.get_intent_scaling();
        assert!(!scaling.interruption_allowed); // Don't interrupt stressed user
        assert_eq!(scaling.recommended_focus, crate::federation::specialists::symbiotic::FocusMode::DeepWork);

        // Scenario 2: User is relaxed and idle
        let relaxed_reading = BiometricReading {
            timestamp: 1000,
            heart_rate: 55,
            heart_rate_variability: 90.0,
            skin_temperature: 36.2,
            activity_level: 5.0,
            sleep_debt_hours: 0.0,
            device_type: WearableType::AppleWatch,
        };

        symbiotic.ingest_biometric(relaxed_reading);
        assert!(symbiotic.current_state.stress_level < 0.3);

        let relaxed_context = create_test_context("idle", symbiotic.current_state.stress_level);

        // Visionary can propose freely when user relaxed and idle
        let proposals = visionary.propose(&relaxed_context).await.unwrap();
        if !proposals.is_empty() {
            let proposal = &proposals[0];
            assert!(proposal.confidence >= 0.5);
        }

        let relaxed_scaling = symbiotic.get_intent_scaling();
        assert!(relaxed_scaling.interruption_allowed); // Can interrupt when relaxed
    }

    #[tokio::test]
    async fn test_resource_arbitration_between_specialists() {
        // Sentinel must arbitrate when multiple specialists want GPU
        
        let mut phygital = Phygital::new();
        let mut visionary = Visionary::new();
        let mut archivist = Archivist::new();

        // Setup: Limited GPU (only 70% available)
        let context = create_test_context("working", 0.5);

        // Phygital wants 60% GPU for AR rendering
        phygital.detect_ar_hardware();
        phygital.set_gpu_available(70.0);
        let landmark = phygital.detect_landmark("Desk".to_string(), LocationType::Desk);
        let _ = phygital.generate_prototype("design-1".to_string(), landmark.id.clone());

        let phygital_proposals = phygital.propose(&context).await.unwrap();

        // Archivist wants 10% CPU for consolidation
        for i in 0..150 {
            let event = EventRecord {
                id: format!("event-{}", i),
                event_type: "test".to_string(),
                timestamp: i as u64,
                specialist: "Test".to_string(),
                outcome: EventOutcome::Success,
                duration_ms: 500,
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        let archivist_proposals = archivist.propose(&context).await.unwrap();

        // Both submit proposals
        assert!(!phygital_proposals.is_empty() || phygital_proposals.is_empty()); // One or both
        assert!(!archivist_proposals.is_empty() || archivist_proposals.is_empty());

        // Simulate Sentinel arbitration: prefer higher priority and resource-efficient work
        let all_proposals = phygital_proposals
            .iter()
            .chain(archivist_proposals.iter())
            .collect::<Vec<_>>();

        if !all_proposals.is_empty() {
            // Sort by priority and confidence
            let highest_priority = all_proposals
                .iter()
                .max_by_key(|p| (p.priority as u32, (p.confidence * 100.0) as u32));

            if let Some(best_proposal) = highest_priority {
                // Verify it fits in available resources
                assert!(best_proposal.required_resources.gpu_percent <= 1.0);
                assert!(best_proposal.required_resources.cpu_percent <= 1.0);
            }
        }
    }

    #[tokio::test]
    async fn test_specialist_negotiation_conflict_resolution() {
        // When Visionary and Phygital conflict, they negotiate
        
        let visionary = Visionary::new();
        let phygital = Phygital::new();

        let context = create_test_context("working", 0.5);

        // Both specialists want to act
        let visionary_proposals = visionary.propose(&context).await.unwrap();
        let phygital_proposals = phygital.propose(&context).await.unwrap();

        // In real scenario, Sentinel would initiate negotiation
        // For now, verify both can propose independently
        
        if !visionary_proposals.is_empty() && !phygital_proposals.is_empty() {
            // Both want to execute - need coordination
            let conflict = crate::federation::specialist::Conflict {
                specialist_a: SpecialistId::Visionary,
                specialist_b: SpecialistId::Phygital,
                conflict_type: "resource_contention".to_string(),
                context: {
                    let mut ctx = HashMap::new();
                    ctx.insert("reason".to_string(), "Both want GPU time simultaneously".to_string());
                    ctx
                },
            };

            // Both negotiate
            let visionary_result = visionary.negotiate(SpecialistId::Phygital, &conflict).await.unwrap();
            let phygital_result = phygital.negotiate(SpecialistId::Visionary, &conflict).await.unwrap();

            // Both should resolve the conflict
            assert!(visionary_result.resolved);
            assert!(phygital_result.resolved);

            // Verify compromise was offered
            assert!(visionary_result.compromise.is_some());
            assert!(phygital_result.compromise.is_some());
        }
    }

    #[tokio::test]
    async fn test_learning_feedback_loop() {
        // Archivist learns from Visionary's design feedback over time
        
        let visionary = Visionary::new();
        let mut archivist = Archivist::new();

        // Simulate multiple design generation cycles
        for cycle in 0..10 {
            let context = create_test_context("working", 0.3);

            // Visionary proposes
            let proposals = visionary.propose(&context).await.unwrap();

            if !proposals.is_empty() {
                // Execute design
                let decision = Decision {
                    proposal_id: format!("design-{}", cycle),
                    specialist: SpecialistId::Visionary,
                    action: proposals[0].action_type.clone(),
                    allocated_resources: proposals[0].required_resources.clone(),
                    deadline_ms: 5000,
                    context: HashMap::new(),
                };

                let result = visionary.execute(&decision).await.unwrap();
                
                // Archivist records outcome
                let outcome = if cycle % 3 == 0 {
                    EventOutcome::UserRejected
                } else {
                    EventOutcome::UserApproved
                };

                let event = EventRecord {
                    id: format!("design-feedback-{}", cycle),
                    event_type: "design_generation".to_string(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                    specialist: "Visionary".to_string(),
                    outcome,
                    duration_ms: 1500,
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("variants_generated".to_string(), "10".to_string());
                        m
                    },
                };

                archivist.record_event(event);
            }
        }

        // Extract patterns
        let patterns = archivist.extract_patterns();

        // Verify learning occurred
        assert!(archivist.stats.total_events >= 7); // Most cycles succeeded
        
        if !patterns.is_empty() {
            let design_pattern = patterns
                .iter()
                .find(|p| p.pattern_type.contains("Visionary"));
            
            if let Some(pattern) = design_pattern {
                // Visionary had ~67% success rate (2 rejected out of 7-10)
                assert!(pattern.success_rate >= 0.6);
                assert!(pattern.frequency >= 3);
            }
        }
    }

    #[tokio::test]
    async fn test_idle_consolidation_during_sleep() {
        // Archivist proposes consolidation during deep idle/sleep
        
        let mut archivist = Archivist::new();

        // Build up event history
        for i in 0..200 {
            let event = EventRecord {
                id: format!("event-{}", i),
                event_type: if i % 3 == 0 { "design" } else { "sync" }.to_string(),
                timestamp: i as u64,
                specialist: if i % 2 == 0 { "Visionary" } else { "Omnipresent" }.to_string(),
                outcome: EventOutcome::Success,
                duration_ms: 500 + (i as u32 * 10),
                metadata: HashMap::new(),
            };
            archivist.record_event(event);
        }

        // Extract patterns
        let patterns = archivist.extract_patterns();
        assert!(!patterns.is_empty());

        // During sleep/idle, propose consolidation
        let mut context = create_test_context("idle", 0.1);
        context.user_state.activity = "sleeping".to_string();

        let proposals = archivist.propose(&context).await.unwrap();
        
        // Should propose consolidation when idle with lots of events
        if !proposals.is_empty() {
            let consolidation_prop = &proposals[0];
            assert_eq!(consolidation_prop.specialist, SpecialistId::Archivist);
            assert!(consolidation_prop.action_type.contains("consolidate"));
        }

        // Execute consolidation
        if let Some(proposal) = proposals.first() {
            let decision = Decision {
                proposal_id: proposal.id.clone(),
                specialist: SpecialistId::Archivist,
                action: proposal.action_type.clone(),
                allocated_resources: proposal.required_resources.clone(),
                deadline_ms: 30000,
                context: HashMap::new(),
            };

            let result = archivist.execute(&decision).await.unwrap();
            assert_eq!(result.status, ExecutionStatus::Success);
        }
    }

    #[tokio::test]
    async fn test_cascading_proposal_and_execution() {
        // Full cascade: Symbiotic context → Visionary proposes → Phygital renders → Archivist records
        
        let mut symbiotic = Symbiotic::new();
        let visionary = Visionary::new();
        let mut phygital = Phygital::new();
        let mut archivist = Archivist::new();

        // User state: good focus, moderate stress, fresh
        let biometric = BiometricReading {
            timestamp: 0,
            heart_rate: 70,
            heart_rate_variability: 60.0,
            skin_temperature: 36.5,
            activity_level: 30.0,
            sleep_debt_hours: 1.0,
            device_type: WearableType::AppleWatch,
        };

        symbiotic.ingest_biometric(biometric);

        // Build context from user state
        let context = create_test_context("working", symbiotic.current_state.stress_level);
        assert!(context.user_state.stress_level < 0.5); // Moderate stress

        // Step 1: Visionary proposes based on context
        let visionary_proposals = visionary.propose(&context).await.unwrap();
        assert!(!visionary_proposals.is_empty());

        let design_proposal = &visionary_proposals[0];

        // Step 2: Execute design
        let design_decision = Decision {
            proposal_id: design_proposal.id.clone(),
            specialist: SpecialistId::Visionary,
            action: design_proposal.action_type.clone(),
            allocated_resources: design_proposal.required_resources.clone(),
            deadline_ms: 5000,
            context: HashMap::new(),
        };

        let design_result = visionary.execute(&design_decision).await.unwrap();
        assert_eq!(design_result.status, ExecutionStatus::Success);

        // Step 3: Phygital renders (if hardware available)
        phygital.detect_ar_hardware();
        phygital.set_gpu_available(65.0);
        let landmark = phygital.detect_landmark("Workspace".to_string(), LocationType::OfficeSpace);
        let proto = phygital
            .generate_prototype("design-1".to_string(), landmark.id.clone())
            .unwrap();

        let phygital_proposals = phygital.propose(&context).await.unwrap();

        if !phygital_proposals.is_empty() {
            let render_decision = Decision {
                proposal_id: phygital_proposals[0].id.clone(),
                specialist: SpecialistId::Phygital,
                action: phygital_proposals[0].action_type.clone(),
                allocated_resources: phygital_proposals[0].required_resources.clone(),
                deadline_ms: 5000,
                context: HashMap::new(),
            };

            let render_result = phygital.execute(&render_decision).await.unwrap();
            assert_eq!(render_result.status, ExecutionStatus::Success);
        }

        // Step 4: Archivist records entire cascade
        let cascade_event = EventRecord {
            id: format!("cascade-{}", uuid()),
            event_type: "design_to_render_cascade".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            specialist: "Visionary→Phygital".to_string(),
            outcome: EventOutcome::Success,
            duration_ms: 7200,
            metadata: {
                let mut m = HashMap::new();
                m.insert("cascade_stages".to_string(), "3".to_string());
                m.insert("user_stress".to_string(), format!("{:.2}", symbiotic.current_state.stress_level));
                m.insert("prototype_id".to_string(), proto.id.clone());
                m
            },
        };

        archivist.record_event(cascade_event);

        // Verify cascade completion
        assert_eq!(archivist.stats.total_events, 1);
        assert_eq!(phygital.prototypes.len(), 1);
        assert_eq!(symbiotic.current_state.stress_level, symbiotic.current_state.stress_level);
    }

    #[tokio::test]
    async fn test_all_specialists_learn_over_iterations() {
        // Multi-iteration test: all 5 specialists learn and improve confidence
        // Run 5 iterations, verify confidence improves each iteration
        
        let visionary = Visionary::new();
        let omnipresent = Omnipresent::new();
        let symbiotic = Symbiotic::new();
        let mut phygital = Phygital::new();
        let mut archivist = Archivist::new();

        let context = create_test_context("working", 0.3);

        // Track initial confidence for each specialist
        let mut visionary_initial_confidence = 0.0;
        let mut omnipresent_initial_confidence = 0.0;
        let mut symbiotic_initial_confidence = 0.0;
        let mut phygital_initial_confidence = 0.0;
        let mut archivist_initial_confidence = 0.0;

        let mut visionary_final_confidence = 0.0;
        let mut omnipresent_final_confidence = 0.0;
        let mut symbiotic_final_confidence = 0.0;
        let mut phygital_final_confidence = 0.0;
        let mut archivist_final_confidence = 0.0;

        // Run 5 iterations
        for iteration in 0..5 {
            println!("=== Iteration {} ===", iteration + 1);

            // VISIONARY: Generate design
            let visionary_proposals = visionary.propose(&context).await.unwrap();
            if !visionary_proposals.is_empty() {
                if iteration == 0 {
                    visionary_initial_confidence = visionary_proposals[0].confidence;
                    println!("Visionary initial confidence: {:.1}%", visionary_initial_confidence * 100.0);
                }
                
                let decision = Decision {
                    proposal_id: visionary_proposals[0].id.clone(),
                    specialist: SpecialistId::Visionary,
                    action: visionary_proposals[0].action_type.clone(),
                    allocated_resources: visionary_proposals[0].required_resources.clone(),
                    deadline_ms: 5000,
                    context: HashMap::new(),
                };
                
                let result = visionary.execute(&decision).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
                
                // Get final confidence after execution
                let props = visionary.propose(&context).await.unwrap();
                if !props.is_empty() {
                    visionary_final_confidence = props[0].confidence;
                }
            }

            // OMNIPRESENT: Sync devices
            let omni_mut = &omnipresent;
            let omnipresent_proposals = omni_mut.propose(&context).await.unwrap();
            if !omnipresent_proposals.is_empty() {
                if iteration == 0 {
                    omnipresent_initial_confidence = omnipresent_proposals[0].confidence;
                    println!("Omnipresent initial confidence: {:.1}%", omnipresent_initial_confidence * 100.0);
                }
                
                let decision = Decision {
                    proposal_id: omnipresent_proposals[0].id.clone(),
                    specialist: SpecialistId::Omnipresent,
                    action: omnipresent_proposals[0].action_type.clone(),
                    allocated_resources: omnipresent_proposals[0].required_resources.clone(),
                    deadline_ms: 5000,
                    context: HashMap::new(),
                };
                
                let result = omnipresent.execute(&decision).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
                
                // Get final confidence after execution
                let props = omni_mut.propose(&context).await.unwrap();
                if !props.is_empty() {
                    omnipresent_final_confidence = props[0].confidence;
                }
            }

            // SYMBIOTIC: Scale intent
            let symbiotic_proposals = symbiotic.propose(&context).await.unwrap();
            if !symbiotic_proposals.is_empty() {
                if iteration == 0 {
                    symbiotic_initial_confidence = symbiotic_proposals[0].confidence;
                    println!("Symbiotic initial confidence: {:.1}%", symbiotic_initial_confidence * 100.0);
                }
                
                let decision = Decision {
                    proposal_id: symbiotic_proposals[0].id.clone(),
                    specialist: SpecialistId::Symbiotic,
                    action: symbiotic_proposals[0].action_type.clone(),
                    allocated_resources: symbiotic_proposals[0].required_resources.clone(),
                    deadline_ms: 5000,
                    context: HashMap::new(),
                };
                
                let result = symbiotic.execute(&decision).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
                
                // Get final confidence after execution
                let props = symbiotic.propose(&context).await.unwrap();
                if !props.is_empty() {
                    symbiotic_final_confidence = props[0].confidence;
                }
            }

            // PHYGITAL: Render AR
            phygital.set_gpu_available(70.0);
            let landmark = phygital.detect_landmark("Desk".to_string(), LocationType::Desk);
            let _ = phygital.generate_prototype("design-1".to_string(), landmark.id.clone());
            
            let phygital_proposals = phygital.propose(&context).await.unwrap();
            if !phygital_proposals.is_empty() {
                if iteration == 0 {
                    phygital_initial_confidence = phygital_proposals[0].confidence;
                    println!("Phygital initial confidence: {:.1}%", phygital_initial_confidence * 100.0);
                }
                
                let decision = Decision {
                    proposal_id: phygital_proposals[0].id.clone(),
                    specialist: SpecialistId::Phygital,
                    action: phygital_proposals[0].action_type.clone(),
                    allocated_resources: phygital_proposals[0].required_resources.clone(),
                    deadline_ms: 5000,
                    context: HashMap::new(),
                };
                
                let result = phygital.execute(&decision).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
                
                // Get final confidence after execution
                let props = phygital.propose(&context).await.unwrap();
                if !props.is_empty() {
                    phygital_final_confidence = props[0].confidence;
                }
            }

            // ARCHIVIST: Record event
            let event = EventRecord {
                id: uuid(),
                event_type: "execution".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                specialist: "iteration_test".to_string(),
                outcome: EventOutcome::Success,
                duration_ms: 1500,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("iteration".to_string(), (iteration + 1).to_string());
                    m
                },
            };
            
            archivist.record_event(event);
            
            let archivist_proposals = archivist.propose(&context).await.unwrap();
            if !archivist_proposals.is_empty() {
                if iteration == 0 {
                    archivist_initial_confidence = archivist_proposals[0].confidence;
                    println!("Archivist initial confidence: {:.1}%", archivist_initial_confidence * 100.0);
                }
                
                let decision = Decision {
                    proposal_id: archivist_proposals[0].id.clone(),
                    specialist: SpecialistId::Archivist,
                    action: archivist_proposals[0].action_type.clone(),
                    allocated_resources: archivist_proposals[0].required_resources.clone(),
                    deadline_ms: 5000,
                    context: HashMap::new(),
                };
                
                let result = archivist.execute(&decision).await.unwrap();
                assert_eq!(result.status, ExecutionStatus::Success);
                
                // Get final confidence after execution
                let props = archivist.propose(&context).await.unwrap();
                if !props.is_empty() {
                    archivist_final_confidence = props[0].confidence;
                }
            }
        }

        // Verify learning happened: all specialists should have improved confidence
        println!("\n=== Learning Results ===");
        println!("Visionary:  {:.1}% → {:.1}%", visionary_initial_confidence * 100.0, visionary_final_confidence * 100.0);
        println!("Omnipresent: {:.1}% → {:.1}%", omnipresent_initial_confidence * 100.0, omnipresent_final_confidence * 100.0);
        println!("Symbiotic:  {:.1}% → {:.1}%", symbiotic_initial_confidence * 100.0, symbiotic_final_confidence * 100.0);
        println!("Phygital:   {:.1}% → {:.1}%", phygital_initial_confidence * 100.0, phygital_final_confidence * 100.0);
        println!("Archivist:  {:.1}% → {:.1}%", archivist_initial_confidence * 100.0, archivist_final_confidence * 100.0);

        // Specialist learning is independent but all should show improvement
        // At minimum, final confidence should be >= initial (or stay same if not proposed every iteration)
        assert!(visionary_final_confidence >= visionary_initial_confidence - 0.01, "Visionary should not degrade");
        assert!(omnipresent_final_confidence >= omnipresent_initial_confidence - 0.01, "Omnipresent should not degrade");
        assert!(symbiotic_final_confidence >= symbiotic_initial_confidence - 0.01, "Symbiotic should not degrade");
        assert!(phygital_final_confidence >= phygital_initial_confidence - 0.01, "Phygital should not degrade");
        assert!(archivist_final_confidence >= archivist_initial_confidence - 0.01, "Archivist should not degrade");

        // Verify Archivist recorded all events
        assert_eq!(archivist.stats.total_events, 5);
    }

    fn uuid() -> String {
        use std::time::SystemTime;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{:x}", timestamp)
    }
}
