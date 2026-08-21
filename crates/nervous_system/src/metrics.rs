// Slab Allocator Metrics & Dashboard Integration
// Exposes runtime metrics for monitoring and dashboard visualization.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::slab_allocator::SlabStats;

/// Aggregate metrics snapshot for dashboard consumption
#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
    /// Slab allocator metrics by agent
    pub slab_metrics: HashMap<String, SlabMetricEntry>,
    /// Intent log metrics
    pub intent_log_entries: u64,
    /// Current synapse generation
    pub synapse_generation: u64,
    /// Total mutations applied
    pub total_mutations: u64,
    /// Total rejections
    pub total_rejections: u64,
    /// Uptime in seconds
    pub uptime_secs: u64,
}

/// Per-agent slab metric entry
#[derive(Debug, Clone, serde::Serialize)]
pub struct SlabMetricEntry {
    /// Slab utilization (0.0 - 1.0)
    pub utilization: f32,
    /// Number of free slots
    pub free_count: u16,
    /// Number of committed slots
    pub committed_count: u16,
    /// Current generation
    pub generation: u64,
    /// Total allocations since start
    pub total_allocations: u64,
    /// Total frees since start
    pub total_frees: u64,
}

/// Thread-safe metrics collector
pub struct MetricsCollector {
    /// Start time in nanoseconds
    start_time_ns: u64,
    /// Total mutations counter
    total_mutations: AtomicU64,
    /// Total rejections counter
    total_rejections: AtomicU64,
    /// Current synapse generation
    synapse_generation: AtomicU64,
    /// Intent log entry count
    intent_log_entries: AtomicU64,
    /// Per-agent slab stats (protected by mutex for simplicity)
    slab_stats: std::sync::Mutex<HashMap<String, SlabStatsWithCounters>>,
}

/// Extended slab stats with allocation counters
#[derive(Debug, Clone)]
struct SlabStatsWithCounters {
    pub stats: SlabStats,
    pub total_allocations: u64,
    pub total_frees: u64,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            start_time_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64,
            total_mutations: AtomicU64::new(0),
            total_rejections: AtomicU64::new(0),
            synapse_generation: AtomicU64::new(0),
            intent_log_entries: AtomicU64::new(0),
            slab_stats: std::sync::Mutex::new(HashMap::new()),
        }
    }

    /// Record a successful mutation
    pub fn record_mutation(&self) {
        self.total_mutations.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rejected intent
    pub fn record_rejection(&self) {
        self.total_rejections.fetch_add(1, Ordering::Relaxed);
    }

    /// Update synapse generation
    pub fn update_generation(&self, generation: u64) {
        self.synapse_generation.store(generation, Ordering::Relaxed);
    }

    /// Update intent log entry count
    pub fn update_intent_log_entries(&self, count: u64) {
        self.intent_log_entries.store(count, Ordering::Relaxed);
    }

    /// Update slab stats for an agent
    pub fn update_slab_stats(&self, agent_id: &str, stats: SlabStats) {
        let mut guard = self.slab_stats.lock().unwrap_or_else(|e| e.into_inner());
        let entry = guard
            .entry(agent_id.to_string())
            .or_insert(SlabStatsWithCounters {
                stats: SlabStats::default(),
                total_allocations: 0,
                total_frees: 0,
            });
        entry.stats = stats;
    }

    /// Record an allocation for an agent
    pub fn record_allocation(&self, agent_id: &str) {
        let mut guard = self.slab_stats.lock().unwrap_or_else(|e| e.into_inner());
        let entry = guard
            .entry(agent_id.to_string())
            .or_insert(SlabStatsWithCounters {
                stats: SlabStats::default(),
                total_allocations: 0,
                total_frees: 0,
            });
        entry.total_allocations += 1;
    }

    /// Record a free for an agent
    pub fn record_free(&self, agent_id: &str) {
        let mut guard = self.slab_stats.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = guard.get_mut(agent_id) {
            entry.total_frees += 1;
        }
    }

    /// Get a metrics snapshot for dashboard
    pub fn snapshot(&self) -> MetricsSnapshot {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let uptime_secs = (now - self.start_time_ns) / 1_000_000_000;

        let slab_metrics = {
            let guard = self.slab_stats.lock().unwrap_or_else(|e| e.into_inner());
            guard
                .iter()
                .map(|(id, entry)| {
                    (
                        id.clone(),
                        SlabMetricEntry {
                            utilization: entry.stats.utilization,
                            free_count: entry.stats.free_count,
                            committed_count: entry.stats.committed_count,
                            generation: entry.stats.current_generation,
                            total_allocations: entry.total_allocations,
                            total_frees: entry.total_frees,
                        },
                    )
                })
                .collect()
        };

        MetricsSnapshot {
            timestamp_ns: now,
            slab_metrics,
            intent_log_entries: self.intent_log_entries.load(Ordering::Relaxed),
            synapse_generation: self.synapse_generation.load(Ordering::Relaxed),
            total_mutations: self.total_mutations.load(Ordering::Relaxed),
            total_rejections: self.total_rejections.load(Ordering::Relaxed),
            uptime_secs,
        }
    }

    /// Export metrics as JSON for external dashboard
    pub fn to_json(&self) -> String {
        let snapshot = self.snapshot();
        serde_json::to_string_pretty(&snapshot)
            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared metrics collector for multi-threaded access
pub type SharedMetricsCollector = Arc<MetricsCollector>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_basic() {
        let collector = MetricsCollector::new();

        collector.record_mutation();
        collector.record_mutation();
        collector.record_rejection();

        collector.update_generation(5);
        collector.update_intent_log_entries(10);

        let snapshot = collector.snapshot();
        assert_eq!(snapshot.total_mutations, 2);
        assert_eq!(snapshot.total_rejections, 1);
        assert_eq!(snapshot.synapse_generation, 5);
        assert_eq!(snapshot.intent_log_entries, 10);
        // uptime_secs is u64, always >= 0
    }

    #[test]
    fn test_slab_metrics() {
        let collector = MetricsCollector::new();

        let stats = SlabStats {
            capacity: 16,
            free_count: 4,
            active_count: 8,
            committed_count: 12,
            error_count: 0,
            utilization: 0.75,
            total_allocations: 0,
            total_frees: 0,
            next_sequence: 0,
            current_generation: 3,
        };
        collector.update_slab_stats("agent_1", stats.clone());
        collector.record_allocation("agent_1");
        collector.record_allocation("agent_1");
        collector.record_free("agent_1");

        let snapshot = collector.snapshot();
        let agent_metrics = snapshot.slab_metrics.get("agent_1").unwrap();
        assert_eq!(agent_metrics.utilization, 0.75);
        assert_eq!(agent_metrics.total_allocations, 2);
        assert_eq!(agent_metrics.total_frees, 1);
    }

    #[test]
    fn test_json_export() {
        let collector = MetricsCollector::new();
        collector.record_mutation();
        collector.update_generation(1);

        let json = collector.to_json();
        assert!(json.contains("total_mutations"));
        assert!(json.contains("synapse_generation"));
    }
}
