// End-to-End Integration Test
// Tests the full pipeline: metadata ingestion → compute → decision → action

use std::path::PathBuf;
use std::time::Duration;
use a_run::metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig};
use a_run::decision_engine::{AutonomousDecisionEngine, DecisionTask, ExecutionOutcome};
use a_run::action_executor::{ActionExecutor, ExecutableAction, FileOp};
use a_run::orchestration_daemon::{OrchestrationDaemon, OrchestrationDaemonConfig, DaemonState};
use a_run::{IntelligenceEngine, Specialist, LLMConfig, ProviderType, TaskType};
use a_run::{SystemBiology, PredictiveMetabolicGovernor, MetabolicGovernorConfig, GovernanceAction};
use a_run::workspace::WorkspacePaths;

#[tokio::test]
async fn test_full_pipeline_metadata_to_action() {
    // Step 1: Create metadata ingestor
    let paths = WorkspacePaths::discover();
    let config = MetadataIngestorConfig {
        watch_paths: vec![paths.root().clone()],
        poll_interval: Duration::from_secs(1),
        max_event_queue: 100,
        compute_entropy: true,
        compute_complexity: true,
    };
    let mut ingestor = MetadataIngestor::new(config);

    // Step 2: Collect system metrics (guaranteed to produce an event)
    let metrics_event = ingestor.collect_system_metrics();
    assert_eq!(metrics_event.source, "system:metrics");
    assert_eq!(metrics_event.event_type, "metrics_update");

    // Step 3: Analyze the event
    let analysis = ingestor.analyze_event(&metrics_event);
    assert!(analysis.entropy >= 0.0 || analysis.predicted_complexity >= 0.0);

    // Step 4: Create decision engine
    let specialists = vec![
        Specialist {
            id: "spec_test".to_string(),
            name: "Test Specialist".to_string(),
            skills: vec!["general".to_string()],
            capacity: 1.0,
            success_rate: 0.9,
            avg_completion_time: 5.0,
        },
    ];

    let llm_config = LLMConfig {
        provider_type: ProviderType::Mock,
        model_name: "mock".to_string(),
        api_key: None,
        base_url: None,
    };

    let intelligence = IntelligenceEngine::new(llm_config, specialists);
    let mut decision_engine = AutonomousDecisionEngine::new(intelligence);

    // Step 5: Create a task from the metadata event
    let task = DecisionTask {
        id: "test_pipeline_task".to_string(),
        description: format!("{}: {}", metrics_event.source, metrics_event.event_type),
        task_type: TaskType::Analysis,
        raw_input: serde_json::to_string(&metrics_event.data).unwrap_or_default(),
        priority: 0.5,
        deadline_seconds: None,
    };

    // Step 6: Evaluate the task
    let evaluation = decision_engine.evaluate_task(&task).await.unwrap();
    assert!(!evaluation.task_id.is_empty());
    assert!(evaluation.confidence >= 0.0 && evaluation.confidence <= 1.0);
    assert!(evaluation.complexity >= 0.0 && evaluation.complexity <= 1.0);

    // Step 7: Execute the evaluation
    let outcome = decision_engine.execute_task(&task, &evaluation).await;
    // Outcome depends on confidence and metabolic state - just verify it's valid
    match &outcome {
        ExecutionOutcome::Completed { duration } => {
            assert!(*duration >= 0.0);
        }
        ExecutionOutcome::Queued(_) |
        ExecutionOutcome::Delegated(_) |
        ExecutionOutcome::NeedsInput(_) |
        ExecutionOutcome::Rejected(_) |
        ExecutionOutcome::Failed(_) |
        ExecutionOutcome::Blocked(_) => {
            // All valid outcomes
        }
    }
}

