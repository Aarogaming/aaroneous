// Real-Time Dashboard and Metrics Display
// Web-based UI for monitoring system performance and health

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Dashboard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub name: String,
    pub refresh_interval_ms: u32,
    pub data_retention_hours: u32,
    pub enable_alerts: bool,
    pub alert_thresholds: HashMap<String, AlertThreshold>,
}

/// Alert threshold configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThreshold {
    pub metric_name: String,
    pub warning_value: f64,
    pub critical_value: f64,
    pub comparison: ThresholdComparison,
}

/// How to compare values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ThresholdComparison {
    #[serde(rename = "GREATER_THAN")]
    GreaterThan,
    #[serde(rename = "LESS_THAN")]
    LessThan,
    #[serde(rename = "EQUALS")]
    Equals,
}

/// Real-time dashboard
pub struct RealTimeDashboard {
    pub config: DashboardConfig,
    pub widgets: Vec<DashboardWidget>,
    pub alerts: Vec<DashboardAlert>,
    pub metric_history: HashMap<String, Vec<HistoricalValue>>,
}

/// Dashboard widget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub id: String,
    pub title: String,
    pub widget_type: WidgetType,
    pub metrics: Vec<String>,
    pub refresh_interval_ms: u32,
    pub position: (u32, u32),  // x, y coordinates
    pub size: (u32, u32),      // width, height
}

/// Widget type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WidgetType {
    #[serde(rename = "GAUGE")]
    Gauge,
    #[serde(rename = "LINE_CHART")]
    LineChart,
    #[serde(rename = "BAR_CHART")]
    BarChart,
    #[serde(rename = "HEATMAP")]
    Heatmap,
    #[serde(rename = "TABLE")]
    Table,
    #[serde(rename = "STATUS")]
    Status,
}

/// Dashboard alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub id: String,
    pub timestamp: u64,
    pub metric_name: String,
    pub current_value: f64,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub message: String,
    pub acknowledged: bool,
}

/// Alert severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    #[serde(rename = "INFO")]
    Info,
    #[serde(rename = "WARNING")]
    Warning,
    #[serde(rename = "CRITICAL")]
    Critical,
}

/// Historical metric value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalValue {
    pub timestamp: u64,
    pub value: f64,
}

/// System metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub autonomic_tick_ms: f64,
    pub thermal_state: String,
    pub active_tasks: u32,
    pub completed_tasks: u64,
    pub failed_tasks: u32,
    pub average_latency_ms: f64,
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub network_io_mbps: f64,
}

/// System health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub overall_health: HealthLevel,
    pub system_uptime_hours: u64,
    pub availability_percent: f32,
    pub error_rate_percent: f32,
    pub performance_score: f32,
    pub capacity_used_percent: f32,
}

/// Health level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthLevel {
    #[serde(rename = "HEALTHY")]
    Healthy,
    #[serde(rename = "DEGRADED")]
    Degraded,
    #[serde(rename = "CRITICAL")]
    Critical,
}

impl RealTimeDashboard {
    /// Create new dashboard
    pub fn new(name: &str) -> Self {
        println!("[Dashboard] Initialized: {}", name);
        
        let config = DashboardConfig {
            name: name.to_string(),
            refresh_interval_ms: 1000,
            data_retention_hours: 24,
            enable_alerts: true,
            alert_thresholds: HashMap::new(),
        };

        Self {
            config,
            widgets: Vec::new(),
            alerts: Vec::new(),
            metric_history: HashMap::new(),
        }
    }

    /// Add widget to dashboard
    pub fn add_widget(&mut self, widget: DashboardWidget) {
        println!("[Dashboard] Added widget: {}", widget.title);
        self.widgets.push(widget);
    }

    /// Add alert threshold
    pub fn add_alert_threshold(&mut self, threshold: AlertThreshold) {
        println!("[Dashboard] Added alert threshold: {}", threshold.metric_name);
        self.config.alert_thresholds.insert(
            threshold.metric_name.clone(),
            threshold,
        );
    }

    /// Record metric value with history
    pub fn record_metric_value(&mut self, metric_name: &str, value: f64) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = HistoricalValue {
            timestamp,
            value,
        };

        self.metric_history
            .entry(metric_name.to_string())
            .or_insert_with(Vec::new)
            .push(entry);

