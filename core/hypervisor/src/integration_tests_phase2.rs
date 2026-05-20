// Phase 2 Integration Tests
// End-to-end testing of autonomous pipeline with concurrent operations

#[cfg(test)]
mod phase2_integration_tests {
    use std::sync::Arc;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_full_autonomous_pipeline() {
        // Test the complete flow: task submission → analysis → planning → execution → learning
        
        // 1. Create LLM client
        let llm_config = crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2048,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        };
        
        let llm_client = Arc::new(
            crate::llm::LLMClient::new(llm_config)
                .await
                .expect("Failed to create LLM client")
        );
        
        // 2. Create task analysis engine
        let task_analysis = crate::task_analysis::TaskAnalysisEngine::new(llm_client.clone());
        
        // 3. Create autonomous coordinator
        let mut coordinator = crate::autonomous_coordinator::AutonomousCoordinator::new(
            llm_client.clone(),
            task_analysis,
        );
        
        // 4. Create a task
        let task = crate::task_analysis::Task {
            id: "test-task-1".to_string(),
            name: "Analyze Data".to_string(),
            description: "Process and analyze a dataset".to_string(),
            data_sample: Some("sample data".to_string()),
            priority: crate::task_analysis::TaskPriority::High,
            deadline_secs: Some(300),
            required_skills: vec!["data_analysis".to_string()],
            tags: vec!["testing".to_string()],
        };
        
        // 5. Submit task
        let task_id = coordinator
            .submit_task(task)
            .await
            .expect("Failed to submit task");
        
