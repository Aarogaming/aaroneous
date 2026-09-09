//! Cratify — Aaroneous Crate Component lifecycle automation.
//!
//! Provides AST inspection, static audit analysis, LLM translation routing,
//! crate generation, workspace harvesting, and cryptographic certification
//! for the ACC standard.

pub mod inspect;
pub mod audit;
pub mod translate;
pub mod generator;
pub mod harvest;
pub mod scaffold;
pub mod verify;
pub mod workspace;
pub mod fascia;
pub mod certify;
pub mod ring;
pub mod python_to_rust;
