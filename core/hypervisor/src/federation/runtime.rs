/// Federation Runtime System
/// 
/// Manages specialist lifecycle, model loading, proposal scheduling, and execution.
/// Runs the hive in production environments with:
/// - Specialist instantiation and lifecycle
/// - GGUF model loading and caching
/// - Asynchronous proposal generation
/// - Execution scheduling and arbitration
/// - Health monitoring and metrics
/// - Graceful shutdown

use crate::federation::specialist::{
    SpecialistId,
};
use crate::federation::bootstrap::DeploymentConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Model loaded in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedModel {
    pub specialist_id: SpecialistId,
    pub name: String,
    pub path: String,
    pub size_mb: u32,
    pub loaded_at: u64,
    pub last_used: u64,
    pub cached: bool,
}

impl LoadedModel {
    pub fn new(specialist_id: SpecialistId, path: String, size_mb: u32) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            specialist_id,
            name: format!("{:?}", specialist_id),
            path,
            size_mb,
            loaded_at: now,
            last_used: now,
            cached: false,
        }
    }

    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.loaded_at
    }

    pub fn mark_used(&mut self) {
        self.last_used = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

/// Model cache management
#[derive(Debug, Clone)]
pub struct ModelManager {
    pub loaded_models: HashMap<SpecialistId, LoadedModel>,
    pub cache_dir: String,
    pub max_cache_size_mb: u32,
    pub current_cache_size_mb: u32,
}

impl ModelManager {
    pub fn new(cache_dir: String, max_size_mb: u32) -> Self {
        Self {
            loaded_models: HashMap::new(),
            cache_dir,
            max_cache_size_mb: max_size_mb,
            current_cache_size_mb: 0,
        }
    }

    /// Load a model into memory
    pub fn load_model(&mut self, specialist_id: SpecialistId, path: String, size_mb: u32) -> Result<LoadedModel, String> {
        // Check if already loaded
        if let Some(model) = self.loaded_models.get_mut(&specialist_id) {
            model.mark_used();
            return Ok(model.clone());
        }

        // Check cache space
        if self.current_cache_size_mb + size_mb > self.max_cache_size_mb {
            self.evict_lru(size_mb)?;
        }

        let mut model = LoadedModel::new(specialist_id, path, size_mb);
        model.cached = true;

        self.loaded_models.insert(specialist_id, model.clone());
        self.current_cache_size_mb += size_mb;

        Ok(model)
    }

    /// Unload a model from cache
    pub fn unload_model(&mut self, specialist_id: &SpecialistId) -> Result<(), String> {
        if let Some(model) = self.loaded_models.remove(specialist_id) {
            self.current_cache_size_mb = self.current_cache_size_mb.saturating_sub(model.size_mb);
            Ok(())
        } else {
            Err(format!("Model not found: {:?}", specialist_id))
        }
    }

    /// Evict least recently used model
    fn evict_lru(&mut self, needed_space: u32) -> Result<(), String> {
        // Find LRU model
        let lru_id = self.loaded_models
            .iter()
            .min_by_key(|(_, model)| model.last_used)
            .map(|(id, _)| *id);

        if let Some(id) = lru_id {
            self.unload_model(&id)?;

            if self.current_cache_size_mb + needed_space > self.max_cache_size_mb {
                self.evict_lru(needed_space)?;
            }
        }

        Ok(())
    }

    /// Get cache stats
    pub fn cache_stats(&self) -> (usize, u32, u32) {
        let count = self.loaded_models.len();
        let used = self.current_cache_size_mb;
        let max = self.max_cache_size_mb;
        (count, used, max)
    }

    /// Clear cache
    pub fn clear(&mut self) {
        self.loaded_models.clear();
        self.current_cache_size_mb = 0;
    }
}

/// Execution metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub proposal_count: u64,
    pub execution_count: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_latency_ms: f32,
    pub total_runtime_ms: u64,
    pub last_execution: u64,
}

impl ExecutionMetrics {
    pub fn new() -> Self {
        Self {
            proposal_count: 0,
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_latency_ms: 0.0,
            total_runtime_ms: 0,
            last_execution: 0,
        }
    }