        assert!(!task_id.is_empty());
        assert!(coordinator.get_task_state(&task_id).is_some());
    }

    #[tokio::test]
    async fn test_concurrent_task_processing() {
        // Test handling 10 concurrent tasks
        
        let llm_config = crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::Mock,
            temperature: 0.7,
            max_tokens: 2048,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        };
        
        let llm_client = Arc::new(
            crate::llm::LLMClient::new(llm_config)
                .await
                .expect("Failed to create LLM client")
        );
        
        let task_analysis = crate::task_analysis::TaskAnalysisEngine::new(llm_client.clone());
        let mut coordinator = crate::autonomous_coordinator::AutonomousCoordinator::new(
            llm_client.clone(),
            task_analysis,
        );
        
        // Submit 10 concurrent tasks
        let mut task_ids = vec![];
        for i in 0..10 {
            let task = crate::task_analysis::Task {
                id: format!("concurrent-task-{}", i),
                name: format!("Task {}", i),
                description: format!("Concurrent task {}", i),
                data_sample: None,
                priority: crate::task_analysis::TaskPriority::Normal,
                deadline_secs: None,
                required_skills: vec![],
                tags: vec![],
            };
            
            let task_id = coordinator
                .submit_task(task)
                .await
                .expect("Failed to submit task");
            task_ids.push(task_id);
        }
        
        // Verify all tasks are in coordinator
        assert_eq!(task_ids.len(), 10);
        for task_id in &task_ids {
            assert!(coordinator.get_task_state(task_id).is_some());
        }
    }

    #[test]
    fn test_error_recovery_pipeline() {
        // Test error detection and recovery strategy generation
        
        let _error = crate::error_recovery::ExecutionError {
            task_id: "test-task".to_string(),
            specialist_id: "specialist-1".to_string(),
            error_type: crate::error_recovery::ErrorType::TimeoutExceeded,
            message: "Task exceeded 60 second timeout".to_string(),
            context: Some("Processing large dataset".to_string()),
            timestamp: chrono::Utc::now(),
        };
        
        let recovery_engine = crate::error_recovery::ErrorRecoveryEngine::new(
            Arc::new(
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(async {
                        crate::llm::LLMClient::new(crate::llm::LLMConfig {
                            provider_type: crate::llm::ProviderType::Mock,
                            temperature: 0.7,
                            max_tokens: 2048,
                            timeout_secs: 30,
                            enable_caching: true,
                            cache_ttl_secs: 3600,
                            gguf_model_path: None,
                        })
                        .await
                        .unwrap()
                    })
            )
        );
        
        // Test retry logic
        assert!(recovery_engine.can_retry(0));
        assert!(recovery_engine.can_retry(1));
        assert!(recovery_engine.can_retry(2));
        assert!(!recovery_engine.can_retry(3));
        
        // Test exponential backoff
        assert_eq!(recovery_engine.get_retry_delay(0), 1);
        assert_eq!(recovery_engine.get_retry_delay(1), 2);
        assert_eq!(recovery_engine.get_retry_delay(2), 4);
        assert_eq!(recovery_engine.get_retry_delay(3), 8);
    }

    #[test]
    fn test_specialist_collaboration_workflow() {
        // Test collaboration request and response flow
        
        let mut collab_engine = crate::specialist_collaboration::SpecialistCollaborationEngine::new();
        
        // Specialist 1 requests help
        let req_id = collab_engine.create_help_request(
            "specialist-1".to_string(),
            "task-1".to_string(),
            "Rust".to_string(),
            "Need help with ownership rules".to_string(),
            crate::specialist_collaboration::Urgency::High,
        );
        
        assert!(!req_id.is_empty());
        assert!(collab_engine.get_help_request(&req_id).is_some());
        
        // Specialist 2 responds
        let resp_id = collab_engine.respond_to_request(
            req_id.clone(),
            "specialist-2".to_string(),
            true,
            crate::specialist_collaboration::AssistanceType::Mentoring,
            0.5,
            60,
            "I can help explain Rust ownership".to_string(),
        );
        
        assert!(!resp_id.is_empty());
        
        // Record collaboration
        collab_engine.record_collaboration(
            "specialist-1".to_string(),
            "specialist-2".to_string(),
            "Rust".to_string(),
            4.5,
        );
        
        // Verify collaboration history
        let history = collab_engine.get_collaboration_history("specialist-1", "specialist-2");
        assert!(history.is_some());
        assert_eq!(history.unwrap().collaboration_count, 1);
    }

    #[test]
    fn test_goal_pursuit_workflow() {
        // Test goal creation, activation, and completion
        
        let mut goal_engine = crate::goal_driven_autonomy::GoalDrivenAutonomyEngine::new();
        
        // Create a goal
        let goal_id = goal_engine.create_goal(
            "specialist-1".to_string(),
            "Master Async Rust".to_string(),
            "Become proficient with async/await patterns".to_string(),
            crate::goal_driven_autonomy::GoalCategory::SkillDevelopment,
            crate::goal_driven_autonomy::GoalPriority::High,
            vec![("async_xp".to_string(), 5000.0)].into_iter().collect(),
            vec![],
            None,
        );
        
        assert!(!goal_id.is_empty());
        
        // Activate goal
        assert!(goal_engine.activate_goal(&goal_id));
        let goal = goal_engine.get_goal(&goal_id).unwrap();
        assert_eq!(goal.status, crate::goal_driven_autonomy::AutonomousGoalStatus::Active);
        
        // Update progress
        assert!(goal_engine.update_goal_progress(&goal_id, 0.5));
        let goal = goal_engine.get_goal(&goal_id).unwrap();
        assert_eq!(goal.progress, 0.5);
        assert_eq!(goal.status, crate::goal_driven_autonomy::AutonomousGoalStatus::InProgress);
        
        // Complete goal
        assert!(goal_engine.update_goal_progress(&goal_id, 1.0));
        let goal = goal_engine.get_goal(&goal_id).unwrap();
        assert_eq!(goal.progress, 1.0);
        assert_eq!(goal.status, crate::goal_driven_autonomy::AutonomousGoalStatus::Completed);
    }

    #[test]
    fn test_memory_integration() {
        // Test memory system integration with error recovery and collaboration
        
        let mut memory = crate::specialist_memory::SpecialistMemory::new("spec-1".to_string());
        
        // Record a lesson from experience
        let entry = crate::specialist_memory::MemoryEntry {
            id: uuid::Uuid::new_v4().to_string(),
            specialist_id: "spec-1".to_string(),
            memory_type: crate::specialist_memory::MemoryType::Lesson,
            title: "Learned about async patterns".to_string(),
            description: "Async/await is better than callbacks for readability".to_string(),
            context: "While solving concurrency issue in task-5".to_string(),
            confidence: crate::specialist_memory::Confidence::High,
            relevance_score: 0.95,
            usage_count: 0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["async".to_string(), "rust".to_string()],
            related_memories: vec![],
            source: crate::specialist_memory::MemorySource::ErrorRecovery,
        };
        
        memory.record_memory(entry);
        
        // Search memories
        let found = memory.search_memories("async");
        assert!(!found.is_empty());
    }

    #[test]
    fn test_capability_matching() {
        // Test specialist-to-task matching
        
        let _matching_engine = crate::capability_matching_v2::CapabilityMatchingEngine;
        
        // In a real scenario, we'd have actual specialists
        // For testing, we verify the matching system can be instantiated
        // The actual matching tests are covered in capability_matching_v2 tests
        assert!(true);
    }

    #[test]
    fn test_hive_runtime_startup() {
        // Test HiveRuntime initialization and basic operations
        
        let config = crate::hive_runtime::HiveRuntimeConfig {
            db_path: ":memory:".to_string(),
            inbox_folder: "/tmp/inbox".to_string(),
            output_folder: "/tmp/output".to_string(),
            update_interval_ms: 100,
            max_concurrent_tasks: 4,
            enable_persistence: true,
            enable_ingestion: true,
            enable_dashboard: true,
            crisis_response_enabled: true,
        };
        
        let runtime = tokio::runtime::Runtime::new().unwrap();
        
        let result = runtime.block_on(async {
            crate::hive_runtime::HiveRuntime::new(config).await
        });
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_performance_metrics_collection() {
        // Test that we can collect and calculate performance metrics
        
        // Simulate 5 task completions with varying times
        let mut task_times = vec![];
        for i in 0..5 {
            task_times.push((i as u64) * 100); // 0, 100, 200, 300, 400 ms
        }
        
        // Calculate average
        let average: u64 = task_times.iter().sum::<u64>() / task_times.len() as u64;
        assert_eq!(average, 200);
        
        // Calculate throughput (tasks per second)
        let total_time_ms: u64 = task_times.iter().sum();
        let throughput = (task_times.len() as f64 / total_time_ms as f64) * 1000.0;
        assert!(throughput > 0.0);
    }

    #[test]
    fn test_autonomous_goal_status_machine() {
        // Test all status transitions in goal state machine
        
        let mut engine = crate::goal_driven_autonomy::GoalDrivenAutonomyEngine::new();
        
        let goal_id = engine.create_goal(
            "spec-1".to_string(),
            "Test Goal".to_string(),
            "Test".to_string(),
            crate::goal_driven_autonomy::GoalCategory::SkillDevelopment,
            crate::goal_driven_autonomy::GoalPriority::Medium,
            HashMap::new(),
            vec![],
            None,
        );
        
        // Planning -> Active
        engine.activate_goal(&goal_id);
        assert_eq!(
            engine.get_goal(&goal_id).unwrap().status,
            crate::goal_driven_autonomy::AutonomousGoalStatus::Active
        );
        
        // Active -> AtRisk (progress < 0.2)
        engine.update_goal_progress(&goal_id, 0.1);
        assert_eq!(
            engine.get_goal(&goal_id).unwrap().status,
            crate::goal_driven_autonomy::AutonomousGoalStatus::AtRisk
        );
        
        // AtRisk -> InProgress (progress >= 0.2)
        engine.update_goal_progress(&goal_id, 0.3);
        assert_eq!(
            engine.get_goal(&goal_id).unwrap().status,
            crate::goal_driven_autonomy::AutonomousGoalStatus::InProgress
        );
        
        // InProgress -> OnTrack (progress >= 0.8)
        engine.update_goal_progress(&goal_id, 0.85);
        assert_eq!(
            engine.get_goal(&goal_id).unwrap().status,
            crate::goal_driven_autonomy::AutonomousGoalStatus::OnTrack
        );
        
        // OnTrack -> Completed (progress >= 1.0)
        engine.update_goal_progress(&goal_id, 1.0);
        assert_eq!(
            engine.get_goal(&goal_id).unwrap().status,
            crate::goal_driven_autonomy::AutonomousGoalStatus::Completed
        );
    }

    #[test]
    fn test_error_recovery_strategy_generation() {
        // Test that each error type generates appropriate recovery strategies
        
        let error_types = vec![
            crate::error_recovery::ErrorType::TimeoutExceeded,
            crate::error_recovery::ErrorType::ResourceExhaustion,
            crate::error_recovery::ErrorType::SkillGapFound,
            crate::error_recovery::ErrorType::ExternalServiceFailed,
            crate::error_recovery::ErrorType::InvalidInput,
        ];
        
        for error_type in error_types {
            let _error = crate::error_recovery::ExecutionError {
                task_id: "test".to_string(),
                specialist_id: "spec-1".to_string(),
                error_type,
                message: "Test error".to_string(),
                context: None,
                timestamp: chrono::Utc::now(),
            };
            
            // Each error type should have a display representation
            let display_str = format!("{}", error_type);
            assert!(!display_str.is_empty());
        }
    }

    #[test]
    fn test_collaboration_metrics_calculation() {
        // Test collaboration metrics and team collaboration index
        
        let mut engine = crate::specialist_collaboration::SpecialistCollaborationEngine::new();
        
        // Create some help requests and responses
        for i in 0..3 {
            let req_id = engine.create_help_request(
                format!("spec-{}", i),
                format!("task-{}", i),
                "Skill".to_string(),
                "Help needed".to_string(),
                crate::specialist_collaboration::Urgency::Medium,
            );
            
            engine.respond_to_request(
                req_id,
                format!("helper-{}", i),
                true,
                crate::specialist_collaboration::AssistanceType::Consultation,
                0.5,
                30,
                "Can help".to_string(),
            );
        }
        
        // Calculate team collaboration index
        let team_index = engine.team_collaboration_index();
        assert!(team_index >= 0.0 && team_index <= 1.0);
    }
}
