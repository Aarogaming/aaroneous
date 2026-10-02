//! Compatibility facade for the reusable Cratify audit engine.
//!
//! The implementation lives in `cratify_core` so tooling crates can share it
//! without depending on this CLI package. Existing consumers may continue to
//! use the established `ast_auditor` library API.

#![deny(unsafe_code)]

pub use cratify_core::*;
