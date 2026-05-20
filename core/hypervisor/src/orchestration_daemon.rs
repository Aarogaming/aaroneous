// Orchestration Daemon
// Long-running service that ties together metadata ingestion, decision making, and action execution

use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time;
use serde::{Serialize, Deserialize};
use crate::metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig, MetadataEvent, MetadataAnalysis};
use crate::decision_engine::{AutonomousDecisionEngine, DecisionTask, TaskEvaluation, Action, ExecutionOutcome, IngestionReport};
use crate::action_executor::{ActionExecutor, ExecutableAction, ActionResult};
use intelligence::{IntelligenceEngine, Specialist, LLMConfig, ProviderType, TaskType};
use crate::constellation_ui::{ConstellationCanvas, NodeMetrics};
use biology::SystemHealthReport;
use compute::thermodynamics::SystemPhase;

/// Configuration for the orchestration daemon
#[derive(Debug, Clone)]
pub struct OrchestrationDaemonConfig {
    pub ingestor_config: MetadataIngestorConfig,
    pub cycle_interval: Duration,
    pub max_tasks_per_cycle: usize,
    pub wasm_enzyme_path: PathBuf,
    pub enable_auto_throttle: bool,
    pub enable_constellation_updates: bool,
}

impl Default for OrchestrationDaemonConfig {
    fn default() -> Self {
        Self {
            ingestor_config: MetadataIngestorConfig::default(),
            cycle_interval: Duration::from_secs(10),
            max_tasks_per_cycle: 5,
            wasm_enzyme_path: PathBuf::from("extensions/wasm/test_enzyme/target/wasm32-unknown-unknown/release/test_enzyme.wasm"),
            enable_auto_throttle: true,
            enable_constellation_updates: true,
        }
    }
}

/// Daemon state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonState {
    Initializing,
    Running,
    Throttled,
    Error(String),
    ShuttingDown,
}

/// Daemon status report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub state: DaemonState,
    pub uptime_seconds: f64,
    pub cycles_completed: u64,
    pub tasks_processed: u64,
    pub actions_executed: u64,
    pub metabolic_health: SystemHealthReport,
    pub execution_stats: crate::action_executor::ExecutionStats,
    pub last_cycle_duration_ms: f64,
}

/// Orchestration Daemon - the main loop that ties everything together
pub struct OrchestrationDaemon {
    pub config: OrchestrationDaemonConfig,
    pub ingestor: MetadataIngestor,
    pub decision_engine: AutonomousDecisionEngine,
    pub executor: ActionExecutor,
    pub constellation: ConstellationCanvas,
    pub state: DaemonState,
    pub start_time: Instant,
    pub cycles_completed: u64,
    pub tasks_processed: u64,
    pub actions_executed: u64,
    pub last_cycle_duration: Duration,
}

