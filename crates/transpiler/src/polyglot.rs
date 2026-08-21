//! crates/transpiler/src/polyglot.rs
//! Polyglot language detection, capsule binding, and self-healing circuit breaker.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolyglotCapsule {
    pub name: String,
    pub language: String,
    pub entry_point: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SelfHealingState {
    pub circuit_breaker_state: CircuitBreakerState,
    pub health_check_interval: Duration,
    pub recovery_threshold: u32,
    pub failure_threshold: u32,
    pub graceful_degradation_level: u32,
    pub predictive_failure_score: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// TelemetryBuffer stores telemetry data for observability
#[derive(Debug, Default, Clone)]
pub struct TelemetryBuffer {
    pub entries: Vec<TelemetryEntry>,
}

#[derive(Debug, Clone)]
pub struct TelemetryEntry {
    pub timestamp: std::time::SystemTime,
    pub level: TelemetryLevel,
    pub message: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelemetryLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl TelemetryBuffer {
    pub fn new() -> Self {
        TelemetryBuffer {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, level: TelemetryLevel, message: &str, source: &str) {
        self.entries.push(TelemetryEntry {
            timestamp: std::time::SystemTime::now(),
            level,
            message: message.to_string(),
            source: source.to_string(),
        });
    }

    pub fn register_language(&mut self, language: &str) {
        self.push(
            TelemetryLevel::Info,
            &format!("Registered language: {}", language),
            "foundry",
        );
    }
}

/// MetricsCollector collects and aggregates metrics
#[derive(Debug, Default, Clone)]
pub struct MetricsCollector {
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, Vec<f64>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        MetricsCollector {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    pub fn increment(&mut self, name: &str, delta: u64) {
        *self.counters.entry(name.to_string()).or_insert(0) += delta;
    }

    pub fn set_gauge(&mut self, name: &str, value: f64) {
        self.gauges.insert(name.to_string(), value);
    }

    pub fn record_histogram(&mut self, name: &str, value: f64) {
        self.histograms
            .entry(name.to_string())
            .or_default()
            .push(value);
    }

    pub fn register_language(&mut self, language: &str) {
        self.increment(&format!("languages_{}", language), 1);
    }
}

/// PolyglotFoundry is a universal code analyzer and language capsule router.
pub struct PolyglotFoundry {
    capsules: HashMap<String, PolyglotCapsule>,
    healing_state: Arc<Mutex<SelfHealingState>>,
    failure_tracker: Arc<AtomicU64>,
    recovery_counter: Arc<AtomicU32>,
    circuit_breaker: Arc<AtomicBool>,
    telemetry_buffer: Arc<Mutex<TelemetryBuffer>>,
    metrics_collector: Arc<Mutex<MetricsCollector>>,
}

impl Default for PolyglotFoundry {
    fn default() -> Self {
        Self::new()
    }
}

impl PolyglotFoundry {
    pub fn new() -> Self {
        let mut foundry = PolyglotFoundry {
            capsules: HashMap::new(),
            healing_state: Arc::new(Mutex::new(SelfHealingState {
                circuit_breaker_state: CircuitBreakerState::Closed,
                health_check_interval: Duration::from_secs(30),
                recovery_threshold: 3,
                failure_threshold: 5,
                graceful_degradation_level: 0,
                predictive_failure_score: 0.0,
            })),
            failure_tracker: Arc::new(AtomicU64::new(0)),
            recovery_counter: Arc::new(AtomicU32::new(0)),
            circuit_breaker: Arc::new(AtomicBool::new(false)),
            telemetry_buffer: Arc::new(Mutex::new(TelemetryBuffer::new())),
            metrics_collector: Arc::new(Mutex::new(MetricsCollector::new())),
        };

        foundry.register_rust_capsule();
        foundry.register_c_capsule();
        foundry.register_python_capsule();
        foundry.register_text_capsule();

        foundry
    }

    pub fn register_capsule(&mut self, capsule: PolyglotCapsule) {
        self.capsules.insert(capsule.name.clone(), capsule);
    }

    pub fn register_rust_capsule(&mut self) {
        self.capsules.insert(
            "rust".to_string(),
            PolyglotCapsule {
                name: "rust".to_string(),
                language: "rust".to_string(),
                entry_point: "main".to_string(),
                capabilities: vec![
                    "memory".to_string(),
                    "threads".to_string(),
                    "async".to_string(),
                ],
            },
        );
        self.telemetry_buffer
            .lock()
            .unwrap()
            .register_language("rust");
        self.metrics_collector
            .lock()
            .unwrap()
            .register_language("rust");
    }

    pub fn register_c_capsule(&mut self) {
        self.capsules.insert(
            "c".to_string(),
            PolyglotCapsule {
                name: "c".to_string(),
                language: "c".to_string(),
                entry_point: "main".to_string(),
                capabilities: vec![
                    "memory".to_string(),
                    "threads".to_string(),
                    "async".to_string(),
                ],
            },
        );
        self.telemetry_buffer.lock().unwrap().register_language("c");
        self.metrics_collector
            .lock()
            .unwrap()
            .register_language("c");
    }

    pub fn register_python_capsule(&mut self) {
        self.capsules.insert(
            "python".to_string(),
            PolyglotCapsule {
                name: "python".to_string(),
                language: "python".to_string(),
                entry_point: "main".to_string(),
                capabilities: vec![
                    "memory".to_string(),
                    "threads".to_string(),
                    "async".to_string(),
                ],
            },
        );
        self.telemetry_buffer
            .lock()
            .unwrap()
            .register_language("python");
        self.metrics_collector
            .lock()
            .unwrap()
            .register_language("python");
    }

    pub fn register_text_capsule(&mut self) {
        self.capsules.insert(
            "text".to_string(),
            PolyglotCapsule {
                name: "text".to_string(),
                language: "text".to_string(),
                entry_point: "parse".to_string(),
                capabilities: vec![
                    "llm".to_string(),
                    "transpilation".to_string(),
                    "reflection".to_string(),
                ],
            },
        );
        self.telemetry_buffer
            .lock()
            .unwrap()
            .register_language("text");
        self.metrics_collector
            .lock()
            .unwrap()
            .register_language("text");
    }

    pub fn detect_language(&self, input: &str) -> String {
        let trimmed = input.trim_start();
        if trimmed.starts_with("#include") || trimmed.starts_with("#pragma") {
            "c".to_string()
        } else if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") || trimmed.starts_with("#[") {
            "rust".to_string()
        } else if trimmed.starts_with("def ") || trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            "python".to_string()
        } else if trimmed.starts_with("function ") || trimmed.starts_with("const ") || trimmed.starts_with("let ") {
            "javascript".to_string()
        } else {
            "text".to_string()
        }
    }

    pub fn get_capsule(&self, name: &str) -> Option<&PolyglotCapsule> {
        self.capsules.get(name)
    }

    pub fn list_capsules(&self) -> Vec<&PolyglotCapsule> {
        self.capsules.values().collect()
    }

    pub fn record_failure(&self) {
        self.failure_tracker.fetch_add(1, Ordering::SeqCst);
        let mut state = self.healing_state.lock().unwrap();
        let current_failures = self.failure_tracker.load(Ordering::SeqCst);

        if current_failures >= state.failure_threshold as u64 {
            self.circuit_breaker.store(false, Ordering::SeqCst);
            state.circuit_breaker_state = CircuitBreakerState::Open;
            state.graceful_degradation_level = state.graceful_degradation_level.saturating_add(1);
            state.predictive_failure_score = (state.predictive_failure_score * 0.7) + (0.9 * 0.3);
        }
    }

    pub fn record_success(&self) {
        self.failure_tracker.store(0, Ordering::SeqCst);
        self.recovery_counter.fetch_add(1, Ordering::SeqCst);
        let mut state = self.healing_state.lock().unwrap();
        let recovery_count = self.recovery_counter.load(Ordering::SeqCst);

        if recovery_count >= state.recovery_threshold {
            self.circuit_breaker.store(true, Ordering::SeqCst);
            state.circuit_breaker_state = CircuitBreakerState::Closed;
            state.graceful_degradation_level = 0;
            state.predictive_failure_score = 0.0;
        }
    }

    pub fn is_circuit_open(&self) -> bool {
        self.healing_state.lock().unwrap().circuit_breaker_state == CircuitBreakerState::Open
    }

    pub fn is_circuit_closed(&self) -> bool {
        self.healing_state.lock().unwrap().circuit_breaker_state == CircuitBreakerState::Closed
    }

    pub fn get_health_state(&self) -> SelfHealingState {
        self.healing_state.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polyglot_language_detection() {
        let foundry = PolyglotFoundry::new();

        assert_eq!(foundry.detect_language("pub fn execute() {}"), "rust");
        assert_eq!(foundry.detect_language("#include <stdio.h>"), "c");
        assert_eq!(foundry.detect_language("def run_task(): pass"), "python");
        assert_eq!(foundry.detect_language("const x = 42;"), "javascript");
        assert_eq!(foundry.detect_language("plain text prompt"), "text");
    }

    #[test]
    fn test_circuit_breaker_transition() {
        let foundry = PolyglotFoundry::new();
        assert!(foundry.is_circuit_closed());

        // Trip the circuit breaker
        for _ in 0..5 {
            foundry.record_failure();
        }

        assert!(foundry.is_circuit_open());

        // Recover
        for _ in 0..3 {
            foundry.record_success();
        }

        assert!(foundry.is_circuit_closed());
    }
}
