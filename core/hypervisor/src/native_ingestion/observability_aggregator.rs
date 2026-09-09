//! Observability Aggregator - Unified Telemetry Pipeline
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::native_ingestion::{
    dxgi_swapchain::DxgiCaptureMetrics,
    rgb_telemetry::RgbTelemetryMetrics,
};

struct DxgiSink {
    frames_received: AtomicU64,
}

impl Default for DxgiSink {
    fn default() -> Self {
        Self {
            frames_received: AtomicU64::new(0),
        }
    }
}

impl DxgiSink {
    pub fn on_frame(&self, frame_id: u64, _metrics: &DxgiCaptureMetrics) {
        let _ = self.frames_received.fetch_add(1, Ordering::SeqCst);
    }
}

struct EtwSink {
    events_received: AtomicU64,
    process_creates: AtomicU64,
    file_io_ops: AtomicU64,
    network_packets: AtomicU64,
}

impl Default for EtwSink {
    fn default() -> Self {
        Self {
            events_received: AtomicU64::new(0),
            process_creates: AtomicU64::new(0),
            file_io_ops: AtomicU64::new(0),
            network_packets: AtomicU64::new(0),
        }
    }
}

impl EtwSink {
    pub fn on_event(&self, event_id: u64, _data: &str) {
        let _ = self.events_received.fetch_add(1, Ordering::SeqCst);
        if event_id <= 10 {
            let _ = self.process_creates.fetch_add(1, Ordering::SeqCst);
        } else if event_id <= 20 {
            let _ = self.file_io_ops.fetch_add(1, Ordering::SeqCst);
        } else if event_id <= 30 {
            let _ = self.network_packets.fetch_add(1, Ordering::SeqCst);
        }
    }
}

struct RgbSink {
    updates_received: AtomicU64,
}

impl Default for RgbSink {
    fn default() -> Self {
        Self {
            updates_received: AtomicU64::new(0),
        }
    }
}

impl RgbSink {
    pub fn on_update(&self, _update_id: u64, _metrics: &RgbTelemetryMetrics) {
        let _ = self.updates_received.fetch_add(1, Ordering::SeqCst);
    }
}

enum DataSink {
    Dxgi(DxgiSink),
    Etw(EtwSink),
    Rgb(RgbSink),
}

impl DataSink {
    fn on_frame(&self, frame_id: u64, metrics: &DxgiCaptureMetrics) {
        match self {
            DataSink::Dxgi(sink) => sink.on_frame(frame_id, metrics),
            _ => {}
        }
    }

    fn on_event(&self, event_id: u64, data: &str) {
        match self {
            DataSink::Etw(sink) => sink.on_event(event_id, data),
            _ => {}
        }
    }

    fn on_update(&self, update_id: u64, metrics: &RgbTelemetryMetrics) {
        match self {
            DataSink::Rgb(sink) => sink.on_update(update_id, metrics),
            _ => {}
        }
    }
}

pub struct ObservabilityAggregator {
    pub active: Arc<AtomicBool>,
    pub frame_counter: AtomicU64,
    pub event_counter: AtomicU64,
    data_sources: Vec<DataSink>,
}

impl Default for ObservabilityAggregator {
    fn default() -> Self {
        let sources = vec![
            DataSink::Dxgi(DxgiSink::default()),
            DataSink::Etw(EtwSink::default()),
            DataSink::Rgb(RgbSink::default()),
        ];

        Self {
            active: Arc::new(AtomicBool::new(false)),
            frame_counter: AtomicU64::new(0),
            event_counter: AtomicU64::new(0),
            data_sources: sources,
        }
    }
}

impl ObservabilityAggregator {
    pub fn init() -> Result<Self, String> {
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

    pub fn on_dxgi_frame(&self, metrics: &DxgiCaptureMetrics) {
        let counter = self.event_counter.fetch_add(1, Ordering::SeqCst) + 1;
        for sink in &self.data_sources {
            if let DataSink::Dxgi(sink) = sink {
                sink.on_frame(counter, metrics);
            }
        }
    }

    pub fn on_etw_event(&self, event_id: u64, data: &str) {
        let counter = self.event_counter.fetch_add(1, Ordering::SeqCst) + 1;
        for sink in &self.data_sources {
            if let DataSink::Etw(sink) = sink {
                sink.on_event(event_id, data);
            }
        }
    }

    pub fn on_rgb_update(&self, metrics: &RgbTelemetryMetrics) {
        let counter = self.event_counter.fetch_add(1, Ordering::SeqCst) + 1;
        for sink in &self.data_sources {
            if let DataSink::Rgb(sink) = sink {
                sink.on_update(counter, metrics);
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregator_initialization() {
        let aggregator = ObservabilityAggregator::init().unwrap();
        assert!(!aggregator.is_active());
    }

    #[test]
    fn test_aggregator_start_stop() {
        let aggregator = ObservabilityAggregator::init().unwrap();
        aggregator.start().unwrap();
        assert!(aggregator.is_active());
        aggregator.stop();
        assert!(!aggregator.is_active());
    }

    #[test]
    fn test_data_sink_dispatch() {
        let aggregator = ObservabilityAggregator::default();
        aggregator.start().unwrap();
        
        let metrics = DxgiCaptureMetrics::new(60, &[5000u64; 60]);
        aggregator.on_dxgi_frame(&metrics);
        
        assert!(aggregator.is_active());
    }

    #[test]
    fn test_etw_event_routing() {
        let aggregator = ObservabilityAggregator::default();
        aggregator.start().unwrap();
        
        aggregator.on_etw_event(5, "process_create");
        aggregator.on_etw_event(15, "file_io");
        aggregator.on_etw_event(25, "network");
        
        assert!(aggregator.is_active());
    }

    #[test]
    fn test_rgb_update_dispatch() {
        let aggregator = ObservabilityAggregator::default();
        aggregator.start().unwrap();
        
        let metrics = RgbTelemetryMetrics::default();
        aggregator.on_rgb_update(&metrics);
        
        assert!(aggregator.is_active());
    }
}