    pub fn record_execution(&mut self, latency_ms: u32, success: bool) {
        self.execution_count += 1;
        if success {
            self.success_count += 1;
        } else {
            self.failure_count += 1;
        }

        // Update average latency
        let total_latency = (self.avg_latency_ms * (self.execution_count - 1) as f32) + latency_ms as f32;
        self.avg_latency_ms = total_latency / self.execution_count as f32;
        self.total_runtime_ms += latency_ms as u64;
        self.last_execution = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    pub fn success_rate(&self) -> f32 {
        if self.execution_count == 0 {
            0.0
        } else {
            (self.success_count as f32 / self.execution_count as f32) * 100.0
        }
    }
}

/// Health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Specialist health
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistHealth {
    pub specialist_id: SpecialistId,
    pub status: HealthStatus,
    pub last_check: u64,
    pub failures: u32,
    pub error_message: Option<String>,
}

impl SpecialistHealth {
    pub fn new(specialist_id: SpecialistId) -> Self {
        Self {
            specialist_id,
            status: HealthStatus::Healthy,
            last_check: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            failures: 0,
            error_message: None,
        }
    }

    pub fn record_failure(&mut self, message: String) {
        self.failures += 1;
        self.error_message = Some(message);
        self.status = if self.failures > 3 {
            HealthStatus::Unhealthy
        } else if self.failures > 1 {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };
    }

    pub fn record_success(&mut self) {
        self.failures = 0;
        self.status = HealthStatus::Healthy;
        self.error_message = None;
    }
}

/// Hive runtime
pub struct HiveRuntime {
    pub config: DeploymentConfig,
    pub model_manager: ModelManager,
    pub metrics: HashMap<SpecialistId, ExecutionMetrics>,
    pub health: HashMap<SpecialistId, SpecialistHealth>,
    pub started_at: u64,
    pub is_running: bool,
}

impl HiveRuntime {
    pub fn new(config: DeploymentConfig) -> Self {
        let mut metrics = HashMap::new();
        let mut health = HashMap::new();

        // Initialize metrics and health for all modules
        for module in &config.manifest.modules {
            let id = match module.name() {
                "Sentinel" => SpecialistId::Sentinel,
                "Visionary" => SpecialistId::Visionary,
                "Omnipresent" => SpecialistId::Omnipresent,
                "Symbiotic" => SpecialistId::Symbiotic,
                "Phygital" => SpecialistId::Phygital,
                "Archivist" => SpecialistId::Archivist,
                _ => continue,
            };

            metrics.insert(id, ExecutionMetrics::new());
            health.insert(id, SpecialistHealth::new(id));
        }

        Self {
            config,
            model_manager: ModelManager::new(
                ".aaroneous/models".to_string(),
                2048, // 2GB default
            ),
            metrics,
            health,
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            is_running: false,
        }
    }

    /// Start the runtime
    pub fn start(&mut self) -> Result<String, String> {
        self.is_running = true;
        Ok(format!(
            "Hive started with {} modules",
            self.config.manifest.modules.len()
        ))
    }

    /// Stop the runtime
    pub fn stop(&mut self) -> Result<String, String> {
        self.is_running = false;
        self.model_manager.clear();
        Ok("Hive stopped".to_string())
    }

    /// Get runtime statistics
    pub fn stats(&self) -> RuntimeStats {
        let uptime_seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - self.started_at;

        let total_executions: u64 = self.metrics.values().map(|m| m.execution_count).sum();
        let total_successes: u64 = self.metrics.values().map(|m| m.success_count).sum();
        let avg_latency: f32 = if self.metrics.is_empty() {
            0.0
        } else {
            let sum: f32 = self.metrics.values().map(|m| m.avg_latency_ms).sum();
            sum / self.metrics.len() as f32
        };

        RuntimeStats {
            uptime_seconds,
            modules_loaded: self.model_manager.loaded_models.len(),
            total_executions,
            total_successes,
            success_rate: if total_executions == 0 {
                0.0
            } else {
                (total_successes as f32 / total_executions as f32) * 100.0
            },
            avg_latency_ms: avg_latency,
            healthy_specialists: self
                .health
                .values()
                .filter(|h| h.status == HealthStatus::Healthy)
                .count(),
            degraded_specialists: self
                .health
                .values()
                .filter(|h| h.status == HealthStatus::Degraded)
                .count(),
        }
    }

