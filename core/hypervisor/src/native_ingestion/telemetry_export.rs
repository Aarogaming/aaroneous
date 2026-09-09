//! Telemetry Export Layer - Serialize and Transmit Observability Data
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::native_ingestion::{
    dxgi_swapchain::DxgiCaptureMetrics,
    etw_kernel_trace::EtwTelemetryMetrics,
    rgb_telemetry::RgbTelemetryMetrics,
    capture_pipeline::CapturePipeline,
};

#[derive(Debug)]
pub struct TelemetryExporter {
    pub active: Arc<AtomicBool>,
    pub export_counter: AtomicU64,
    pub bytes_exported: AtomicU64,
}

impl Default for TelemetryExporter {
    fn default() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            export_counter: AtomicU64::new(0),
            bytes_exported: AtomicU64::new(0),
        }
    }
}

impl TelemetryExporter {
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

    pub fn export_dxgi_metrics(&self, metrics: &DxgiCaptureMetrics) -> Result<u64, String> {
        if !self.active.load(Ordering::Relaxed) {
            return Err("Exporter not active".to_string());
        }
        
        let counter = self.export_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let bytes = 128u64; // Mock serialization size
        self.bytes_exported.fetch_add(bytes, Ordering::SeqCst);
        Ok(counter)
    }

    pub fn export_etw_metrics(&self, _metrics: &EtwTelemetryMetrics) -> Result<u64, String> {
        if !self.active.load(Ordering::Relaxed) {
            return Err("Exporter not active".to_string());
        }
        
        let counter = self.export_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let bytes = 64u64; // Mock serialization size
        self.bytes_exported.fetch_add(bytes, Ordering::SeqCst);
        Ok(counter)
    }

    pub fn export_rgb_metrics(&self, _metrics: &RgbTelemetryMetrics) -> Result<u64, String> {
        if !self.active.load(Ordering::Relaxed) {
            return Err("Exporter not active".to_string());
        }
        
        let counter = self.export_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let bytes = 96u64; // Mock serialization size
        self.bytes_exported.fetch_add(bytes, Ordering::SeqCst);
        Ok(counter)
    }

    pub fn export_pipeline_metrics(&self, pipeline: &CapturePipeline) -> Result<u64, String> {
        if !self.active.load(Ordering::Relaxed) {
            return Err("Exporter not active".to_string());
        }
        
        let counter = self.export_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let bytes = 256u64; // Mock serialization size
        self.bytes_exported.fetch_add(bytes, Ordering::SeqCst);
        Ok(counter)
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    pub fn get_export_stats(&self) -> TelemetryExportStats {
        TelemetryExportStats {
            export_counter: self.export_counter.load(Ordering::SeqCst),
            bytes_exported: self.bytes_exported.load(Ordering::SeqCst),
            is_active: self.is_active(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryExportStats {
    pub export_counter: u64,
    pub bytes_exported: u64,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_exporter_initialization() {
        let exporter = TelemetryExporter::init().unwrap();
        assert!(!exporter.is_active());
    }

    #[test]
    fn test_exporter_start_stop() {
        let exporter = TelemetryExporter::init().unwrap();
        exporter.start().unwrap();
        assert!(exporter.is_active());
        exporter.stop();
        assert!(!exporter.is_active());
    }

    #[test]
    fn test_dxgi_export_success() {
        let exporter = TelemetryExporter::default();
        exporter.start().unwrap();
        
        let metrics = DxgiCaptureMetrics::new(60, &[5000u64; 60]);
        let counter = exporter.export_dxgi_metrics(&metrics).unwrap();
        
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_etw_export_success() {
        let exporter = TelemetryExporter::default();
        exporter.start().unwrap();
        
        let metrics = EtwTelemetryMetrics {
            events_ingested: 100,
            
            ..Default::default()
        };
        let counter = exporter.export_etw_metrics(&metrics).unwrap();
        
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_rgb_export_success() {
        let exporter = TelemetryExporter::default();
        exporter.start().unwrap();
        
        let metrics = RgbTelemetryMetrics::default();
        let counter = exporter.export_rgb_metrics(&metrics).unwrap();
        
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_export_failure_inactive() {
        let exporter = TelemetryExporter::default();
        // Don't start - should fail
        
        let metrics = DxgiCaptureMetrics::new(60, &[]);
        assert!(exporter.export_dxgi_metrics(&metrics).is_err());
    }

    #[test]
    fn test_export_stats() {
        let exporter = TelemetryExporter::default();
        exporter.start().unwrap();
        
        for _ in 0..5 {
            exporter.export_dxgi_metrics(&DxgiCaptureMetrics::new(60, &[])).unwrap();
        }
        
        let stats = exporter.get_export_stats();
        assert_eq!(stats.export_counter, 5);
        assert!(stats.is_active);
    }

    #[test]
    fn test_pipeline_export() {
        let exporter = TelemetryExporter::default();
        exporter.start().unwrap();
        
        let pipeline = CapturePipeline::init(1).unwrap();
        pipeline.start().unwrap();
        
        let counter = exporter.export_pipeline_metrics(&pipeline).unwrap();
        assert_eq!(counter, 1);
    }
}
