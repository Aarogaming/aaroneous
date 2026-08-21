//! probing.rs
//! Backend process probing, system event interception, and high-frequency datalogging.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::debug;

use crate::traits::ProbingTrace;

/// High-frequency datalogger ring buffer for backend process probing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessProbeLogger {
    pub max_capacity: usize,
    pub traces: VecDeque<ProbingTrace>,
    pub target_processes: Vec<String>,
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
        }
    }

    /// Add a target process filter for backend probing
    pub fn add_target_process(&mut self, process_name: &str) {
        if !self.target_processes.contains(&process_name.to_string()) {
            self.target_processes.push(process_name.to_string());
        }
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
}