    /// Health check all specialists
    pub fn health_check(&mut self) -> HealthReport {
        let statuses: HashMap<SpecialistId, HealthStatus> = self
            .health
            .iter()
            .map(|(id, h)| (*id, h.status.clone()))
            .collect();

        let overall = if statuses.values().all(|s| s == &HealthStatus::Healthy) {
            HealthStatus::Healthy
        } else if statuses.values().any(|s| s == &HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else {
            HealthStatus::Degraded
        };

        HealthReport {
            overall,
            specialists: statuses,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Record specialist execution
    pub fn record_execution(
        &mut self,
        specialist_id: SpecialistId,
        latency_ms: u32,
        success: bool,
    ) {
        if let Some(metrics) = self.metrics.get_mut(&specialist_id) {
            metrics.record_execution(latency_ms, success);
        }

        if let Some(health) = self.health.get_mut(&specialist_id) {
            if success {
                health.record_success();
            } else {
                health.record_failure("Execution failed".to_string());
            }
        }
    }
}

/// Runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    pub uptime_seconds: u64,
    pub modules_loaded: usize,
    pub total_executions: u64,
    pub total_successes: u64,
    pub success_rate: f32,
    pub avg_latency_ms: f32,
    pub healthy_specialists: usize,
    pub degraded_specialists: usize,
}

/// Health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall: HealthStatus,
    pub specialists: HashMap<SpecialistId, HealthStatus>,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::bootstrap::DeploymentTarget;

    fn create_test_config() -> DeploymentConfig {
        DeploymentConfig::new(DeploymentTarget::Desktop)
    }

    #[test]
    fn test_loaded_model_creation() {
        let model = LoadedModel::new(
            SpecialistId::Visionary,
            "/models/visionary.gguf".to_string(),
            1000,
        );

        assert_eq!(model.specialist_id, SpecialistId::Visionary);
        assert_eq!(model.size_mb, 1000);
        assert_eq!(model.name, "Visionary");
    }

    #[test]
    fn test_loaded_model_mark_used() {
        let mut model = LoadedModel::new(
            SpecialistId::Omnipresent,
            "/models/omnipresent.gguf".to_string(),
            1000,
        );

        let old_last_used = model.last_used;
        model.mark_used();
        assert!(model.last_used >= old_last_used);
    }

    #[test]
    fn test_model_manager_creation() {
        let manager = ModelManager::new(".cache".to_string(), 2048);

        assert_eq!(manager.cache_dir, ".cache");
        assert_eq!(manager.max_cache_size_mb, 2048);
        assert_eq!(manager.current_cache_size_mb, 0);
    }

    #[test]
    fn test_model_manager_load() {
        let mut manager = ModelManager::new(".cache".to_string(), 2048);

        let result = manager.load_model(
            SpecialistId::Visionary,
            "/models/visionary.gguf".to_string(),
            1000,
        );

        assert!(result.is_ok());
        assert_eq!(manager.current_cache_size_mb, 1000);
        assert!(manager.loaded_models.contains_key(&SpecialistId::Visionary));
    }

    #[test]
    fn test_model_manager_unload() {
        let mut manager = ModelManager::new(".cache".to_string(), 2048);

        manager
            .load_model(
                SpecialistId::Visionary,
                "/models/visionary.gguf".to_string(),
                1000,
            )
            .unwrap();

        let result = manager.unload_model(&SpecialistId::Visionary);

        assert!(result.is_ok());
        assert_eq!(manager.current_cache_size_mb, 0);
    }

    #[test]
    fn test_model_manager_cache_stats() {
        let mut manager = ModelManager::new(".cache".to_string(), 2048);

        manager
            .load_model(
                SpecialistId::Visionary,
                "/models/visionary.gguf".to_string(),
                1000,
            )
            .unwrap();

        let (count, used, max) = manager.cache_stats();

        assert_eq!(count, 1);
        assert_eq!(used, 1000);
        assert_eq!(max, 2048);
    }

    #[test]
    fn test_model_manager_overflow() {
        let mut manager = ModelManager::new(".cache".to_string(), 1500);

        manager
            .load_model(
                SpecialistId::Visionary,
                "/models/visionary.gguf".to_string(),
                1000,
            )
            .unwrap();

        // This should trigger LRU eviction
        let result = manager.load_model(
            SpecialistId::Omnipresent,
            "/models/omnipresent.gguf".to_string(),
            1000,
        );

        // Should succeed by evicting older model
        assert!(result.is_ok());
    }

    #[test]
    fn test_execution_metrics_record() {
        let mut metrics = ExecutionMetrics::new();

        metrics.record_execution(100, true);
        metrics.record_execution(150, true);
        metrics.record_execution(200, false);

        assert_eq!(metrics.execution_count, 3);
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.failure_count, 1);
    }

