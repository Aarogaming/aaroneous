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
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// A standardized metric observation point
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricObservation {
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp_us: u64,
    pub labels: HashMap<String, String>,
}

/// Statistical summary of a specific named metric over recorded observations
#[derive(Debug, Clone, PartialEq)]
pub struct MetricSummary {
    pub metric_name: String,
    pub count: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub latest: f64,
}

/// Sink trait for receiving telemetry observations
pub trait UniversalMetricsSink: Send + Sync {
    fn record(&self, observation: MetricObservation);
    fn flush(&self) {}
}

/// Universal In-Memory Ring Buffer Sink (for Desktop HUD & Overlays) with O(1) eviction
#[derive(Clone)]
pub struct InMemoryMetricsSink {
    history: Arc<Mutex<VecDeque<MetricObservation>>>,
    max_capacity: usize,
}

impl Default for InMemoryMetricsSink {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl InMemoryMetricsSink {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            max_capacity: capacity.max(1),
        }
    }

    pub fn snapshot(&self) -> Vec<MetricObservation> {
        self.history
            .lock()
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn count(&self) -> usize {
        self.history.lock().map(|h| h.len()).unwrap_or(0)
    }

    pub fn clear(&self) {
        if let Ok(mut h) = self.history.lock() {
            h.clear();
        }
    }
}

impl UniversalMetricsSink for InMemoryMetricsSink {
    fn record(&self, observation: MetricObservation) {
        if let Ok(mut h) = self.history.lock() {
            if h.len() >= self.max_capacity {
                h.pop_front();
            }
            h.push_back(observation);
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
        self.observe_with_labels(name, value, unit, timestamp_us, &[]);
    }

    pub fn observe_with_labels(
        &self,
        name: impl Into<String>,
        value: f64,
        unit: impl Into<String>,
        timestamp_us: u64,
        labels: &[(&str, &str)],
    ) {
        let mut label_map = HashMap::with_capacity(labels.len());
        for &(k, v) in labels {
            label_map.insert(k.to_string(), v.to_string());
        }

        let obs = MetricObservation {
            metric_name: name.into(),
            value,
            unit: unit.into(),
            timestamp_us,
            labels: label_map,
        };

        for sink in &self.sinks {
            sink.record(obs.clone());
        }
    }

    /// Directly ingests state from `homeostasis::DynamicEquilibriumState` into standard metrics
    pub fn observe_homeostasis(&self, state: &crate::homeostasis::DynamicEquilibriumState, timestamp_us: u64) {
        self.observe("homeostasis.energy_reserve", state.global_energy_reserve as f64, "tokens", timestamp_us);
        self.observe("homeostasis.cognitive_load", state.active_cognitive_load as f64, "load", timestamp_us);
        self.observe("homeostasis.memory_pressure_mb", state.memory_pressure_mb as f64, "MB", timestamp_us);
        self.observe("homeostasis.throttle_factor", state.throttle_factor as f64, "factor", timestamp_us);
        self.observe("homeostasis.degradation_tier", state.degradation_tier as u8 as f64, "tier", timestamp_us);
    }

    /// Computes summary statistics (min, max, mean, count) for a named metric
    pub fn summarize_metric(observations: &[MetricObservation], name: &str) -> Option<MetricSummary> {
        let matching: Vec<&MetricObservation> = observations
            .iter()
            .filter(|o| o.metric_name == name)
            .collect();

        if matching.is_empty() {
            return None;
        }

        let count = matching.len();
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut sum = 0.0;
        let latest = matching.last().map(|o| o.value).unwrap_or(0.0);

        for obs in &matching {
            if obs.value < min {
                min = obs.value;
            }
            if obs.value > max {
                max = obs.value;
            }
            sum += obs.value;
        }

        Some(MetricSummary {
            metric_name: name.to_string(),
            count,
            min,
            max,
            mean: sum / count as f64,
            latest,
        })
    }

    /// Formats all current observations into standard Prometheus exposition format with label support
    pub fn format_prometheus(observations: &[MetricObservation]) -> String {
        let mut out = String::new();
        for obs in observations {
            if obs.labels.is_empty() {
                out.push_str(&format!(
                    "# TYPE {name} gauge\n{name} {val} {ts}\n",
                    name = obs.metric_name,
                    val = obs.value,
                    ts = obs.timestamp_us / 1000
                ));
            } else {
                let mut sorted_labels: Vec<_> = obs.labels.iter().collect();
                sorted_labels.sort_by_key(|&(k, _)| k);
                let label_str = sorted_labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(",");

                out.push_str(&format!(
                    "# TYPE {name} gauge\n{name}{{{labels}}} {val} {ts}\n",
                    name = obs.metric_name,
                    labels = label_str,
                    val = obs.value,
                    ts = obs.timestamp_us / 1000
                ));
            }
        }
        out
    }

    /// Formats all current observations into standard OpenTelemetry (OTLP) JSON metrics structure
    pub fn format_otlp_json(observations: &[MetricObservation]) -> String {
        let mut metrics = Vec::new();
        for obs in observations {
            let mut attrs = Vec::new();
            for (k, v) in &obs.labels {
                attrs.push(format!("{{\"key\":\"{}\",\"value\":{{\"stringValue\":\"{}\"}}}}", k, v));
            }
            let attr_str = attrs.join(",");

            metrics.push(format!(
                "{{\"name\":\"{}\",\"unit\":\"{}\",\"gauge\":{{\"dataPoints\":[{{\"timeUnixNano\":{},\"asDouble\":{},\"attributes\":[{}]}}]}}}}",
                obs.metric_name, obs.unit, obs.timestamp_us * 1000, obs.value, attr_str
            ));
        }

        format!(
            "{{\"resourceMetrics\":[{{\"scopeMetrics\":[{{\"metrics\":[{}]}}]}}]}}",
            metrics.join(",")
        )
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
        exporter.observe_with_labels(
            "cycle_latency_us",
            16.0,
            "microseconds",
            5000,
            &[("subsystem", "jit")],
        );

        let snap = sink_ref.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].metric_name, "thermodynamic_free_energy");

