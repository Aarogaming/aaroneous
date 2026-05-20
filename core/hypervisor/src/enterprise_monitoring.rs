/// Enterprise Performance Monitoring & Metrics Dashboard
///
/// Real-time monitoring system for production environments:
/// - System health tracking (CPU, memory, latency)
/// - Request/task throughput monitoring
/// - Cache hit rate analytics
/// - Error rate tracking & alerting
/// - Multi-tenant performance isolation
/// - Dashboards and reporting

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::info;

/// System health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage_percent: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub network_latency_ms: f32,
    pub is_healthy: bool,
}

impl SystemHealth {
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            network_latency_ms: 0.0,
            is_healthy: true,
        }
    }

    pub fn check_health(&mut self) {
        self.is_healthy = self.cpu_usage_percent < 85.0
            && self.memory_usage_percent < 85.0
            && self.disk_usage_percent < 90.0
            && self.network_latency_ms < 500.0;
    }
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics for operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub timestamp: DateTime<Utc>,
    pub operation_name: String,
    pub count: u64,
    pub min_latency_ms: f32,
    pub max_latency_ms: f32,
    pub avg_latency_ms: f32,
    pub p95_latency_ms: f32,
    pub p99_latency_ms: f32,
    pub success_rate: f32, // 0-100
    pub error_count: u64,
}

impl PerformanceMetrics {
    pub fn new(operation_name: String) -> Self {
        Self {
            timestamp: Utc::now(),
            operation_name,
            count: 0,
            min_latency_ms: f32::INFINITY,
            max_latency_ms: 0.0,
            avg_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            success_rate: 100.0,
            error_count: 0,
        }
    }

    pub fn calculate_from_samples(&mut self, latencies: &[f32], errors: u64) {
        if latencies.is_empty() {
            return;
        }

        self.count = latencies.len() as u64;
        self.min_latency_ms = latencies.iter().cloned().fold(f32::INFINITY, f32::min);
        self.max_latency_ms = latencies.iter().cloned().fold(0.0, f32::max);
        self.avg_latency_ms = latencies.iter().sum::<f32>() / latencies.len() as f32;
        self.error_count = errors;

        let total = self.count + errors;
        self.success_rate = if total > 0 {
            (self.count as f32 / total as f32) * 100.0
        } else {
            100.0
        };

        // Percentile calculation
        let mut sorted = latencies.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let p95_idx = ((sorted.len() as f32 * 0.95) as usize).min(sorted.len() - 1);
        let p99_idx = ((sorted.len() as f32 * 0.99) as usize).min(sorted.len() - 1);
        
        self.p95_latency_ms = sorted[p95_idx];
        self.p99_latency_ms = sorted[p99_idx];
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub timestamp: DateTime<Utc>,
    pub cache_level: String,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f32,
    pub total_size_bytes: u64,
}

impl CacheStats {
    pub fn new(cache_level: String) -> Self {
        Self {
            timestamp: Utc::now(),
            cache_level,
            total_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            hit_rate: 0.0,
            total_size_bytes: 0,
        }
    }

    pub fn update_hit_rate(&mut self) {
        if self.total_requests > 0 {
            self.hit_rate = (self.cache_hits as f32 / self.total_requests as f32) * 100.0;
        }
    }
}

/// Tenant-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMetrics {
    pub tenant_id: String,
    pub timestamp: DateTime<Utc>,
    pub total_requests: u64,
    pub total_errors: u64,
    pub api_calls: u64,
    pub storage_used_bytes: u64,
    pub users_active: u32,
}

impl TenantMetrics {
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            timestamp: Utc::now(),
            total_requests: 0,
            total_errors: 0,
            api_calls: 0,
            storage_used_bytes: 0,
            users_active: 0,
        }
    }

    pub fn error_rate(&self) -> f32 {
        if self.total_requests > 0 {
            (self.total_errors as f32 / self.total_requests as f32) * 100.0
        } else {
            0.0
        }
    }
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub metric: String,
    pub threshold: f32,
    pub comparison: AlertComparison,
    pub duration_secs: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertComparison {
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
}

/// Monitoring dashboard
pub struct MonitoringDashboard {
    system_health: Arc<RwLock<SystemHealth>>,
    performance_metrics: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
    cache_stats: Arc<RwLock<HashMap<String, CacheStats>>>,
    tenant_metrics: Arc<RwLock<HashMap<String, TenantMetrics>>>,
    alerts: Arc<RwLock<Vec<Alert>>>,
}

