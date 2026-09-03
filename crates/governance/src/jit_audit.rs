//! crates/governance/src/jit_audit.rs
//! Governance JIT Bytecode Security Audit Gate.
//!
//! Enforces zero-trust execution constraints on crystallized native machine code,
//! auditing buffers against unauthorized system calls, hardware interrupts, or
//! privileged instruction execution.

pub use si_format::audit::{audit, jit_audit, AuditResult};