        let summary = UniversalMetricsExporter::summarize_metric(&snap, "cycle_latency_us").unwrap();
        assert_eq!(summary.count, 1);
        assert_eq!(summary.mean, 16.0);

        let prom = UniversalMetricsExporter::format_prometheus(&snap);
        assert!(prom.contains("thermodynamic_free_energy 0.014"));
        assert!(prom.contains("cycle_latency_us{subsystem=\"jit\"} 16"));

        let otlp = UniversalMetricsExporter::format_otlp_json(&snap);
        assert!(otlp.contains("thermodynamic_free_energy"));
        assert!(otlp.contains("resourceMetrics"));
    }

    #[test]
    fn test_ring_buffer_capacity_eviction() {
        let sink = InMemoryMetricsSink::new(3);
        let obs = |val: f64| MetricObservation {
            metric_name: "test".to_string(),
            value: val,
            unit: "u".to_string(),
            timestamp_us: 100,
            labels: HashMap::new(),
        };

        sink.record(obs(1.0));
        sink.record(obs(2.0));
        sink.record(obs(3.0));
        sink.record(obs(4.0)); // Should evict 1.0

        let snap = sink.snapshot();
        assert_eq!(snap.len(), 3);
        assert_eq!(snap[0].value, 2.0);
        assert_eq!(snap[2].value, 4.0);
    }

    #[test]
    fn test_observe_homeostasis() {
        let sink = Box::new(InMemoryMetricsSink::new(10));
        let mut exporter = UniversalMetricsExporter::new();
        let sink_ref = sink.clone();
        exporter.register_sink(sink);

        let regulator = crate::homeostasis::FeedbackRegulator::default();
        exporter.observe_homeostasis(regulator.state(), 1000);

        let snap = sink_ref.snapshot();
        assert_eq!(snap.len(), 5);
        assert!(snap.iter().any(|o| o.metric_name == "homeostasis.energy_reserve"));
        assert!(snap.iter().any(|o| o.metric_name == "homeostasis.throttle_factor"));
    }
}