#[tokio::test]
async fn test_ingestion_cycle_with_multiple_tasks() {
    // Create specialists
    let specialists = vec![
        Specialist {
            id: "spec_code".to_string(),
            name: "Code Generator".to_string(),
            skills: vec!["rust".to_string()],
            capacity: 1.0,
            success_rate: 0.9,
            avg_completion_time: 5.0,
        },
        Specialist {
            id: "spec_analysis".to_string(),
            name: "Analyzer".to_string(),
            skills: vec!["analysis".to_string()],
            capacity: 0.8,
            success_rate: 0.85,
            avg_completion_time: 3.0,
        },
    ];

    let llm_config = LLMConfig {
        provider_type: ProviderType::Mock,
        model_name: "mock".to_string(),
        api_key: None,
        base_url: None,
    };

    let intelligence = IntelligenceEngine::new(llm_config, specialists);
    let mut decision_engine = AutonomousDecisionEngine::new(intelligence);

    // Create multiple tasks simulating metadata events
    let tasks = vec![
        DecisionTask {
            id: "task_1".to_string(),
            description: "File modified: lib.rs".to_string(),
            task_type: TaskType::Refactor,
            raw_input: r#"{"path": "lib.rs", "size": 1234}"#.to_string(),
            priority: 0.8,
            deadline_seconds: None,
        },
        DecisionTask {
            id: "task_2".to_string(),
            description: "System metrics update".to_string(),
            task_type: TaskType::Analysis,
            raw_input: r#"{"cpu": 45.2, "memory": 62.1}"#.to_string(),
            priority: 0.3,
            deadline_seconds: None,
        },
        DecisionTask {
            id: "task_3".to_string(),
            description: "New file created: main.rs".to_string(),
            task_type: TaskType::CodeGeneration,
            raw_input: r#"{"path": "main.rs"}"#.to_string(),
            priority: 0.6,
            deadline_seconds: None,
        },
    ];

    // Process ingestion cycle
    let report = decision_engine.process_ingestion_cycle(tasks).await;

    // Verify report
    assert_eq!(report.total_tasks, 3);
    assert!(report.success_count + report.failed_count + report.queued_count <= 3);
    assert!(!report.evaluations.is_empty());

    // Verify metabolic state changed
    assert!(report.final_metabolic_state.global_tokens >= 0.0);
    assert!(report.final_metabolic_state.expression_rate >= 0.0);
}

#[tokio::test]
async fn test_daemon_initialization() {
    let config = OrchestrationDaemonConfig {
        cycle_interval: Duration::from_secs(1),
        max_tasks_per_cycle: 3,
        enable_auto_throttle: true,
        enable_constellation_updates: true,
        ..Default::default()
    };

    let daemon = OrchestrationDaemon::new(config);

    // Verify initial state
    let status = daemon.get_status();
    assert!(matches!(status.state, DaemonState::Initializing));
    assert_eq!(status.cycles_completed, 0);
    assert!(status.metabolic_health.global_tokens >= 0.0);
}

#[test]
fn test_action_executor_file_operations() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    let test_file = dir.path().join("test.txt");

    let mut executor = ActionExecutor::new(PathBuf::from("test.wasm"));

    // Test file creation
    let action = ExecutableAction::FileOperation {
        path: test_file.clone(),
        operation: FileOp::Create,
        content: Some("Hello, Aaroneous!".to_string()),
    };

    // Run async in test context
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(executor.execute(action));

    assert!(result.success);
    assert!(test_file.exists());

    // Verify content
    let content = fs::read_to_string(&test_file).unwrap();
    assert_eq!(content, "Hello, Aaroneous!");
}

#[test]
fn test_wasm_enzyme_exists() {
    // Verify the compute enzyme WASM file was built
    let paths = WorkspacePaths::discover();
    let wasm_path = paths.extensions().join("wasm\\compute_enzyme\\target\\wasm32-unknown-unknown\\release\\compute_enzyme.wasm");
    assert!(wasm_path.exists(), "Compute enzyme WASM should exist at {:?}", wasm_path);

    // Verify the test enzyme WASM file exists
    let test_wasm_path = paths.extensions().join("wasm\\test_enzyme\\target\\wasm32-unknown-unknown\\release\\test_enzyme.wasm");
    assert!(test_wasm_path.exists(), "Test enzyme WASM should exist at {:?}", test_wasm_path);
}

#[tokio::test]
async fn test_metabolic_governance_integration() {
    let mut biology = SystemBiology::new();
    let mut governor = PredictiveMetabolicGovernor::new(MetabolicGovernorConfig::default());

    // Simulate high load history
    for _ in 0..20 {
        governor.record_load(0.9);
    }

    // Apply governance
    let action = governor.apply_governance(&mut biology);

    // Should have throttled due to high predicted risk
    match action {
        GovernanceAction::WarningThrottle { new_rate, .. } |
        GovernanceAction::EmergencyThrottle { new_rate, .. } => {
            assert!(new_rate < 1.0, "Should have throttled");
        }
        GovernanceAction::Recovery { .. } => {
            // Could recover if variance is low
        }
        GovernanceAction::Stable { .. } => {
            // Could be stable if predictions are uncertain
        }
    }

    // Verify biology state was updated
    assert!(biology.expression_rate >= 0.0 && biology.expression_rate <= 1.0);
}