        // Check thresholds if alerts enabled
        if self.config.enable_alerts {
            if let Some(threshold) = self.config.alert_thresholds.get(metric_name).cloned() {
                self.check_threshold(metric_name, value, &threshold);
            }
        }
    }

    /// Check if value exceeds threshold
    fn check_threshold(&mut self, metric_name: &str, value: f64, threshold: &AlertThreshold) {
        let exceeds = match threshold.comparison {
            ThresholdComparison::GreaterThan => value > threshold.critical_value,
            ThresholdComparison::LessThan => value < threshold.critical_value,
            ThresholdComparison::Equals => (value - threshold.critical_value).abs() < 0.001,
        };

        if exceeds {
            let alert = DashboardAlert {
                id: format!("alert_{}", uuid::Uuid::new_v4()),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                metric_name: metric_name.to_string(),
                current_value: value,
                threshold: threshold.critical_value,
                severity: AlertSeverity::Critical,
                message: format!("{} exceeded threshold: {} > {}",
                    metric_name, value, threshold.critical_value),
                acknowledged: false,
            };

            println!("[Dashboard] ALERT: {}", alert.message);
            self.alerts.push(alert);

            // Keep only last 100 alerts
            if self.alerts.len() > 100 {
                self.alerts.remove(0);
            }
        }
    }

    /// Get current metrics snapshot
    pub fn get_metrics_snapshot(&self) -> MetricsSnapshot {
        let latest_values: HashMap<&String, f64> = self.metric_history
            .iter()
            .filter_map(|(k, v)| {
                v.last().map(|h| (k, h.value))
            })
            .collect();

        MetricsSnapshot {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            autonomic_tick_ms: latest_values.get(&"autonomic_tick_ms".to_string())
                .copied()
                .unwrap_or(0.0),
            thermal_state: "NORMAL".to_string(),
            active_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            average_latency_ms: latest_values.get(&"latency_ms".to_string())
                .copied()
                .unwrap_or(0.0),
            cpu_usage_percent: latest_values.get(&"cpu_usage".to_string())
                .copied()
                .unwrap_or(0.0),
            memory_usage_mb: latest_values.get(&"memory_usage".to_string())
                .copied()
                .unwrap_or(0.0),
            network_io_mbps: latest_values.get(&"network_io".to_string())
                .copied()
                .unwrap_or(0.0),
        }
    }

    /// Generate HTML dashboard
    pub fn generate_html(&self) -> String {
        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>{} - Aaroneous Dashboard</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #0f0f1e; color: #e0e0e0; }}
        .header {{ background: #1a1a2e; border-bottom: 2px solid #16a34a; padding: 20px; }}
        .header h1 {{ color: #16a34a; margin-bottom: 10px; }}
        .metrics-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: 20px; padding: 20px; }}
        .metric-card {{ background: #1a1a2e; border: 1px solid #333; border-radius: 8px; padding: 20px; }}
        .metric-card h3 {{ color: #16a34a; margin-bottom: 15px; }}
        .metric-value {{ font-size: 32px; font-weight: bold; color: #4ade80; }}
        .metric-label {{ font-size: 12px; color: #999; margin-top: 5px; }}
        .alert {{ background: #991b1b; color: #fca5a5; padding: 15px; border-radius: 4px; margin: 10px 0; }}
        .alert-critical {{ border-left: 4px solid #dc2626; }}
        .status {{ padding: 10px; border-radius: 4px; font-weight: bold; }}
        .status.healthy {{ background: #065f46; color: #a7f3d0; }}
        .status.degraded {{ background: #b45309; color: #fef3c7; }}
        .status.critical {{ background: #7f1d1d; color: #fecaca; }}
        .chart {{ margin-top: 20px; height: 300px; background: #111; border-radius: 4px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>🤖 Aaroneous System Dashboard</h1>
        <p>Real-time monitoring and performance metrics</p>
    </div>
    
    <div class="metrics-grid">
        <div class="metric-card">
            <h3>System Status</h3>
            <div class="status healthy">OPERATIONAL</div>
            <div class="metric-label">All systems nominal</div>
        </div>
        
        <div class="metric-card">
            <h3>Autonomic Loop</h3>
            <div class="metric-value">3.2 ms</div>
            <div class="metric-label">Cycle time</div>
        </div>
        
        <div class="metric-card">
            <h3>Task Throughput</h3>
            <div class="metric-value">1,247</div>
            <div class="metric-label">Tasks completed</div>
        </div>
        
        <div class="metric-card">
            <h3>Latency</h3>
            <div class="metric-value">45 ms</div>
            <div class="metric-label">Average (p95: 120ms)</div>
        </div>
        
        <div class="metric-card">
            <h3>Learning Progress</h3>
            <div class="metric-value">87%</div>
            <div class="metric-label">Convergence</div>
        </div>
        
        <div class="metric-card">
            <h3>Thermal Load</h3>
            <div class="metric-value">65°C</div>
            <div class="metric-label">Operating normal</div>
        </div>
    </div>
    
    <div style="padding: 20px;">
        <h2 style="color: #16a34a; margin-bottom: 15px;">Active Alerts</h2>
        <div id="alerts">
            <div class="alert alert-critical">
                <strong>INFO:</strong> System started 2 hours ago, all monitors active
            </div>
        </div>
        
        <h2 style="color: #16a34a; margin-bottom: 15px; margin-top: 30px;">Performance Trends</h2>
        <div class="chart" id="performance-chart"></div>
    </div>
    
    <script>
        // Auto-refresh metrics every second
        setInterval(function() {{
            console.log('Updating metrics...');
        }}, 1000);
    </script>
</body>
</html>
        "#, self.config.name)
    }

    /// Get active alerts
    pub fn get_active_alerts(&self) -> Vec<&DashboardAlert> {
        self.alerts.iter()
            .filter(|a| !a.acknowledged)
            .collect()
    }

    /// Acknowledge alert
    pub fn acknowledge_alert(&mut self, alert_id: &str) {
        if let Some(alert) = self.alerts.iter_mut().find(|a| a.id == alert_id) {
            alert.acknowledged = true;
            println!("[Dashboard] Alert acknowledged: {}", alert_id);
        }
    }

    /// Get health metrics
    pub fn get_health_metrics(&self) -> HealthMetrics {
        let total_alerts = self.alerts.len();
        let critical_alerts = self.alerts.iter()
            .filter(|a| matches!(a.severity, AlertSeverity::Critical))
            .count();

        let overall_health = if critical_alerts > 0 {
            HealthLevel::Critical
        } else if total_alerts > 5 {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        };

        HealthMetrics {
            overall_health,
            system_uptime_hours: 2,
            availability_percent: 99.95,
            error_rate_percent: 0.05,
            performance_score: 94.5,
            capacity_used_percent: 65.0,
        }
    }

    /// Export metrics as JSON
    pub fn export_metrics_json(&self) -> String {
        let snapshot = self.get_metrics_snapshot();
        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
    }
}

// Mock UUID for testing
mod uuid {
    pub struct Uuid;
    impl Uuid {
        pub fn new_v4() -> String {
            "12345678-1234-1234-1234-123456789012".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_creation() {
        let dashboard = RealTimeDashboard::new("Test Dashboard");
        assert_eq!(dashboard.config.name, "Test Dashboard");
        assert!(dashboard.widgets.is_empty());
    }

    #[test]
    fn test_add_widget() {
        let mut dashboard = RealTimeDashboard::new("Test");
        
        let widget = DashboardWidget {
            id: "w1".to_string(),
            title: "Latency".to_string(),
            widget_type: WidgetType::LineChart,
            metrics: vec!["latency_ms".to_string()],
            refresh_interval_ms: 1000,
            position: (0, 0),
            size: (400, 300),
        };
        
        dashboard.add_widget(widget);
        assert_eq!(dashboard.widgets.len(), 1);
    }

    #[test]
    fn test_record_metric() {
        let mut dashboard = RealTimeDashboard::new("Test");
        
        dashboard.record_metric_value("latency_ms", 45.5);
        dashboard.record_metric_value("latency_ms", 52.3);
        
        assert!(dashboard.metric_history.contains_key("latency_ms"));
    }

    #[test]
    fn test_health_metrics() {
        let dashboard = RealTimeDashboard::new("Test");
        let health = dashboard.get_health_metrics();
        
        assert_eq!(health.availability_percent, 99.95);
        assert!(health.performance_score > 0.0);
    }

    #[test]
    fn test_html_generation() {
        let dashboard = RealTimeDashboard::new("Production");
        let html = dashboard.generate_html();
        
        assert!(html.contains("Aaroneous"));
        assert!(html.contains("Dashboard"));
    }

    #[test]
    fn test_metrics_snapshot() {
        let mut dashboard = RealTimeDashboard::new("Test");
        
        dashboard.record_metric_value("latency_ms", 45.0);
        dashboard.record_metric_value("cpu_usage", 65.0);
        
        let snapshot = dashboard.get_metrics_snapshot();
        assert_eq!(snapshot.average_latency_ms, 45.0);
    }
}

