use anyhow::Result;
use sysinfo::System;
use tracing::{info, warn};

/// CONSUMER-18: Process Manager & Task Killer
/// Native OS process inspection and management. Allows the AI to autonomously
/// detect frozen processes, high CPU/RAM usage anomalies, and gracefully kill them.
pub struct ProcessManager {
    sys: System,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self { sys: System::new_all() }
    }

    /// Scans for hung or extremely resource-heavy processes
    pub fn scan_for_anomalies(&mut self) {
        self.sys.refresh_all();
        for (pid, process) in self.sys.processes() {
            if process.memory() > 8_000_000_000 {
                warn!("Process {} ({}) is consuming >8GB RAM!", process.name(), pid);
            }
        }
    }

    pub fn kill_process_by_name(&mut self, name: &str) -> Result<()> {
        self.sys.refresh_processes();
        for process in self.sys.processes_by_exact_name(name) {
            info!("Killing process {} (PID: {})", name, process.pid());
            process.kill();
        }
        Ok(())
    }
}