    #[test]
    fn test_execution_metrics_success_rate() {
        let mut metrics = ExecutionMetrics::new();

        metrics.record_execution(100, true);
        metrics.record_execution(150, true);
        metrics.record_execution(200, false);

        let rate = metrics.success_rate();
        assert!(rate >= 66.0 && rate <= 67.0); // ~66.7%
    }

    #[test]
    fn test_specialist_health_creation() {
        let health = SpecialistHealth::new(SpecialistId::Visionary);

        assert_eq!(health.specialist_id, SpecialistId::Visionary);
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.failures, 0);
    }

    #[test]
    fn test_specialist_health_record_failure() {
        let mut health = SpecialistHealth::new(SpecialistId::Visionary);

        health.record_failure("Error 1".to_string());
        assert_eq!(health.status, HealthStatus::Healthy); // 1 failure is ok

        health.record_failure("Error 2".to_string());
        assert_eq!(health.status, HealthStatus::Degraded); // 2 failures = degraded

        health.record_failure("Error 3".to_string());
        assert_eq!(health.status, HealthStatus::Degraded); // 3 = still degraded

        health.record_failure("Error 4".to_string());
        assert_eq!(health.status, HealthStatus::Unhealthy); // 4+ = unhealthy
    }

    #[test]
    fn test_specialist_health_recovery() {
        let mut health = SpecialistHealth::new(SpecialistId::Visionary);

        health.record_failure("Error".to_string());
        health.record_failure("Error".to_string());
        assert_eq!(health.status, HealthStatus::Degraded);

        health.record_success();
        assert_eq!(health.status, HealthStatus::Healthy);
        assert_eq!(health.failures, 0);
    }

    #[test]
    fn test_hive_runtime_creation() {
        let config = create_test_config();
        let runtime = HiveRuntime::new(config);

        assert!(!runtime.is_running);
        assert_eq!(runtime.metrics.len(), 6); // All specialists
    }

    #[test]
    fn test_hive_runtime_start_stop() {
        let config = create_test_config();
        let mut runtime = HiveRuntime::new(config);

        let start_result = runtime.start();
        assert!(start_result.is_ok());
        assert!(runtime.is_running);

        let stop_result = runtime.stop();
        assert!(stop_result.is_ok());
        assert!(!runtime.is_running);
    }

    #[test]
    fn test_hive_runtime_stats() {
        let config = create_test_config();
        let mut runtime = HiveRuntime::new(config);

        runtime.record_execution(SpecialistId::Visionary, 100, true);
        runtime.record_execution(SpecialistId::Visionary, 150, true);
        runtime.record_execution(SpecialistId::Omnipresent, 200, false);

        let stats = runtime.stats();

        assert_eq!(stats.total_executions, 3);
        assert_eq!(stats.total_successes, 2);
        assert!(stats.success_rate >= 66.0);
    }

    #[test]
    fn test_hive_runtime_health_check() {
        let config = create_test_config();
        let mut runtime = HiveRuntime::new(config);

        let report = runtime.health_check();

        assert_eq!(report.overall, HealthStatus::Healthy);
        assert_eq!(report.specialists.len(), 6);
    }

    #[test]
    fn test_hive_runtime_record_execution() {
        let config = create_test_config();
        let mut runtime = HiveRuntime::new(config);

        runtime.record_execution(SpecialistId::Visionary, 100, true);

        assert_eq!(runtime.metrics[&SpecialistId::Visionary].execution_count, 1);
        assert_eq!(runtime.health[&SpecialistId::Visionary].status, HealthStatus::Healthy);
    }

    #[test]
    fn test_hive_runtime_unhealthy_specialist() {
        let config = create_test_config();
        let mut runtime = HiveRuntime::new(config);

        // Record multiple failures
        for _ in 0..5 {
            runtime.record_execution(SpecialistId::Visionary, 100, false);
        }

        let report = runtime.health_check();

        assert_eq!(report.overall, HealthStatus::Unhealthy);
        assert_eq!(
            report.specialists[&SpecialistId::Visionary],
            HealthStatus::Unhealthy
        );
    }
}
