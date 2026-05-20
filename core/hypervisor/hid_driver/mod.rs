/// HID Driver Module: OS-level input control (Mouse, Keyboard, Scroll)
/// 
/// This module provides high-performance, low-latency input simulation
/// targeting <1ms latency for marionette control.
///
/// On Windows: Uses Windows API (SetCursorPos, mouse_event, keybd_event)
/// On Linux: Uses uinput or libevdev (future implementation)

pub mod platform;
pub mod metrics;
pub mod commands;

pub use platform::HidPlatform;
pub use metrics::{HidMetrics, LatencyPercentiles};
pub use commands::{HidCommand, HidResponse, MouseButton};

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::Instant;
use std::collections::VecDeque;

/// Main HID Driver interface
#[derive(Clone)]
pub struct HidDriver {
    /// Platform-specific implementation
    platform: Arc<dyn HidPlatform>,
    
    /// Performance metrics
    metrics: Arc<RwLock<HidMetrics>>,
    
    /// Command history for debugging
    history: Arc<RwLock<VecDeque<CommandRecord>>>,
    
    /// Last command timestamp (microseconds)
    last_command_us: Arc<AtomicU64>,
    
    /// Total commands executed
    total_commands: Arc<AtomicU32>,
}

/// Record of a single command execution
#[derive(Clone, Debug)]
pub struct CommandRecord {
    pub command: String,
    pub latency_us: u32,
    pub status: String,
    pub timestamp_ms: u64,
}

impl HidDriver {
    /// Create new HID driver
    pub async fn new() -> Result<Self, String> {
        let platform = platform::create_platform_backend()?;
        
        Ok(Self {
            platform,
            metrics: Arc::new(RwLock::new(HidMetrics::default())),
            history: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            last_command_us: Arc::new(AtomicU64::new(0)),
            total_commands: Arc::new(AtomicU32::new(0)),
        })
    }
    
    /// Execute HID command with latency tracking
    pub async fn execute(&self, cmd: HidCommand) -> Result<HidResponse, String> {
        let start = Instant::now();
        
        // Execute the command
        let response = self.platform.execute_command(&cmd).await?;
        
        // Measure latency
        let latency_us = start.elapsed().as_micros() as u32;
        
        // Update metrics
        self.update_metrics(latency_us);
        
        // Record in history
        self.record_command(&cmd, latency_us, &response);
        
        // Update last command timestamp
        self.last_command_us.store(latency_us as u64, Ordering::Release);
        
        // Increment counter
        self.total_commands.fetch_add(1, Ordering::Relaxed);
        
        Ok(response)
    }
    
    /// Update performance metrics
    fn update_metrics(&self, latency_us: u32) {
        let mut metrics = self.metrics.write();
        
        metrics.total_commands += 1;
        metrics.sum_latency_us += latency_us as u64;
        metrics.min_latency_us = metrics.min_latency_us.min(latency_us);
        metrics.max_latency_us = metrics.max_latency_us.max(latency_us);
        
        metrics.latencies.push_back(latency_us);
        if metrics.latencies.len() > 10000 {
            metrics.latencies.pop_front();
        }
    }
    
    /// Record command in history
    fn record_command(&self, cmd: &HidCommand, latency_us: u32, response: &HidResponse) {
        let mut history = self.history.write();
        
        let record = CommandRecord {
            command: format!("{:?}", cmd),
            latency_us,
            status: format!("{:?}", response),
            timestamp_ms: now_ms(),
        };
        
        history.push_back(record);
        if history.len() > 1000 {
            history.pop_front();
        }
    }
    
    /// Get current metrics
    pub fn metrics(&self) -> HidMetrics {
        self.metrics.read().clone()
    }
    
    /// Get latency percentiles
    pub fn latency_percentiles(&self) -> Result<LatencyPercentiles, String> {
        let metrics = self.metrics.read();
        
        if metrics.latencies.is_empty() {
            return Err("No latency data yet".to_string());
        }
        
        let mut sorted: Vec<u32> = metrics.latencies.iter().copied().collect();
        sorted.sort();
        
        let len = sorted.len();
        let p50 = sorted[len / 2];
        let p95 = sorted[(len * 95) / 100];
        let p99 = sorted[(len * 99) / 100];
        
        Ok(LatencyPercentiles { p50, p95, p99 })
    }
    
    /// Clear history
    pub fn clear_history(&self) {
        self.history.write().clear();
    }
    
