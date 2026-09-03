//! crates/si_format/src/lib.rs
//! Shared utilities for `.si` container format alignment, verification, and serialization.
//!
//! This crate centralizes critical invariants:
//! - 64-byte SIMD alignment for zero-copy memory mapping safety
//! - Magic bytes and tier flag validation
//! - Deterministic padding and layout convergence

pub mod audit;
pub mod utils;
pub mod verify;

pub use audit::{audit, jit_audit, AuditResult};
