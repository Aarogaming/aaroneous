// src/lib.rs

//! Runtime Monitor fast‑path crate.
//!
//! Provides zero‑copy ingestion of telemetry from the Hypervisor RingBuffer and
//! generation of POD `Trigger` events for the mutation engine.

mod types;
mod streaming_adaptation;
mod error_interceptor;

pub use types::Trigger;
pub use streaming_adaptation::StreamingLoraAdaptationPipeline;
pub use error_interceptor::ProcessErrorInterceptor;

