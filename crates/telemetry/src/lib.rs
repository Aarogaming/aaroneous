// src/lib.rs
use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use once_cell::sync::Lazy;

/// Simple telemetry event for demonstration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TelemetryEvent {
    pub timestamp: u64,
    pub message: String,
    pub level: String,
}

/// Global shared telemetry log.
pub type TelemetryLog = Arc<Mutex<Vec<TelemetryEvent>>>;

static GLOBAL_LOG: Lazy<TelemetryLog> = Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Record a telemetry event using the global log.
pub fn log(event: TelemetryEvent) -> Result<()> {
    let mut guard = GLOBAL_LOG.lock();
    guard.push(event);
    Ok(())
}
