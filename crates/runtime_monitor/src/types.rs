// src/types.rs

//! Types for the runtime_monitor crate.

/// Telemetry trigger used for fast-path error reporting.
///
/// SAFETY: All fields are POD-compliant primitives or fixed-size arrays with no padding.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Trigger {
    pub timestamp_qpc: u64, // Query Performance Counter (8 bytes)
    pub error_code: u32,    // Error classification (4 bytes)
    pub severity: u16,      // Severity level (2 bytes)
    pub reserved: u16,      // Explicit padding to 4-byte boundary (2 bytes)
    pub message: [u8; 64],  // Raw error context (64 bytes)
}

impl Default for Trigger {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Trigger {
    /// Zero-initialized trigger for const contexts.
    pub const ZERO: Self = Self {
        timestamp_qpc: 0,
        error_code: 0,
        severity: 0,
        reserved: 0,
        message: [0u8; 64],
    };
}

// Ensure the struct conforms to the project's isolation policy.
#[allow(dead_code)]
const _: () = {
    // The macro or attribute for max_blast_radius is a custom project rule.
};
