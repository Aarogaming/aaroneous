// Unified Orchestration Daemon
// Uses the Unified Learning Loop for all system operations
// Replaces the old decision engine + governor architecture

use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::time;
use serde::{Serialize, Deserialize};
use crate::metadata_ingestor::{MetadataIngestor, MetadataIngestorConfig, MetadataEvent, MetadataAnalysis};
use crate::action_executor::{ActionExecutor, ExecutableAction, ActionResult};
use crate::constellation_ui::{ConstellationCanvas, NodeMetrics};
use crate::tensor_router::TaskEmbedding;
use crate::unified_learning::{UnifiedLearningLoop, UnifiedLearningConfig, UnifiedCycleResult, SystemHealthSummary};
use crate::spectral_layout::spectral_layout_2d;
use crate::hox_registry::HoxRegistry;
use crate::enzyme_runner::EnzymeRunner;
use crate::autonomic_loop::AutonomicNervousSystem;
use crate::splicing_engine::WasmSplicingEngine;
use crate::chaos_monkey::ChaosMonkey;
use crate::nlm_sentinel::NlmSentinel;
use biology::SystemHealthReport;
use intelligence::{IntelligenceEngine, Specialist, LLMConfig, ProviderType, TaskType};

/// Configuration for the unified orchestration daemon
#[derive(Debug, Clone)]
pub struct UnifiedOrchestrationConfig {
    pub ingestor_config: MetadataIngestorConfig,
    pub cycle_interval: Duration,
    pub max_tasks_per_cycle: usize,
    pub wasm_enzyme_path: PathBuf,
    pub enable_auto_throttle: bool,
    pub enable_constellation_updates: bool,
    pub learning_config: UnifiedLearningConfig,
    pub n_specialists: usize,
    pub hox_db_path: String,
}

impl Default for UnifiedOrchestrationConfig {
    fn default() -> Self {
        Self {
            ingestor_config: MetadataIngestorConfig::default(),
            cycle_interval: Duration::from_secs(10),
            max_tasks_per_cycle: 5,
            wasm_enzyme_path: PathBuf::from("extensions/wasm/test_enzyme/target/wasm32-unknown-unknown/release/test_enzyme.wasm"),
            enable_auto_throttle: true,
            enable_constellation_updates: true,
            learning_config: UnifiedLearningConfig::default(),
            n_specialists: 3,
            hox_db_path: "data/hox_registry.db".to_string(),
        }
    }
}

/// Daemon state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnifiedDaemonState {
    Initializing,
    Running,
    Throttled,
    Learning,
    Error(String),
    ShuttingDown,
}

/// Daemon status report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedDaemonStatus {
    pub state: UnifiedDaemonState,
    pub uptime_seconds: f64,
    pub cycles_completed: u64,
    pub tasks_processed: u64,
    pub actions_executed: u64,
    pub health_summary: SystemHealthSummary,
    pub last_cycle_duration_ms: f64,
    pub learning_rate: f64,
    pub prediction_error: f64,
}

/// Unified Orchestration Daemon
/// Single loop that integrates all mathematical frameworks
pub struct UnifiedOrchestrationDaemon {
    pub config: UnifiedOrchestrationConfig,
    pub ingestor: MetadataIngestor,
    pub executor: ActionExecutor,
    pub learning_loop: Arc<parking_lot::RwLock<UnifiedLearningLoop>>,
    pub constellation: ConstellationCanvas,
    pub state: UnifiedDaemonState,
    pub start_time: Instant,
    pub cycles_completed: u64,
    pub tasks_processed: u64,
    pub actions_executed: u64,
    pub last_cycle_duration: Duration,
    pub last_cycle_result: Option<UnifiedCycleResult>,
    pub hox_registry: Arc<HoxRegistry>,
    pub enzyme_runner: Arc<EnzymeRunner>,
    pub autonomic_ns: AutonomicNervousSystem,
    pub splicing_engine: WasmSplicingEngine,
    pub nlm_sentinel: Arc<NlmSentinel>,
    pub chaos_monkey: Option<ChaosMonkey>,
    
