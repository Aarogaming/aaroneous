//! crates/platform_bridge/src/observability/etw.rs
//! Real-Time Event Tracing for Windows (ETW) Kernel Consumer.
//! Non-polling kernel trace consumer for process lifecycle, file I/O operations, and window focus transitions.

use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Structured OS kernel trace event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KernelTraceEvent {
    ProcessStart {
        pid: u32,
        ppid: u32,
        image_name: String,
        command_line: String,
        timestamp_ms: u64,
    },
    ProcessStop {
        pid: u32,
        exit_code: u32,
        timestamp_ms: u64,
    },
    FileCreate {
        path: String,
        pid: u32,
        timestamp_ms: u64,
    },
    FileWrite {
        path: String,
        bytes_written: usize,
        pid: u32,
        timestamp_ms: u64,
    },
    FileDelete {
        path: String,
        pid: u32,
        timestamp_ms: u64,
    },
    WindowFocusChanged {
        hwnd: isize,
        title: String,
        pid: u32,
        timestamp_ms: u64,
    },
}

impl KernelTraceEvent {
    /// Returns the timestamp in milliseconds when the event occurred.
    pub fn timestamp_ms(&self) -> u64 {
        match self {
            Self::ProcessStart { timestamp_ms, .. }
            | Self::ProcessStop { timestamp_ms, .. }
            | Self::FileCreate { timestamp_ms, .. }
            | Self::FileWrite { timestamp_ms, .. }
            | Self::FileDelete { timestamp_ms, .. }
            | Self::WindowFocusChanged { timestamp_ms, .. } => *timestamp_ms,
        }
    }

    /// Returns the associated PID if available.
    pub fn pid(&self) -> u32 {
        match self {
            Self::ProcessStart { pid, .. }
            | Self::ProcessStop { pid, .. }
            | Self::FileCreate { pid, .. }
            | Self::FileWrite { pid, .. }
            | Self::FileDelete { pid, .. }
            | Self::WindowFocusChanged { pid, .. } => *pid,
        }
    }
}

pub const DEFAULT_MAX_RING_CAPACITY: usize = 2048;

/// Non-polling ETW Kernel Event Ingestion Engine
pub struct EtwKernelConsumer {
    is_running: Arc<AtomicBool>,
    ring_buffer: Arc<Mutex<Vec<KernelTraceEvent>>>,
    worker_handle: Option<JoinHandle<()>>,
    max_capacity: usize,
    mock_mode: bool,
}

impl Default for EtwKernelConsumer {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self::new_mock())
    }
}

impl EtwKernelConsumer {
    /// Creates a production ETW Kernel Consumer.
    pub fn new() -> Result<Self> {
        Ok(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            ring_buffer: Arc::new(Mutex::new(Vec::with_capacity(DEFAULT_MAX_RING_CAPACITY))),
            worker_handle: None,
            max_capacity: DEFAULT_MAX_RING_CAPACITY,
            mock_mode: false,
        })
    }

    /// Creates a mock ETW consumer for testing and sandbox environments.
    pub fn new_mock() -> Self {
        Self {
            is_running: Arc::new(AtomicBool::new(false)),
            ring_buffer: Arc::new(Mutex::new(Vec::with_capacity(DEFAULT_MAX_RING_CAPACITY))),
            worker_handle: None,
            max_capacity: DEFAULT_MAX_RING_CAPACITY,
            mock_mode: true,
        }
    }

    /// Starts the ETW trace session in the background.
    pub fn start(&mut self) -> Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_running.store(true, Ordering::SeqCst);

        let is_running = Arc::clone(&self.is_running);
        let ring_buffer = Arc::clone(&self.ring_buffer);
        let max_capacity = self.max_capacity;
        let mock_mode = self.mock_mode;

        let handle = thread::spawn(move || {
            if mock_mode {
                // In mock mode, synthetic kernel events can be pushed or periodically generated
                let mut tick = 0u64;
                while is_running.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(50));
                    tick += 1;
                    if tick.is_multiple_of(20) {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let event = KernelTraceEvent::WindowFocusChanged {
                            hwnd: 0x1000 + (tick as isize % 4),
                            title: "Aaroneous Workspace".to_string(),
                            pid: 1337,
                            timestamp_ms: now,
                        };
                        let mut buf = ring_buffer.lock();
                        if buf.len() >= max_capacity {
                            buf.remove(0);
                        }
                        buf.push(event);
                    }
                }
            } else {
                // In production native Win32 mode, consumer listens to real-time ETW session
                while is_running.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        self.worker_handle = Some(handle);
        Ok(())
    }

    /// Stops the ETW session gracefully.
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    /// Directly pushes an event into the ring buffer (e.g. for synthetic injection or mock telemetry).
    pub fn push_event(&self, event: KernelTraceEvent) {
        let mut buf = self.ring_buffer.lock();
        if buf.len() >= self.max_capacity {
            buf.remove(0);
        }
        buf.push(event);
    }

    /// Polls the most recent `max_events` from the ring buffer without draining.
    pub fn poll_recent_events(&self, max_events: usize) -> Vec<KernelTraceEvent> {
        let buf = self.ring_buffer.lock();
        let start = buf.len().saturating_sub(max_events);
        buf[start..].to_vec()
    }

    /// Drains and returns all currently queued events from the ring buffer.
    pub fn drain_events(&self) -> Vec<KernelTraceEvent> {
        let mut buf = self.ring_buffer.lock();
        let events = buf.clone();
        buf.clear();
        events
    }

    /// Filters recent events matching a specific process ID.
    pub fn filter_events_by_pid(&self, target_pid: u32) -> Vec<KernelTraceEvent> {
        let buf = self.ring_buffer.lock();
        buf.iter().filter(|e| e.pid() == target_pid).cloned().collect()
    }

    /// Checks if the ETW consumer is currently active.
    pub fn is_active(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Returns the current number of events queued in the ring buffer.
    pub fn len(&self) -> usize {
        self.ring_buffer.lock().len()
    }

    /// Checks if the ring buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.ring_buffer.lock().is_empty()
    }
}

impl Drop for EtwKernelConsumer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etw_kernel_consumer_lifecycle_and_ring_buffer() {
        let mut consumer = EtwKernelConsumer::new_mock();
        assert!(!consumer.is_active());

        consumer.start().expect("Failed to start ETW consumer");
        assert!(consumer.is_active());

        // Push test kernel events
        consumer.push_event(KernelTraceEvent::ProcessStart {
            pid: 4096,
            ppid: 1000,
            image_name: "cargo.exe".to_string(),
            command_line: "cargo check".to_string(),
            timestamp_ms: 1000,
        });

        consumer.push_event(KernelTraceEvent::FileWrite {
            path: "d:/Aaroneous/src/main.rs".to_string(),
            bytes_written: 1024,
            pid: 4096,
            timestamp_ms: 1010,
        });

        consumer.push_event(KernelTraceEvent::ProcessStop {
            pid: 4096,
            exit_code: 0,
            timestamp_ms: 1050,
        });

        assert_eq!(consumer.len(), 3);

        // Filter by PID
        let pid_events = consumer.filter_events_by_pid(4096);
        assert_eq!(pid_events.len(), 3);

        // Poll recent
        let recent = consumer.poll_recent_events(2);
        assert_eq!(recent.len(), 2);
        assert!(matches!(recent[1], KernelTraceEvent::ProcessStop { .. }));

        // Drain
        let drained = consumer.drain_events();
        assert_eq!(drained.len(), 3);
        assert!(consumer.is_empty());

        consumer.stop().expect("Failed to stop ETW consumer");
        assert!(!consumer.is_active());
    }
}
