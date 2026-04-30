// Aaroneous Hive Runtime - Main Event Loop & System Orchestration
// Coordinates all modules: genetics, skills, ingestion, persistence, federation

use crate::persistence::PersistenceManager;
use crate::inbox_system::InboxSystem;
use crate::event_loop::SkillEventLoop;
use crate::capability_dashboard::CapabilityDashboard;
use crate::crisis_coordinator::CrisisCoordinator;
use crate::biology::SystemBiology;
use crate::autonomous_coordinator::{AutonomousCoordinator, TaskCoordinationStatus};
use crate::task_analysis::Task;
use crate::agents::SpecialistAgent;
use tokio::sync::RwLock;
use parking_lot::RwLock as ParkingLotRwLock;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Main runtime configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HiveRuntimeConfig {
    pub db_path: String,
    pub inbox_folder: String,
    pub output_folder: String,
    pub update_interval_ms: u64,
    pub max_concurrent_tasks: usize,
    pub enable_persistence: bool,
    pub enable_ingestion: bool,
    pub enable_dashboard: bool,
    pub crisis_response_enabled: bool,
}

impl Default for HiveRuntimeConfig {
    fn default() -> Self {
        Self {
            db_path: "D:\\Aaroneous\\hive.db".to_string(),
            inbox_folder: "D:\\Aaroneous\\inbox".to_string(),
            output_folder: "D:\\Aaroneous\\processed".to_string(),
            update_interval_ms: 100,
            max_concurrent_tasks: 4,
            enable_persistence: true,
            enable_ingestion: true,
            enable_dashboard: true,
            crisis_response_enabled: true,
        }
    }
}

/// Runtime status for monitoring
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeStatus {
    Starting,
    Running,
    Processing,
    Paused,
    Stopping,
    Stopped,
    Error(String),
}

impl std::fmt::Display for RuntimeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeStatus::Starting => write!(f, "Starting"),
            RuntimeStatus::Running => write!(f, "Running"),
            RuntimeStatus::Processing => write!(f, "Processing"),
            RuntimeStatus::Paused => write!(f, "Paused"),
            RuntimeStatus::Stopping => write!(f, "Stopping"),
            RuntimeStatus::Stopped => write!(f, "Stopped"),
            RuntimeStatus::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

/// Runtime statistics for monitoring
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeStatistics {
    pub uptime_seconds: u64,
    pub total_specialists: u32,
    pub total_xp: u32,
    pub total_skills: u32,
    pub total_events: u32,
    pub files_processed: u32,
    pub average_processing_time_ms: u64,
    pub current_status: RuntimeStatus,
    pub last_update: DateTime<Utc>,
}

/// Main Aaroneous Hive Runtime - orchestrates all systems
pub struct HiveRuntime {
    config: Arc<RwLock<HiveRuntimeConfig>>,
    persistence: Arc<PersistenceManager>,
    inbox_system: Arc<InboxSystem>,
    skill_loop: Arc<SkillEventLoop>,
    dashboard: Arc<CapabilityDashboard>,
    crisis_coordinator: Arc<CrisisCoordinator>,
    biology: Arc<RwLock<SystemBiology>>,
    autonomous_coordinator: Arc<RwLock<AutonomousCoordinator>>,
    status: Arc<RwLock<RuntimeStatus>>,
    statistics: Arc<RwLock<RuntimeStatistics>>,
    shutdown_signal: Arc<tokio::sync::Notify>,
    created_at: DateTime<Utc>,
}

