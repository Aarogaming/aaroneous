//! crates/platform_bridge/src/observability/mod.rs
//! Deep OS Observability & Multi-Modal Sensor Fusion Engine.
//! Provides UI Automation (UIA) tree hierarchy inspection and WASAPI loopback audio ingestion.

pub mod audio_features;
pub mod etw;
pub mod mmcss;
pub mod power_gate;
pub mod raw_input;
pub mod rdtsc;
pub mod shadow_stream;
pub mod uia;
pub mod wasapi;

pub use audio_features::{AcousticFeatureExtractor, AcousticLatent, FFT_SIZE, LATENT_DIM};
pub use etw::{EtwKernelConsumer, KernelTraceEvent, DEFAULT_MAX_RING_CAPACITY};
pub use mmcss::{enable_mmcss_time_critical, set_thread_performance_affinity};
pub use power_gate::{SensorPowerGate, SensorPowerMode};
pub use raw_input::{RawInputListener, RawInputPacket};
pub use rdtsc::{read_cpu_timestamp, HardwareCycleProfiler};
pub use shadow_stream::{ShadowDistillationTap, ShadowExchange};
pub use uia::{UiaElementNode, UiaTreeWalker};
pub use wasapi::{WasapiCaptureConfig, WasapiLoopbackCapture};
