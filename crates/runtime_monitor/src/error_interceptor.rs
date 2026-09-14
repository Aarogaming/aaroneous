// src/error_interceptor.rs

use anyhow::Result;
use super::push_trigger;
use hypervisor::state::telemetry::Trigger;

pub fn intercept_and_push(stderr: &str, timestamp: u64) -> Result<()> {
    let (anomaly, context) = parse_stderr(stderr);
    if anomaly == 0 { return Ok(()); }
    
    let trigger = Trigger {
        timestamp_qpc: timestamp,
        error_code: anomaly,
        severity: if anomaly == 0 { 0 } else { 3 },
        reserved: 0,
        message: context,
    };
    
    push_trigger(trigger);
    Ok(())
}

fn parse_stderr(stderr: &str) -> (u32, [u8; 64]) {
    let mut anomaly = 0u32;
    let mut context = [0u8; 64];
    for line in stderr.lines() {
        if line.contains("error[E") { anomaly = 4; }
        else if line.contains("syntax error") { anomaly = 1; }
        else if line.contains("mismatched types") || line.contains("type mismatch") { anomaly = 2; }
        else if line.contains("panicked at") { anomaly = 3; }
        let bytes = line.as_bytes();
        let len = bytes.len().min(64);
        context[..len].copy_from_slice(&bytes[..len]);
        if anomaly != 0 { break; }
    }
    (anomaly, context)
}