    /// Get command history
    pub fn history(&self) -> Vec<CommandRecord> {
        self.history.read().iter().cloned().collect()
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_hid_driver_creation() {
        let driver = HidDriver::new().await;
        assert!(driver.is_ok(), "Failed to create HID driver");
    }
    
    #[tokio::test]
    async fn test_mouse_move_command() {
        let driver = HidDriver::new().await.unwrap();
        
        let cmd = HidCommand::MouseMove { x: 100, y: 100 };
        let response = driver.execute(cmd).await;
        
        assert!(response.is_ok(), "MouseMove command failed");
        
        let metrics = driver.metrics();
        assert_eq!(metrics.total_commands, 1);
        assert!(metrics.max_latency_us > 0);
    }
    
    #[tokio::test]
    async fn test_key_press_command() {
        let driver = HidDriver::new().await.unwrap();
        
        let cmd = HidCommand::KeyPress { key: 0x41, modifiers: 0 };  // 'A'
        let response = driver.execute(cmd).await;
        
        assert!(response.is_ok());
    }
    
    #[tokio::test]
    async fn test_multiple_commands() {
        let driver = HidDriver::new().await.unwrap();
        
        for i in 0..10 {
            let cmd = HidCommand::MouseMove { x: i * 10, y: i * 10 };
            let response = driver.execute(cmd).await;
            assert!(response.is_ok());
        }
        
        let metrics = driver.metrics();
        assert_eq!(metrics.total_commands, 10);
    }
    
    #[tokio::test]
    async fn test_latency_tracking() {
        let driver = HidDriver::new().await.unwrap();
        
        for _ in 0..100 {
            let cmd = HidCommand::GetCursorPos;
            let _ = driver.execute(cmd).await;
        }
        
        let percentiles = driver.latency_percentiles().unwrap();
        
        // All latencies should be reasonable (relaxed threshold for test environments)
        assert!(percentiles.p99 < 20000, "p99 latency {}us exceeds 20ms", percentiles.p99);
    }
    
    #[tokio::test]
    async fn test_stress_1000_commands() {
        let driver = HidDriver::new().await.unwrap();
        
        let start = Instant::now();
        
        // Execute 1000 diverse commands
        for i in 0..1000 {
            let cmd = match i % 5 {
                0 => HidCommand::MouseMove { x: (i as i32) % 1024, y: (i as i32) % 768 },
                1 => HidCommand::KeyPress { key: 0x41 + (i % 26) as u32, modifiers: 0 },
                2 => HidCommand::Scroll { delta: if i % 2 == 0 { 1 } else { -1 } },
                3 => HidCommand::GetCursorPos,
                _ => HidCommand::QueryKeyState { key: 0x41 },
            };
            
            let response = driver.execute(cmd).await;
            assert!(response.is_ok(), "Command {} failed", i);
        }
        
        let total_time = start.elapsed();
        let metrics = driver.metrics();
        
        // Verify all 1000 commands were executed
        assert_eq!(metrics.total_commands, 1000, "Expected 1000 commands executed");
        
        // Get latency percentiles
        let percentiles = driver.latency_percentiles().unwrap();
        
        // Print stats for verification
        println!("Stress Test Results (1000 commands):");
        println!("  Total time: {:?}", total_time);
        println!("  Avg latency: {}us", metrics.average_latency_us());
        println!("  Min latency: {}us", metrics.min_latency_us);
        println!("  Max latency: {}us", metrics.max_latency_us);
        println!("  p50 latency: {}us", percentiles.p50);
        println!("  p95 latency: {}us", percentiles.p95);
        println!("  p99 latency: {}us", percentiles.p99);
        
        // Validation: keep the deterministic backend bounded without enforcing the
        // real hardware target, which is much tighter.
        assert!(percentiles.p99 < 25000,
            "p99 latency {}us exceeds 25ms test threshold for HID driver",
            percentiles.p99);

        // p95 should also remain bounded.
        assert!(percentiles.p95 < 20000,
            "p95 latency {}us exceeds 20ms test threshold",
            percentiles.p95);
    }
    
    #[tokio::test]
    async fn test_latency_validation_p99() {
        let driver = HidDriver::new().await.unwrap();
        
        // Execute 500 fast operations to build percentile data
        for i in 0..500 {
            let cmd = HidCommand::MouseMove {
                x: (i % 100) as i32,
                y: (i % 100) as i32,
            };
            let _ = driver.execute(cmd).await;
        }
        
        let percentiles = driver.latency_percentiles().unwrap();
        
        // Verify p99 is bounded in the test environment.
        // Real hardware target stays at < 1ms (1000us).
        assert!(percentiles.p99 < 25000,
            "p99 latency must be <25ms in tests for marionette control, got {}us",
            percentiles.p99);

        println!("Latency validation passed: p99={}us (target <25ms in test env, <1ms on real hw)", percentiles.p99);
    }
}
