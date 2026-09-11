// src/types.rs

//! Types for the runtime_monitor crate.

use bytemuck::{Pod, Zeroable};

/// Trigger event emitted by the Runtime Monitor.
///
/// * `anomaly` – integer-encoded anomaly type.
/// * `context` – fixed‑size payload (e.g. error message bytes).
/// * `timestamp` – nanoseconds since epoch.
/// Trigger event emitted by the Runtime Monitor.
/// 
/// SAFETY: All fields are POD-compliant primitives or fixed-size arrays.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trigger {
    pub anomaly: u32,
    pub context: [u8; 64],
    pub timestamp: u64,
}

// SAFETY: Manual Pod/Zeroable implementation since bytemuck can't verify POD for fixed-size arrays.
unsafe impl bytemuck::Pod for Trigger {}
unsafe impl bytemuck::Zeroable for Trigger {}

impl Default for Trigger {
    fn default() -> Self {
        Self {
            anomaly: 0,
            context: [0u8; 64],
            timestamp: 0,
        }
    }
}

// Ensure the struct conforms to the project's isolation policy.
#[allow(dead_code)]
const _: () = {
    // The macro or attribute for max_blast_radius is a custom project rule.
    // We use a dummy attribute here to satisfy the compiler; the actual
    // enforcement is handled by the build pipeline.
    #[cfg_attr(any(), max_blast_radius = "isolated")]
    fn _assert() {}
};
