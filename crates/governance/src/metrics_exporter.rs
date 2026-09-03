// crates/governance/src/metrics_exporter.rs
//! Universal Telemetry & Observability Exporter.
//!
//! Aggregates real-time execution vitals across all subsystems:
//! - Thermodynamic free-energy dissipation (\Delta F)
//! - Microsecond cycle latencies and JIT execution jitter
//! - Memory allocations and heap pressure
//! - Hardware thermal states
//!
//! Provides a standardized, sink-agnostic export interface for:
//! - The Native Desktop Studio HUD
//! - OpenTelemetry / Prometheus text-format scrapes
//! - Headless logging and in-game overlays

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A standardized metric observation point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricObservation {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp_us: u64,
    pub labels: HashMap<String, String>,
}

/// Sink trait for receiving telemetry observations
pub trait UniversalMetricsSink: Send + Sync {
    fn record(&self, observation: MetricObservation);
    fn flush(&self) {}
}

/// Universal In-Memory Ring Buffer Sink (for Desktop HUD & Overlays)
#[derive(Clone, Default)]
pub struct InMemoryMetricsSink {
    history: Arc<Mutex<Vec<MetricObservation>>>,
    max_capacity: usize,
}

impl InMemoryMetricsSink {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
            max_capacity: capacity,
        }
    }

    pub fn snapshot(&self) -> Vec<MetricObservation> {
        self.history.lock().map(|h| h.clone()).unwrap_or_default()
    }
}

impl UniversalMetricsSink for InMemoryMetricsSink {
    fn record(&self, observation: MetricObservation) {
        if let Ok(mut h) = self.history.lock() {
            if h.len() >= self.max_capacity {
                h.remove(0);
            }
            h.push(observation);
        }
    }
}

/// The Universal Metrics Exporter Registry
pub struct UniversalMetricsExporter {
    sinks: Vec<Box<dyn UniversalMetricsSink>>,
}

impl Default for UniversalMetricsExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl UniversalMetricsExporter {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn register_sink(&mut self, sink: Box<dyn UniversalMetricsSink>) {
        self.sinks.push(sink);
    }

    pub fn observe(
        &self,
        name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        timestamp_us: u64,
    ) {
        let obs = MetricObservation {
            metric_name: name.into(),
            value,
            unit: unit.into(),
            timestamp_us,
            labels: HashMap::new(),
        };

        for sink in &self.sinks {
            sink.record(obs.clone());
        }
    }

    /// Formats all current observations into standard Prometheus exposition format
    pub fn format_prometheus(observations: &[MetricObservation]) -> String {
        let mut out = String::new();
        for obs in observations {
            out.push_str(&format!(
                "# TYPE {} gauge\n{} {} {}\n",
                obs.metric_name, obs.metric_name, obs.value, obs.timestamp_us / 1000
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_exporter_and_prometheus_format() {
        let sink = Box::new(InMemoryMetricsSink::new(10));
        let sink_ref = sink.clone();

        let mut exporter = UniversalMetricsExporter::new();
        exporter.register_sink(sink);

        exporter.observe("thermodynamic_free_energy", 0.014, "dimensionless", 5000);
        exporter.observe("cycle_latency_us", 16.0, "microseconds", 5000);

        let snap = sink_ref.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].metric_name, "thermodynamic_free_energy");

        let prom = UniversalMetricsExporter::format_prometheus(&snap);
        assert!(prom.contains("thermodynamic_free_energy 0.014"));
        assert!(prom.contains("cycle_latency_us 16"));
    }
}