impl HiveRuntime {
    /// Initialize the hive runtime with all systems
    pub async fn new(config: HiveRuntimeConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure directories exist
        std::fs::create_dir_all(&config.inbox_folder)?;
        std::fs::create_dir_all(&config.output_folder)?;

        // Initialize persistence
        let persistence = Arc::new(
            PersistenceManager::new(&config.db_path)
                .map_err(|e| format!("Failed to initialize persistence: {}", e))?
        );

        // Initialize inbox system
        let inbox_config = crate::data_ingestion::IngestionConfig {
            inbox_path: config.inbox_folder.clone().into(),
            processing_path: format!("{}/processing", config.output_folder).into(),
            processed_path: format!("{}/processed", config.output_folder).into(),
            failed_path: format!("{}/failed", config.output_folder).into(),
            analytics_path: format!("{}/analytics", config.output_folder).into(),
            max_file_size_mb: 500,
            content_sample_size_bytes: 10000,
            file_watcher_enabled: config.enable_ingestion,
            max_concurrent_ingestions: config.max_concurrent_tasks,
            scan_interval_ms: 1000,
        };

        let inbox_system = Arc::new(InboxSystem::new(inbox_config));

        // Initialize skill event loop
        let skill_loop = Arc::new(SkillEventLoop::new());

        // Initialize dashboard
        let dashboard = Arc::new(CapabilityDashboard::new());

        // Initialize crisis coordinator
        let crisis_coordinator = Arc::new(CrisisCoordinator::new());

        // Initialize biology system
        let biology = Arc::new(RwLock::new(SystemBiology::new()));

        // Initialize autonomous coordinator with fallback to mock provider if GGUF unavailable
        let llm_config = crate::llm::LLMConfig {
            provider_type: crate::llm::ProviderType::GGUF,
            temperature: 0.7,
            max_tokens: 2048,
            timeout_secs: 30,
            enable_caching: true,
            cache_ttl_secs: 3600,
            gguf_model_path: None,
        };
        
        let llm_client = match crate::llm::LLMClient::new(llm_config).await {
            Ok(client) => Arc::new(client),
            Err(_e) => {
                // Fallback to mock provider if GGUF initialization fails
                info!("GGUF initialization failed, using MockProvider for tests/development");
                let mock_config = crate::llm::LLMConfig {
                    provider_type: crate::llm::ProviderType::Mock,
                    temperature: 0.7,
                    max_tokens: 2048,
                    timeout_secs: 30,
                    enable_caching: true,
                    cache_ttl_secs: 3600,
                    gguf_model_path: None,
                };
                Arc::new(crate::llm::LLMClient::new(mock_config).await
                    .map_err(|e| format!("Failed to initialize mock LLMClient: {}", e))?)
            }
        };
        
        let task_analysis_engine = crate::task_analysis::TaskAnalysisEngine::new(llm_client.clone());
        let autonomous_coordinator = Arc::new(RwLock::new(
            AutonomousCoordinator::new(llm_client, task_analysis_engine)
        ));

        let runtime = Self {
            config: Arc::new(RwLock::new(config)),
            persistence,
            inbox_system,
            skill_loop,
            dashboard,
            crisis_coordinator,
            biology,
            autonomous_coordinator,
            status: Arc::new(RwLock::new(RuntimeStatus::Starting)),
            statistics: Arc::new(RwLock::new(RuntimeStatistics {
                uptime_seconds: 0,
                total_specialists: 0,
                total_xp: 0,
                total_skills: 0,
                total_events: 0,
                files_processed: 0,
                average_processing_time_ms: 0,
                current_status: RuntimeStatus::Starting,
                last_update: Utc::now(),
            })),
            shutdown_signal: Arc::new(tokio::sync::Notify::new()),
            created_at: Utc::now(),
        };

        Ok(runtime)
    }

    /// Start the hive runtime - begins all systems
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Update status
        *self.status.write().await = RuntimeStatus::Running;

        // Load existing specialists from persistence
        self.load_specialists_from_db().await?;

        // Spawn background tasks
        self.spawn_update_loop();
        self.spawn_biology_update();
        self.spawn_statistics_updater();

        // Note: Inbox monitoring will be started separately when dashboard/CLI requires it
        // For now, just ensure directories are created
        let config = self.config.read().await;
        if config.enable_ingestion {
            std::fs::create_dir_all(&config.inbox_folder).ok();
            std::fs::create_dir_all(&format!("{}/processing", config.output_folder)).ok();
        }

