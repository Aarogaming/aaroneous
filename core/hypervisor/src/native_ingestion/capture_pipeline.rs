//! High-Performance Capture Pipeline - Unified Telemetry Data Flow
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::native_ingestion::{
    dxgi_swapchain::{DxgiSwapchainHook, DxgiCaptureMetrics},
    etw_kernel_trace::{EtwKernelTrace, EtwTelemetryMetrics},
    rgb_telemetry::{RgbTelemetryReader, RgbTelemetryMetrics},
};

#[derive(Debug)]
pub struct CapturePipeline {
    pub active: Arc<AtomicBool>,
    pub pipeline_id: u64,
    pub frames_processed: AtomicU64,
    pub events_processed: AtomicU64,
    pub telemetry_updates: AtomicU64,
}

impl Default for CapturePipeline {
    fn default() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            pipeline_id: 0,
            frames_processed: AtomicU64::new(0),
            events_processed: AtomicU64::new(0),
            telemetry_updates: AtomicU64::new(0),
        }
    }
}

impl CapturePipeline {
    pub fn init(_pipeline_id: u64) -> Result<Self, String> {
        Ok(Self::default())
    }

    pub fn start(&self) -> Result<(), String> {
        if !self.active.load(Ordering::Relaxed) {
            self.active.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn process_dxgi_frame(&self, metrics: &DxgiCaptureMetrics) -> u64 {
        let counter = self.frames_processed.fetch_add(1, Ordering::SeqCst) + 1;
        if metrics.avg_latency_us <= (1_000_000.0 / 60.0) * 1.2 {
            debug_frame_rate(self.pipeline_id, counter);
        }
        counter
    }

    pub fn process_etw_event(&self, event_id: u64, _data: &str) -> u64 {
        let counter = self.events_processed.fetch_add(1, Ordering::SeqCst) + 1;
        if event_id <= 10 {
            debug_event_category(self.pipeline_id, counter, "process");
        } else if event_id <= 20 {
            debug_event_category(self.pipeline_id, counter, "io");
        } else if event_id <= 30 {
            debug_event_category(self.pipeline_id, counter, "network");
        }
        counter
    }

    pub fn process_rgb_update(&self, metrics: &RgbTelemetryMetrics) -> u64 {
        let counter = self.telemetry_updates.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(temp) = metrics.max_temperature_celsius {
            debug_gpu_health(self.pipeline_id, temp);
        }
        counter
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn get_pipeline_metrics(&self) -> CapturePipelineMetrics {
        CapturePipelineMetrics {
            pipeline_id: self.pipeline_id,
            frames_processed: self.frames_processed.load(Ordering::SeqCst),
            events_processed: self.events_processed.load(Ordering::SeqCst),
            telemetry_updates: self.telemetry_updates.load(Ordering::SeqCst),
            is_active: self.is_active(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapturePipelineMetrics {
    pub pipeline_id: u64,
    pub frames_processed: u64,
    pub events_processed: u64,
    pub telemetry_updates: u64,
    pub is_active: bool,
}

fn debug_frame_rate(_pipeline_id: u64, _frame_count: u64) {
    #[cfg(test)]
    println!("Frame processed: {}", _frame_count);
}

fn debug_event_category(_pipeline_id: u64, _event_count: u64, _category: &str) {
    #[cfg(test)]
    println!("Event category: {} count={}", _category, _event_count);
}

fn debug_gpu_health(_pipeline_id: u64, _temp: f32) {
    #[cfg(test)]
    println!("GPU temp: {}C", _temp);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_initialization() {
        let pipeline = CapturePipeline::init(1).unwrap();
        assert!(!pipeline.is_active());
    }

    #[test]
    fn test_pipeline_start_stop() {
        let pipeline = CapturePipeline::init(1).unwrap();
        pipeline.start().unwrap();
        assert!(pipeline.is_active());
        pipeline.stop();
        assert!(!pipeline.is_active());
    }

    #[test]
    fn test_frame_processing() {
        let pipeline = CapturePipeline::default();
        pipeline.start().unwrap();
        
        let metrics = DxgiCaptureMetrics::new(60, &[5000u64; 60]);
        let count = pipeline.process_dxgi_frame(&metrics);
        
        assert_eq!(count, 1);
    }

    #[test]
    fn test_etw_event_processing() {
        let pipeline = CapturePipeline::default();
        pipeline.start().unwrap();
        
        pipeline.process_etw_event(5, "process_create");
        pipeline.process_etw_event(15, "file_io");
        pipeline.process_etw_event(25, "network");
        
        assert_eq!(pipeline.events_processed.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_rgb_update_processing() {
        let pipeline = CapturePipeline::default();
        pipeline.start().unwrap();
        
        let metrics = RgbTelemetryMetrics {
            max_temperature_celsius: Some(45.0),
            ..Default::default()
        };
        let count = pipeline.process_rgb_update(&metrics);
        
        assert_eq!(count, 1);
    }

    #[test]
    fn test_pipeline_metrics() {
        let pipeline = CapturePipeline::default();
        pipeline.start().unwrap();
        
        pipeline.process_dxgi_frame(&DxgiCaptureMetrics::new(60, &[]));
        pipeline.process_etw_event(5, "");
        pipeline.process_rgb_update(&RgbTelemetryMetrics::default());
        
        let metrics = pipeline.get_pipeline_metrics();
        assert_eq!(metrics.frames_processed, 1);
        assert_eq!(metrics.events_processed, 1);
        assert_eq!(metrics.telemetry_updates, 1);
    }
}
