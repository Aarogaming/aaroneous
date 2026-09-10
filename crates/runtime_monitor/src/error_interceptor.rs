// src/error_interceptor.rs

//! Zero‑copy process error interceptor for the Runtime Monitor fast‑path.
//! Parses stderr, constructs a `Trigger` POD and pushes it onto a lock‑free queue.

use anyhow::Result;
use bytemuck::Pod;
use super::types::Trigger;
use super::push_trigger;

/// Parses the stderr into an anomaly type and a context buffer.
///
/// * Returns `anomaly` as an integer code (0 = none, 1 = syntax, 2 = type,
///   3 = panic, 4 = other error).
/// * Copies up to 64 bytes of the first matching line into `context`.
fn parse_stderr(stderr: &str) -> (u32, [u8; 64]) {
    let mut anomaly = 0u32;
    let mut context = [0u8; 64];
    for line in stderr.lines() {
        // Detect known patterns.
        if line.contains("error[E") {
            anomaly = 4; // other error code
        } else if line.contains("syntax error") {
            anomaly = 1;
        } else if line.contains("mismatched types") || line.contains("type mismatch") {
            anomaly = 2;
        } else if line.contains("panicked at") {
            anomaly = 3;
        }
        // Copy the line into the fixed buffer (truncated if needed).
        let bytes = line.as_bytes();
        let len = bytes.len().min(64);
        context[..len].copy_from_slice(&bytes[..len]);
        // Stop after first line that gave us an anomaly.
        if anomaly != 0 {
            break;
        }
    }
    (anomaly, context)
}

/// Intercepts process error output and pushes a `Trigger` onto the global queue.
///
/// Returns `Ok(())` for successful handling.
pub fn intercept_and_push(stderr: &str, timestamp: u64) -> Result<()> {
    let (anomaly, context) = parse_stderr(stderr);
    if anomaly == 0 {
        // No relevant error – nothing to push.
        return Ok(());
    }
    let trigger = Trigger {
        anomaly,
        context,
        timestamp,
    };
    push_trigger(trigger);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn test_intercept_and_push() {
        let stderr = "error[E0433]: cannot find type `PathBuf` in this scope\nthread 'main' panicked at 'assertion failed', src/main.rs:15:5";
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64;
        intercept_and_push(stderr, ts).unwrap();
        // Verify that a trigger was queued.
        // The queue is internal; we can only ensure no panic.
    }
}