impl OrchestrationDaemon {
    pub fn new(config: OrchestrationDaemonConfig) -> Self {
        // Create test specialists for the intelligence engine
        let specialists = vec![
            Specialist {
                id: "spec_code_gen".to_string(),
                name: "Code Generator".to_string(),
                skills: vec!["rust".to_string(), "python".to_string()],
                capacity: 1.0,
                success_rate: 0.9,
                avg_completion_time: 5.0,
            },
            Specialist {
                id: "spec_bug_fix".to_string(),
                name: "Bug Fixer".to_string(),
                skills: vec!["debugging".to_string()],
                capacity: 0.8,
                success_rate: 0.85,
                avg_completion_time: 8.0,
            },
            Specialist {
                id: "spec_analysis".to_string(),
                name: "Code Analyzer".to_string(),
                skills: vec!["analysis".to_string(), "review".to_string()],
                capacity: 0.9,
                success_rate: 0.95,
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
        let decision_engine = AutonomousDecisionEngine::new(intelligence);
        let executor = ActionExecutor::new(config.wasm_enzyme_path.clone());
        let ingestor_config = config.ingestor_config.clone();
        
        Self {
            config,
            ingestor: MetadataIngestor::new(ingestor_config),
            decision_engine,
            executor,
            constellation: ConstellationCanvas::new(),
            state: DaemonState::Initializing,
            start_time: Instant::now(),
            cycles_completed: 0,
            tasks_processed: 0,
            actions_executed: 0,
            last_cycle_duration: Duration::ZERO,
        }
    }

    /// Run the daemon main loop
    pub async fn run(&mut self) -> Result<(), String> {
        self.state = DaemonState::Running;
        println!("[OrchestrationDaemon] Starting...");
        
        let mut interval = time::interval(self.config.cycle_interval);
        
        loop {
            interval.tick().await;
            
            let cycle_start = Instant::now();
            
            // Run one cycle
            match self.run_cycle().await {
                Ok(_) => {
                    self.cycles_completed += 1;
                    self.last_cycle_duration = cycle_start.elapsed();
                }
                Err(e) => {
                    self.state = DaemonState::Error(e.clone());
                    eprintln!("[OrchestrationDaemon] Cycle error: {}", e);
                }
            }
            
            // Check if we should throttle based on thermodynamic phase
            if self.config.enable_auto_throttle {
                let forecast = self.decision_engine.governor.predict_metabolic_risk();
                
                // Throttle if system is in critical or disordered phase
                if matches!(forecast.phase, SystemPhase::Critical | SystemPhase::Disordered) {
                    self.state = DaemonState::Throttled;
                    println!("[OrchestrationDaemon] Throttled: phase={:?}, free_energy={:.3}", 
                        forecast.phase, forecast.free_energy);
                } else if matches!(self.state, DaemonState::Throttled) 
                    && matches!(forecast.phase, SystemPhase::Ordered | SystemPhase::Mixed) {
                    self.state = DaemonState::Running;
                    println!("[OrchestrationDaemon] Resumed: phase={:?}", forecast.phase);
                }
            }
        }
    }

    /// Run a single cycle
    async fn run_cycle(&mut self) -> Result<(), String> {
        // Step 1: Ingest metadata
        let events = self.ingestor.process_pending_events();
        
        if events.is_empty() {
            return Ok(());
        }
        
        // Step 2: Convert events to tasks
        let tasks: Vec<DecisionTask> = events
            .into_iter()
            .take(self.config.max_tasks_per_cycle)
            .map(|(event, analysis)| {
                self.convert_event_to_task(&event, &analysis)
            })
            .collect();
        
        if tasks.is_empty() {
            return Ok(());
        }
        
        // Step 3: Process tasks through decision engine
        let report = self.decision_engine.process_ingestion_cycle(tasks).await;
        self.tasks_processed += report.total_tasks as u64;
        
        // Step 4: Execute actions based on evaluations
        for (evaluation, outcome) in report.evaluations.iter().zip(report.outcomes.iter()) {
            if let ExecutionOutcome::Completed { .. } = outcome {
                // Convert evaluation to executable action
                let action = ActionExecutor::from_decision(
                    evaluation,
                    &evaluation.recommended_action,
                    None, // Would extract file path from event data
                );
                
                if let Some(exec_action) = action {
                    let result = self.executor.execute(exec_action).await;
                    self.actions_executed += 1;
                    
                    // Update constellation if enabled
                    if self.config.enable_constellation_updates {
                        self.update_constellation_from_action(&result, evaluation);
                    }
                }
            }
        }
        
        // Step 5: Update biology metabolism
        self.decision_engine.biology.update_metabolism();
        
        // Step 6: Apply thermodynamic governance
        let _governance = self.decision_engine.governor.apply_governance(&mut self.decision_engine.biology);
        
        Ok(())
    }

    /// Convert a metadata event to a decision task
    fn convert_event_to_task(&self, event: &MetadataEvent, analysis: &MetadataAnalysis) -> DecisionTask {
        let task_type = match event.event_type.as_str() {
            "file_modified" => TaskType::Refactor,
            "file_created" => TaskType::CodeGeneration,
            "metrics_update" => TaskType::Analysis,
            _ => TaskType::Custom(event.event_type.clone()),
        };
        
        let priority = if analysis.predicted_complexity > 0.7 {
            0.8
        } else if analysis.entropy > 3.0 {
            0.6
        } else {
            0.4
        };
        
        DecisionTask {
            id: format!("task_{}", event.timestamp),
            description: format!("{}: {}", event.source, event.event_type),
            task_type,
            raw_input: serde_json::to_string(&event.data).unwrap_or_default(),
            priority,
            deadline_seconds: None,
        }
    }

    /// Update constellation visualization from action result
    fn update_constellation_from_action(&mut self, _result: &ActionResult, evaluation: &TaskEvaluation) {
        // Create or update a node for this task
        let node_id = format!("node_{}", evaluation.task_id);
        
        // Check if node exists
        if !self.constellation.nodes.iter().any(|n| n.id == node_id) {
            // Would create a new ConstellationNode here
            // For now, just update metrics
        }
        
        let metrics = NodeMetrics {
            entropy: evaluation.entropy,
            confidence: evaluation.confidence,
            metabolic_risk: evaluation.metabolic_risk,
            centrality: 0.5,
            mdp_value: evaluation.routing.confidence,
        };
        
        // Update in constellation
        if let Some(index) = self.constellation.nodes.iter().position(|n| n.id == node_id) {
            self.constellation.update_node_metrics(index, metrics);
        }
    }

    /// Get current daemon status
    pub fn get_status(&self) -> DaemonStatus {
        DaemonStatus {
            state: self.state.clone(),
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
            cycles_completed: self.cycles_completed,
            tasks_processed: self.tasks_processed,
            actions_executed: self.actions_executed,
            metabolic_health: self.decision_engine.biology.get_health_report(),
            execution_stats: self.executor.get_stats(),
            last_cycle_duration_ms: self.last_cycle_duration.as_secs_f64() * 1000.0,
        }
    }

    /// Gracefully shutdown the daemon
    pub fn shutdown(&mut self) {
        self.state = DaemonState::ShuttingDown;
        println!("[OrchestrationDaemon] Shutting down...");
        println!("[OrchestrationDaemon] Final stats: {} cycles, {} tasks, {} actions",
            self.cycles_completed, self.tasks_processed, self.actions_executed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_creation() {
        let config = OrchestrationDaemonConfig::default();
        let daemon = OrchestrationDaemon::new(config);
        
        assert!(matches!(daemon.state, DaemonState::Initializing));
        assert_eq!(daemon.cycles_completed, 0);
    }

    #[test]
    fn test_daemon_status() {
        let config = OrchestrationDaemonConfig::default();
        let daemon = OrchestrationDaemon::new(config);
        let status = daemon.get_status();
        
        assert!(matches!(status.state, DaemonState::Initializing));
        assert_eq!(status.cycles_completed, 0);
    }

    #[test]
    fn test_convert_event_to_task() {
        let config = OrchestrationDaemonConfig::default();
        let daemon = OrchestrationDaemon::new(config);
        
        let event = MetadataEvent {
            source: "test".to_string(),
            event_type: "file_modified".to_string(),
            timestamp: 0.0,
            data: serde_json::json!({"path": "test.rs"}),
            raw_bytes: None,
        };
        
        let analysis = crate::metadata_ingestor::MetadataAnalysis::default();
        let task = daemon.convert_event_to_task(&event, &analysis);
        
        assert!(task.id.starts_with("task_"));
        assert!(matches!(task.task_type, TaskType::Refactor));
    }
}