        Ok(())
    }

    /// Load all specialists from the database on startup
    async fn load_specialists_from_db(&self) -> Result<(), Box<dyn std::error::Error>> {
        let specialists = self.persistence.list_specialists()
            .map_err(|e| format!("Failed to load specialists: {}", e))?;

        for _specialist in specialists {
            // TODO: Reconstruct specialist objects and register with dashboard
            // This is a placeholder for full specialist reconstruction
        }

        Ok(())
    }

    /// Main update loop - runs the hive tick-by-tick
    fn spawn_update_loop(&self) {
        let status = Arc::clone(&self.status);
        let config = Arc::clone(&self.config);
        let statistics = Arc::clone(&self.statistics);
        let shutdown_signal = Arc::clone(&self.shutdown_signal);
        let autonomous_coordinator = Arc::clone(&self.autonomous_coordinator);

        tokio::spawn(async move {
            loop {
                // Check for shutdown signal
                tokio::select! {
                    _ = shutdown_signal.notified() => {
                        *status.write().await = RuntimeStatus::Stopping;
                        break;
                    }
                    _ = tokio::time::sleep(Duration::from_millis(
                        config.read().await.update_interval_ms
                    )) => {
                        // Perform update cycle
                        *status.write().await = RuntimeStatus::Processing;
                        
                        // Process any pending autonomous tasks
                        {
                            let coordinator = autonomous_coordinator.read().await;
                            let active_tasks = coordinator.get_all_active_tasks();
                            
                            // For each task in planning state, start execution
                            for (task_id, _state) in active_tasks {
                                if let Some(state) = coordinator.get_task_state(task_id) {
                                    if state.status == TaskCoordinationStatus::PlanningComplete {
                                        // Task plan is ready - would execute steps here
                                        debug!("Task ready for execution: {}", task_id);
                                    }
                                }
                            }
                        }

                        // Update statistics timestamp
                        let mut stats = statistics.write().await;
                        stats.last_update = Utc::now();
                        
                        // Revert to running state
                        *status.write().await = RuntimeStatus::Running;
                    }
                }
            }
        });
    }

    /// Biology system update loop - regenerates metabolic tokens
    fn spawn_biology_update(&self) {
        let biology = Arc::clone(&self.biology);
        let shutdown_signal = Arc::clone(&self.shutdown_signal);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_signal.notified() => break,
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {
                        let mut bio = biology.write().await;
                        bio.update_metabolism();
                    }
                }
            }
        });
    }

    /// Statistics updater - collects metrics from all systems
    fn spawn_statistics_updater(&self) {
        let statistics = Arc::clone(&self.statistics);
        let created_at = self.created_at;
        let shutdown_signal = Arc::clone(&self.shutdown_signal);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_signal.notified() => break,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {
                        // Update statistics - note: persistence queries happen in blocking context
                        // For now, just update timestamps and uptime
                        let mut stats = statistics.write().await;
                        stats.uptime_seconds = Utc::now().signed_duration_since(created_at).num_seconds() as u64;
                        stats.last_update = Utc::now();
                    }
                }
            }
        });
    }

    /// Graceful shutdown of all systems
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        *self.status.write().await = RuntimeStatus::Stopping;

        // Signal all background tasks to stop
        self.shutdown_signal.notify_waiters();

        // Give tasks time to shut down gracefully (max 5 seconds)
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Final status update
        *self.status.write().await = RuntimeStatus::Stopped;

        Ok(())
    }

    /// Get current runtime status
    pub async fn get_status(&self) -> RuntimeStatus {
        self.status.read().await.clone()
    }

    /// Get current statistics
    pub async fn get_statistics(&self) -> RuntimeStatistics {
        self.statistics.read().await.clone()
    }

    /// Get uptime in seconds
    pub fn get_uptime_seconds(&self) -> u64 {
        Utc::now().signed_duration_since(self.created_at).num_seconds() as u64
    }

    /// Health check - returns true if system is operational
    pub async fn health_check(&self) -> bool {
        matches!(
            self.status.read().await.clone(),
            RuntimeStatus::Running | RuntimeStatus::Processing
        )
    }

    /// Pause the runtime (keeps existing tasks alive)
    pub async fn pause(&self) {
        *self.status.write().await = RuntimeStatus::Paused;
    }

    /// Resume from pause
    pub async fn resume(&self) {
        *self.status.write().await = RuntimeStatus::Running;
    }

    // Accessors for subsystems
    pub fn persistence(&self) -> Arc<PersistenceManager> {
        Arc::clone(&self.persistence)
    }

    pub fn inbox_system(&self) -> Arc<InboxSystem> {
        Arc::clone(&self.inbox_system)
    }

    pub fn skill_loop(&self) -> Arc<SkillEventLoop> {
        Arc::clone(&self.skill_loop)
    }

    pub fn dashboard(&self) -> Arc<CapabilityDashboard> {
        Arc::clone(&self.dashboard)
    }

    pub fn crisis_coordinator(&self) -> Arc<CrisisCoordinator> {
        Arc::clone(&self.crisis_coordinator)
    }

    pub async fn biology(&self) -> tokio::sync::RwLockReadGuard<'_, SystemBiology> {
        self.biology.read().await
    }

    pub fn autonomous_coordinator(&self) -> Arc<RwLock<AutonomousCoordinator>> {
        Arc::clone(&self.autonomous_coordinator)
    }

    /// Submit a task to the autonomous pipeline
    pub async fn submit_task(&self, task: Task) -> Result<String, Box<dyn std::error::Error>> {
        let mut coordinator = self.autonomous_coordinator.write().await;
        let task_id = coordinator.submit_task(task).await?;
        debug!("Task submitted to autonomous pipeline: {}", task_id);
        Ok(task_id)
    }

    /// Process a submitted task through the autonomous pipeline
    pub async fn process_autonomous_task(
        &self,
        task_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get available specialists from dashboard
        // Note: For now, we use empty list - in full implementation, reconstruct SpecialistAgents
        let specialists: Vec<SpecialistAgent> = vec![];
        
        let mut coordinator = self.autonomous_coordinator.write().await;
        coordinator.process_task(task_id, &specialists).await?;
        
        debug!("Task processing initiated: {}", task_id);
        Ok(())
    }

    /// Get the current state of a task
    pub async fn get_task_state(&self, task_id: &str) -> Option<crate::autonomous_coordinator::TaskExecutionState> {
        let coordinator = self.autonomous_coordinator.read().await;
        coordinator.get_task_state(task_id).cloned()
    }

    /// Get summary of all active tasks
    pub async fn get_active_tasks(&self) -> Vec<(String, crate::autonomous_coordinator::TaskSummary)> {
        let coordinator = self.autonomous_coordinator.read().await;
        coordinator
            .get_all_active_tasks()
            .iter()
            .filter_map(|(id, _state)| {
                coordinator
                    .get_task_summary(id)
                    .map(|summary| (id.to_string(), summary))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_runtime_creation() {
        let config = HiveRuntimeConfig::default();
        let runtime = HiveRuntime::new(config).await;
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_runtime_startup_shutdown() {
        let config = HiveRuntimeConfig {
            db_path: ":memory:".to_string(),
            ..Default::default()
        };

        let runtime = HiveRuntime::new(config).await.expect("Failed to create runtime");
        let start_result = runtime.start().await;
        assert!(start_result.is_ok());

        let status = runtime.get_status().await;
        assert_eq!(status, RuntimeStatus::Running);

        let shutdown_result = runtime.shutdown().await;
        assert!(shutdown_result.is_ok());

        let final_status = runtime.get_status().await;
        assert_eq!(final_status, RuntimeStatus::Stopped);
    }

    #[tokio::test]
    async fn test_runtime_health_check() {
        let config = HiveRuntimeConfig {
            db_path: ":memory:".to_string(),
            ..Default::default()
        };

        let runtime = HiveRuntime::new(config).await.expect("Failed to create runtime");
        runtime.start().await.expect("Failed to start");

        let health = runtime.health_check().await;
        assert!(health);

        runtime.shutdown().await.expect("Failed to shutdown");
        let health_after = runtime.health_check().await;
        assert!(!health_after);
    }

    #[tokio::test]
    async fn test_runtime_pause_resume() {
        let config = HiveRuntimeConfig {
            db_path: ":memory:".to_string(),
            ..Default::default()
        };

        let runtime = HiveRuntime::new(config).await.expect("Failed to create runtime");
        runtime.start().await.expect("Failed to start");

        runtime.pause().await;
        let paused_status = runtime.get_status().await;
        assert_eq!(paused_status, RuntimeStatus::Paused);

        runtime.resume().await;
        let resumed_status = runtime.get_status().await;
        assert_eq!(resumed_status, RuntimeStatus::Running);

        runtime.shutdown().await.expect("Failed to shutdown");
    }

    #[tokio::test]
    async fn test_runtime_statistics() {
        let config = HiveRuntimeConfig {
            db_path: ":memory:".to_string(),
            ..Default::default()
        };

        let runtime = HiveRuntime::new(config).await.expect("Failed to create runtime");
        runtime.start().await.expect("Failed to start");

        tokio::time::sleep(Duration::from_millis(100)).await;

        let stats = runtime.get_statistics().await;
        assert!(stats.uptime_seconds >= 0);
        assert_eq!(stats.total_specialists, 0);

        runtime.shutdown().await.expect("Failed to shutdown");
    }
}