impl MonitoringDashboard {
    pub fn new() -> Self {
        Self {
            system_health: Arc::new(RwLock::new(SystemHealth::new())),
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
            cache_stats: Arc::new(RwLock::new(HashMap::new())),
            tenant_metrics: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn update_system_health(&self, health: SystemHealth) {
        *self.system_health.write().unwrap() = health;
    }

    pub fn get_system_health(&self) -> SystemHealth {
        self.system_health.read().unwrap().clone()
    }

    pub fn record_operation(
        &self,
        operation: String,
        latency_ms: f32,
        success: bool,
    ) {
        let mut metrics = self.performance_metrics.write().unwrap();
        let entry = metrics
            .entry(operation)
            .or_insert_with(|| PerformanceMetrics::new(String::new()));

        entry.count += 1;
        entry.min_latency_ms = entry.min_latency_ms.min(latency_ms);
        entry.max_latency_ms = entry.max_latency_ms.max(latency_ms);
        
        if !success {
            entry.error_count += 1;
        }

        let total = entry.count + entry.error_count;
        entry.success_rate = if total > 0 {
            (entry.count as f32 / total as f32) * 100.0
        } else {
            100.0
        };
    }

    pub fn get_performance_metrics(&self, operation: &str) -> Option<PerformanceMetrics> {
        let metrics = self.performance_metrics.read().unwrap();
        metrics.get(operation).cloned()
    }

    pub fn record_cache_hit(&self, cache_level: &str) {
        let mut stats = self.cache_stats.write().unwrap();
        let entry = stats
            .entry(cache_level.to_string())
            .or_insert_with(|| CacheStats::new(cache_level.to_string()));

        entry.total_requests += 1;
        entry.cache_hits += 1;
        entry.update_hit_rate();
    }

    pub fn record_cache_miss(&self, cache_level: &str) {
        let mut stats = self.cache_stats.write().unwrap();
        let entry = stats
            .entry(cache_level.to_string())
            .or_insert_with(|| CacheStats::new(cache_level.to_string()));

        entry.total_requests += 1;
        entry.cache_misses += 1;
        entry.update_hit_rate();
    }

    pub fn get_cache_stats(&self, cache_level: &str) -> Option<CacheStats> {
        let stats = self.cache_stats.read().unwrap();
        stats.get(cache_level).cloned()
    }

    pub fn record_tenant_activity(
        &self,
        tenant_id: &str,
        requests: u64,
        errors: u64,
        api_calls: u64,
    ) {
        let mut metrics = self.tenant_metrics.write().unwrap();
        let entry = metrics
            .entry(tenant_id.to_string())
            .or_insert_with(|| TenantMetrics::new(tenant_id.to_string()));

        entry.total_requests += requests;
        entry.total_errors += errors;
        entry.api_calls += api_calls;
    }

    pub fn get_tenant_metrics(&self, tenant_id: &str) -> Option<TenantMetrics> {
        let metrics = self.tenant_metrics.read().unwrap();
        metrics.get(tenant_id).cloned()
    }

    pub fn add_alert(&self, alert: Alert) -> String {
        let alert_id = alert.id.clone();
        let mut alerts = self.alerts.write().unwrap();
        alerts.push(alert);
        info!("Alert created: {}", alert_id);
        alert_id
    }

    pub fn get_active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        alerts
            .iter()
            .filter(|a| a.is_active)
            .cloned()
            .collect()
    }

    pub fn check_alert_conditions(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().unwrap();
        let health = self.system_health.read().unwrap();

        alerts
            .iter()
            .filter(|alert| self.check_alert(alert, &health))
            .cloned()
            .collect()
    }

    fn check_alert(&self, alert: &Alert, health: &SystemHealth) -> bool {
        if !alert.is_active {
            return false;
        }

        let value = match alert.metric.as_str() {
            "cpu" => health.cpu_usage_percent,
            "memory" => health.memory_usage_percent,
            "disk" => health.disk_usage_percent,
            "latency" => health.network_latency_ms,
            _ => return false,
        };

        match alert.comparison {
            AlertComparison::GreaterThan => value > alert.threshold,
            AlertComparison::LessThan => value < alert.threshold,
            AlertComparison::GreaterOrEqual => value >= alert.threshold,
            AlertComparison::LessOrEqual => value <= alert.threshold,
            AlertComparison::Equal => (value - alert.threshold).abs() < 0.01,
        }
    }

    pub fn generate_report(&self) -> DashboardReport {
        DashboardReport {
            generated_at: Utc::now(),
            system_health: self.system_health.read().unwrap().clone(),
            top_operations: self.get_top_operations(5),
            cache_summary: self.get_cache_summary(),
            active_alerts: self.get_active_alerts(),
        }
    }

    fn get_top_operations(&self, limit: usize) -> Vec<PerformanceMetrics> {
        let metrics = self.performance_metrics.read().unwrap();
        let mut ops: Vec<_> = metrics.values().cloned().collect();
        ops.sort_by(|a, b| b.count.cmp(&a.count));
        ops.into_iter().take(limit).collect()
    }

    fn get_cache_summary(&self) -> HashMap<String, f32> {
        let stats = self.cache_stats.read().unwrap();
        stats
            .iter()
            .map(|(k, v)| (k.clone(), v.hit_rate))
            .collect()
    }
}

impl Default for MonitoringDashboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Dashboard report for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardReport {
    pub generated_at: DateTime<Utc>,
    pub system_health: SystemHealth,
    pub top_operations: Vec<PerformanceMetrics>,
    pub cache_summary: HashMap<String, f32>,
    pub active_alerts: Vec<Alert>,
}

impl DashboardReport {
    pub fn display(&self) {
        info!("╔════════════════════════════════════════════╗");
        info!("║       PERFORMANCE MONITORING REPORT         ║");
        info!("║       Generated: {:?}", self.generated_at);
        info!("╠════════════════════════════════════════════╣");
        info!("║ System Health: {}", if self.system_health.is_healthy { "✅ Healthy" } else { "⚠️ Warning" });
        info!("║ CPU: {:.1}% | Memory: {:.1}% | Disk: {:.1}%", 
            self.system_health.cpu_usage_percent,
            self.system_health.memory_usage_percent,
            self.system_health.disk_usage_percent
        );
        info!("║ Network Latency: {:.1}ms", self.system_health.network_latency_ms);
        info!("╠════════════════════════════════════════════╣");
        info!("║ Top Operations:");
        for op in &self.top_operations {
            info!("║ • {}: {:.2}ms avg ({})", op.operation_name, op.avg_latency_ms, op.count);
        }
        info!("╠════════════════════════════════════════════╣");
        info!("║ Cache Hit Rates:");
        for (cache, rate) in &self.cache_summary {
            info!("║ • {}: {:.1}%", cache, rate);
        }
        info!("╠════════════════════════════════════════════╣");
        info!("║ Active Alerts: {}", self.active_alerts.len());
        info!("╚════════════════════════════════════════════╝");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_health_creation() {
        let health = SystemHealth::new();
        assert!(health.is_healthy);
    }

    #[test]
    fn test_performance_metrics_creation() {
        let metrics = PerformanceMetrics::new("test_op".to_string());
        assert_eq!(metrics.operation_name, "test_op");
        assert_eq!(metrics.count, 0);
    }

    #[test]
    fn test_performance_metrics_calculation() {
        let mut metrics = PerformanceMetrics::new("test".to_string());
        let latencies = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        metrics.calculate_from_samples(&latencies, 0);

        assert_eq!(metrics.count, 5);
        assert_eq!(metrics.min_latency_ms, 10.0);
        assert_eq!(metrics.max_latency_ms, 50.0);
        assert_eq!(metrics.success_rate, 100.0);
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new("L1".to_string());
        stats.total_requests = 100;
        stats.cache_hits = 75;
        stats.update_hit_rate();

        assert_eq!(stats.hit_rate, 75.0);
    }

    #[test]
    fn test_tenant_metrics() {
        let metrics = TenantMetrics::new("tenant-1".to_string());
        assert_eq!(metrics.tenant_id, "tenant-1");
        assert_eq!(metrics.error_rate(), 0.0);
    }

    #[test]
    fn test_monitoring_dashboard_creation() {
        let dashboard = MonitoringDashboard::new();
        let health = dashboard.get_system_health();
        assert!(health.is_healthy);
    }

    #[test]
    fn test_dashboard_record_operation() {
        let dashboard = MonitoringDashboard::new();
        dashboard.record_operation("test_op".to_string(), 10.5, true);
        
        let metrics = dashboard.get_performance_metrics("test_op");
        assert!(metrics.is_some());
        assert_eq!(metrics.unwrap().count, 1);
    }

    #[test]
    fn test_dashboard_cache_stats() {
        let dashboard = MonitoringDashboard::new();
        dashboard.record_cache_hit("L1");
        dashboard.record_cache_hit("L1");
        dashboard.record_cache_miss("L1");

        let stats = dashboard.get_cache_stats("L1");
        assert!(stats.is_some());
        let hit_rate = stats.unwrap().hit_rate;
        assert!((hit_rate - 66.666).abs() < 0.01); // Allow for floating point precision
    }

    #[test]
    fn test_alert_creation() {
        let dashboard = MonitoringDashboard::new();
        let alert = Alert {
            id: "alert-1".to_string(),
            name: "CPU Alert".to_string(),
            metric: "cpu".to_string(),
            threshold: 80.0,
            comparison: AlertComparison::GreaterThan,
            duration_secs: 300,
            is_active: true,
        };

        dashboard.add_alert(alert);
        let alerts = dashboard.get_active_alerts();
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_dashboard_report() {
        let dashboard = MonitoringDashboard::new();
        dashboard.record_operation("op1".to_string(), 5.0, true);
        dashboard.record_cache_hit("L1");

        let report = dashboard.generate_report();
        assert!(!report.top_operations.is_empty());
        assert!(!report.cache_summary.is_empty());
    }
}