    // Node features for spectral layout
    pub node_features: Vec<Vec<f64>>,
}

impl UnifiedOrchestrationDaemon {
    pub fn new(config: UnifiedOrchestrationConfig) -> Self {
        use parking_lot::RwLock;

        // Create specialists for the learning loop
        let specialist_ids: Vec<String> = (0..config.n_specialists)
            .map(|i| format!("specialist_{}", i))
            .collect();

        let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
            config.learning_config.clone(),
            config.n_specialists,
            specialist_ids,
        )));

        let executor = ActionExecutor::new(config.wasm_enzyme_path.clone());
        let ingestor_config = config.ingestor_config.clone();
        
        let hox_registry = Arc::new(HoxRegistry::new(&config.hox_db_path).unwrap());
        let enzyme_runner = Arc::new(EnzymeRunner::new().unwrap());
        let nlm_sentinel = Arc::new(NlmSentinel::new().unwrap());

        let splicing_engine_arc = Arc::new(WasmSplicingEngine::new(
            hox_registry.clone(),
            crate::workspace::WorkspacePaths::discover().root().clone(),
        ));

        let autonomic_ns = AutonomicNervousSystem::new(
            "AUTONOMIC_SYNAPSE",
            100, // 10Hz heartbeat
            enzyme_runner.clone(),
            hox_registry.clone(),
            splicing_engine_arc.clone(),
            learning_loop.clone(),
        ).unwrap();

        let chaos_monkey = ChaosMonkey::new(autonomic_ns.get_synapse());

        Self {
            config,
            ingestor: MetadataIngestor::new(ingestor_config),
            executor,
            learning_loop,
            constellation: ConstellationCanvas::new(),
            state: UnifiedDaemonState::Initializing,
            start_time: Instant::now(),
            cycles_completed: 0,
            tasks_processed: 0,
            actions_executed: 0,
            last_cycle_duration: Duration::ZERO,
            last_cycle_result: None,
            hox_registry,
            enzyme_runner,
            autonomic_ns,
            splicing_engine: (*splicing_engine_arc).clone(),
            nlm_sentinel,
            chaos_monkey: Some(chaos_monkey),
            node_features: Vec::new(),
        }
    }

    /// Run the daemon main loop
    pub async fn run(&mut self) -> Result<(), String> {
        self.state = UnifiedDaemonState::Running;
        println!("[UnifiedOrchestrationDaemon] Starting unified learning loop and autonomic heartbeat...");
        
        // Start the autonomic loop in the background
        self.autonomic_ns.clone().start();
        
        // Start the chaos monkey if enabled
        if let Some(chaos) = self.chaos_monkey.take() {
            chaos.start();
        }
        
        let mut interval = time::interval(self.config.cycle_interval);
        
        loop {
            interval.tick().await;
            
            let cycle_start = Instant::now();
            
            // Run one unified cycle
            match self.run_cycle().await {
                Ok(_) => {
                    self.cycles_completed += 1;
                    self.last_cycle_duration = cycle_start.elapsed();
                }
                Err(e) => {
                    self.state = UnifiedDaemonState::Error(e.clone());
                    eprintln!("[UnifiedOrchestrationDaemon] Cycle error: {}", e);
                }
            }
            
            // Check throttle state based on system phase
            if self.config.enable_auto_throttle {
                let health = self.learning_loop.read().get_health_summary();
                
                if matches!(health.phase, compute::thermodynamics::SystemPhase::Critical | compute::thermodynamics::SystemPhase::Disordered) {
                    self.state = UnifiedDaemonState::Throttled;
                    println!("[UnifiedOrchestrationDaemon] Throttled: phase={:?}, free_energy={:.3}", 
                        health.phase, health.free_energy);
                } else if matches!(self.state, UnifiedDaemonState::Throttled) 
                    && matches!(health.phase, compute::thermodynamics::SystemPhase::Ordered | compute::thermodynamics::SystemPhase::Mixed) {
                    self.state = UnifiedDaemonState::Running;
                    println!("[UnifiedOrchestrationDaemon] Resumed: phase={:?}", health.phase);
                }
            }
        }
    }

    /// Run a single unified cycle
    async fn run_cycle(&mut self) -> Result<(), String> {
        // Phase 1: OBSERVE - Ingest metadata
        let events = self.ingestor.process_pending_events();
        
        if events.is_empty() {
            // Even with no events, run learning loop with default observations
            let current_load = self.learning_loop.read().system_state.estimated_load;
            let observations = vec![current_load];
            let task_features = vec![0.5, 0.5, 0.5, 0.5]; // Default task
            let result = self.learning_loop.write().run_cycle(&observations, &task_features);
            self.last_cycle_result = Some(result);
            return Ok(());
        }
        
        // Phase 2: ESTIMATE - Convert events to task features
        let tasks: Vec<(String, Vec<f64>)> = events
            .into_iter()
            .take(self.config.max_tasks_per_cycle)
            .map(|(event, analysis)| {
                let features = self.extract_task_features(&event, &analysis);
                (format!("task_{}", event.timestamp), features)
            })
            .collect();
        
        if tasks.is_empty() {
            return Ok(());
        }
        
        // Phase 3: PREDICT + ROUTE - Run unified learning cycle
        let current_load = self.learning_loop.read().system_state.estimated_load;
        let observations = vec![current_load];
        let task_features = tasks[0].1.clone(); // Use first task for routing
        
        let result = self.learning_loop.write().run_cycle(&observations, &task_features);
        self.last_cycle_result = Some(result.clone());
        
        // Phase 4: ACT - Execute tasks based on routing
        for (task_id, features) in &tasks {
            let task_embedding = TaskEmbedding {
                task_id: task_id.clone(),
                features: features.clone(),
            };
            
            let routing = {
                let mut loop_guard = self.learning_loop.write();
                loop_guard.tensor_router.route(&task_embedding)
            };
            
            // Execute task via selected specialist
            let success = self.execute_task(task_id, &routing.selected_specialist).await;
            
            // Phase 5: LEARN - Update from outcome
            self.learning_loop.write().learn_from_outcome(features, &routing.selected_specialist, success);
            
            self.tasks_processed += 1;
            
            // Update constellation
            if self.config.enable_constellation_updates {
                self.update_constellation(task_id, &routing, success);
            }
        }
        
        // Update spectral layout periodically
        if self.cycles_completed % 5 == 0 && !self.node_features.is_empty() {
            self.update_spectral_layout();
        }
        
        Ok(())
    }

    /// Extract task features from metadata event
    fn extract_task_features(&self, event: &MetadataEvent, analysis: &MetadataAnalysis) -> Vec<f64> {
        let loop_guard = self.learning_loop.read();
        vec![
            analysis.predicted_complexity,           // Complexity
            if event.event_type == "file_modified" { 0.7 } else { 0.3 }, // Urgency
            analysis.entropy / 5.0,                  // Skill match (normalized entropy)
            1.0 - (loop_guard.system_state.token_availability), // Resource need
        ]
    }

    /// Execute task via specialist
    async fn execute_task(&self, task_id: &str, specialist_id: &str) -> bool {
        println!("[UnifiedOrchestrationDaemon] Executing {} via {}", task_id, specialist_id);
        
        // Check if specialist has a registered enzyme
        if let Ok(Some(cap)) = self.hox_registry.get_capability(specialist_id) {
            // Spawn enzyme via WASM runner
            let _ = self.enzyme_runner.spawn_enzyme(&cap.enzyme_hash, task_id).await;
            true
        } else {
            // Fallback to legacy action executor
            let action = ExecutableAction {
                id: task_id.to_string(),
                action_type: "process".to_string(),
                payload: serde_json::json!({"specialist": specialist_id}),
                target_enzyme: self.config.wasm_enzyme_path.to_string_lossy().to_string(),
            };
            
            let result = self.executor.execute(action).await;
            result.success
        }
    }

    /// Update constellation with task result
    fn update_constellation(&mut self, task_id: &str, routing: &crate::tensor_router::RoutingResult, success: bool) {
        // Update node features for spectral layout
        let feature_idx = self.constellation.nodes.iter().position(|n| n.id == task_id);
        
        if let Some(idx) = feature_idx {
            // Update existing node features
            if idx < self.node_features.len() {
                self.node_features[idx] = vec![
                    routing.confidence,
                    if success { 1.0 } else { 0.0 },
                    routing.entropy,
                ];
            }
        } else {
            // Add new node features
            self.node_features.push(vec![
                routing.confidence,
                if success { 1.0 } else { 0.0 },
                routing.entropy,
            ]);
        }
        
        // Update UI metrics
        let metrics = NodeMetrics {
            entropy: routing.entropy,
            confidence: routing.confidence,
            metabolic_risk: self.learning_loop.read().system_state.free_energy,
            centrality: 0.5,
            mdp_value: routing.confidence,
        };
        
        // Would update constellation node here
        let _ = metrics;
    }

    /// Update constellation positions using spectral layout
    fn update_spectral_layout(&mut self) {
        let n_nodes = self.node_features.len();
        if n_nodes < 2 {
            return;
        }
        
        let positions = spectral_layout_2d(n_nodes, &[]);
        
        // Update node positions
        for (i, (x, y)) in positions.iter().enumerate() {
            if i < self.constellation.nodes.len() {
                self.constellation.nodes[i].spatial_coord.x = *x as i32;
                self.constellation.nodes[i].spatial_coord.y = *y as i32;
            }
        }
    }

    /// Get current daemon status
    pub fn get_status(&self) -> UnifiedDaemonStatus {
        let loop_guard = self.learning_loop.read();
        let health = loop_guard.get_health_summary();
        
        UnifiedDaemonStatus {
            state: self.state.clone(),
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
            cycles_completed: self.cycles_completed,
            tasks_processed: self.tasks_processed,
            actions_executed: self.actions_executed,
            health_summary: health,
            last_cycle_duration_ms: self.last_cycle_duration.as_secs_f64() * 1000.0,
            learning_rate: loop_guard.config.learning_rate,
            prediction_error: loop_guard.system_state.prediction_error,
        }
    }

    /// Gracefully shutdown the daemon
    pub fn shutdown(&mut self) {
        self.state = UnifiedDaemonState::ShuttingDown;
        println!("[UnifiedOrchestrationDaemon] Shutting down...");
        println!("[UnifiedOrchestrationDaemon] Final stats: {} cycles, {} tasks",
            self.cycles_completed, self.tasks_processed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_daemon_creation() {
        let config = UnifiedOrchestrationConfig::default();
        let daemon = UnifiedOrchestrationDaemon::new(config);
        
        assert!(matches!(daemon.state, UnifiedDaemonState::Initializing));
        assert_eq!(daemon.cycles_completed, 0);
        assert_eq!(daemon.learning_loop.read().biology.specialist_metabolism.len(), 3);
    }

    #[test]
    fn test_unified_daemon_status() {
        let config = UnifiedOrchestrationConfig::default();
        let daemon = UnifiedOrchestrationDaemon::new(config);
        let status = daemon.get_status();
        
        assert!(matches!(status.state, UnifiedDaemonState::Initializing));
        assert_eq!(status.cycles_completed, 0);
        assert!(status.health_summary.free_energy.is_finite());
    }

    #[test]
    fn test_extract_task_features() {
        let config = UnifiedOrchestrationConfig::default();
        let daemon = UnifiedOrchestrationDaemon::new(config);
        
        let event = MetadataEvent {
            source: "test".to_string(),
            event_type: "file_modified".to_string(),
            timestamp: 0.0,
            data: serde_json::json!({"path": "test.rs"}),
            raw_bytes: None,
        };
        
        let analysis = MetadataAnalysis::default();
        let features = daemon.extract_task_features(&event, &analysis);
        
        assert_eq!(features.len(), 4);
        assert!(features[0] >= 0.0 && features[0] <= 1.0); // Complexity
        assert!(features[1] > 0.5); // Urgency for file_modified
    }
}
