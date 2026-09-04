//! probing.rs
//! Backend process probing, system event interception, and high-frequency datalogging.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tracing::debug;

use crate::traits::ProbingTrace;

/// High-frequency datalogger ring buffer for backend process probing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessProbeLogger {
    pub max_capacity: usize,
    pub traces: VecDeque<ProbingTrace>,
    pub target_processes: Vec<String>,
    #[serde(skip)]
    async_tx: Option<tokio::sync::mpsc::UnboundedSender<ProbingTrace>>,
}

impl Default for ProcessProbeLogger {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl ProcessProbeLogger {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            max_capacity,
            traces: VecDeque::with_capacity(max_capacity),
            target_processes: Vec::new(),
            async_tx: None,
        }
    }

    /// Initializes a non-blocking asynchronous multiplexed MPSC channel for thread-safe event publishing
    pub fn create_async_channel(&mut self) -> tokio::sync::mpsc::UnboundedReceiver<ProbingTrace> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.async_tx = Some(tx);
        rx
    }

    /// Non-blocking dispatch to async channel if initialized, or synchronous record fallback
    pub fn emit_trace(&mut self, trace: ProbingTrace) -> Result<()> {
        if let Some(ref tx) = self.async_tx {
            let _ = tx.send(trace.clone());
        }
        self.record_trace(trace)
    }

    /// Add a target process filter for backend probing
    pub fn add_target_process(&mut self, process_name: &str) {
        if !self.target_processes.contains(&process_name.to_string()) {
            self.target_processes.push(process_name.to_string());
        }
    }

    /// Check whether a process name is currently monitored
    pub fn is_monitored_process(&self, process_name: &str) -> bool {
        self.target_processes.iter().any(|p| p == process_name)
    }

    /// Record a probe event trace into the bounded ring buffer
    pub fn record_trace(&mut self, trace: ProbingTrace) -> Result<()> {
        if self.traces.len() >= self.max_capacity {
            self.traces.pop_front();
        }
        debug!(
            target: "marionette::probing",
            process = %trace.target_process,
            event = %trace.event_type,
            "Captured backend execution trace"
        );
        self.traces.push_back(trace);
        Ok(())
    }

    /// Retrieve the most recent N traces for a specific process
    pub fn get_recent_traces(&self, process_name: Option<&str>, count: usize) -> Vec<ProbingTrace> {
        self.traces
            .iter()
            .rev()
            .filter(|t| {
                if let Some(name) = process_name {
                    t.target_process == name
                } else {
                    true
                }
            })
            .take(count)
            .cloned()
            .collect()
    }

    /// Retrieve the most recent N traces filtered by event type
    pub fn get_traces_by_event_type(&self, event_type: &str, count: usize) -> Vec<ProbingTrace> {
        self.traces
            .iter()
            .rev()
            .filter(|t| t.event_type == event_type)
            .take(count)
            .cloned()
            .collect()
    }

    /// Retrieve all traces recorded within a microsecond timestamp window [start_us, end_us]
    pub fn get_traces_in_window(&self, start_us: u64, end_us: u64) -> Vec<ProbingTrace> {
        self.traces
            .iter()
            .filter(|t| t.timestamp_us >= start_us && t.timestamp_us <= end_us)
            .cloned()
            .collect()
    }

    /// Generates frequency distribution counts across observed event types
    pub fn event_distribution(&self) -> HashMap<String, usize> {
        let mut dist = HashMap::new();
        for t in &self.traces {
            *dist.entry(t.event_type.clone()).or_insert(0) += 1;
        }
        dist
    }

    /// Exports recent traces as lightweight tuples suitable for direct ingestion into `si_ir` DAG synthesis
    pub fn export_trace_tuples(&self, count: usize) -> Vec<(String, String, String, u64)> {
        self.traces
            .iter()
            .rev()
            .take(count)
            .rev()
            .map(|t| (t.target_process.clone(), t.event_type.clone(), t.payload.clone(), t.timestamp_us))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_probing_logger_capacity() {
        let mut logger = ProcessProbeLogger::new(3);
        for i in 0..5 {
            logger.record_trace(ProbingTrace {
                target_process: "target.exe".to_string(),
                event_type: "mem_read".to_string(),
                payload: format!("payload_{}", i),
                timestamp_us: i as u64,
            }).unwrap();
        }

        assert_eq!(logger.traces.len(), 3);
        let recent = logger.get_recent_traces(Some("target.exe"), 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].payload, "payload_4");
    }

    #[test]
    fn test_probing_logger_event_filter_and_window() {
        let mut logger = ProcessProbeLogger::new(10);
        logger.add_target_process("game.exe");
        assert!(logger.is_monitored_process("game.exe"));
        assert!(!logger.is_monitored_process("calc.exe"));

        logger.record_trace(ProbingTrace {
            target_process: "game.exe".to_string(),
            event_type: "render_frame".to_string(),
            payload: "frame_1".to_string(),
            timestamp_us: 1000,
        }).unwrap();

        logger.record_trace(ProbingTrace {
            target_process: "game.exe".to_string(),
            event_type: "input_poll".to_string(),
            payload: "mouse".to_string(),
            timestamp_us: 2000,
        }).unwrap();

        logger.record_trace(ProbingTrace {
            target_process: "game.exe".to_string(),
            event_type: "render_frame".to_string(),
            payload: "frame_2".to_string(),
            timestamp_us: 3000,
        }).unwrap();

        let render_events = logger.get_traces_by_event_type("render_frame", 10);
        assert_eq!(render_events.len(), 2);

        let window = logger.get_traces_in_window(1500, 3500);
        assert_eq!(window.len(), 2);

        let dist = logger.event_distribution();
        assert_eq!(dist.get("render_frame"), Some(&2));
        assert_eq!(dist.get("input_poll"), Some(&1));
    }

    #[tokio::test]
    async fn test_async_probing_channel() {
        let mut logger = ProcessProbeLogger::new(10);
        let mut rx = logger.create_async_channel();

        let trace = ProbingTrace {
            target_process: "worker.exe".to_string(),
            event_type: "syscall".to_string(),
            payload: "nt_query".to_string(),
            timestamp_us: 42,
        };

        logger.emit_trace(trace.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.target_process, "worker.exe");
        assert_eq!(received.timestamp_us, 42);
        assert_eq!(logger.traces.len(), 1);

        let tuples = logger.export_trace_tuples(1);
        assert_eq!(tuples.len(), 1);
        assert_eq!(tuples[0].0, "worker.exe");
        assert_eq!(tuples[0].1, "syscall");
        assert_eq!(tuples[0].2, "nt_query");
        assert_eq!(tuples[0].3, 42);
    }
}
