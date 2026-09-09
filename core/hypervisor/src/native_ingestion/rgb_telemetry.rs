//! Hardware RGB Telemetry Status Sync - GPU Health Monitoring
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RgbTelemetryConfig {
    pub gpu_vendor: Option<String>,
    pub monitor_fps: u32,
    pub include_temperature: bool,
    pub include_clock_speed: bool,
}

impl Default for RgbTelemetryConfig {
    fn default() -> Self {
        Self {
            gpu_vendor: None,
            monitor_fps: 60,
            include_temperature: true,
            include_clock_speed: true,
        }
    }
}

#[derive(Debug)]
pub struct RgbTelemetryReader {
    pub active: Arc<AtomicBool>,
    pub frame_counter: AtomicU64,
    pub last_update_ns: AtomicU64,
}

impl Default for RgbTelemetryReader {
    fn default() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            frame_counter: AtomicU64::new(0),
            last_update_ns: AtomicU64::new(0),
        }
    }
}

impl RgbTelemetryReader {
    pub fn init(_config: &RgbTelemetryConfig) -> Result<Self, String> {
        Ok(Self::default())
    }

    pub fn start_sync(&self) -> Result<(), String> {
        if !self.active.load(Ordering::Relaxed) {
            self.active.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn stop_sync(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn get_frame_counter(&self) -> u64 {
        self.frame_counter.load(Ordering::SeqCst)
    }

    pub fn increment_frame(&self) -> u64 {
        self.frame_counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn record_update(&self) -> u64 {
        let ns: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .try_into()
            .unwrap_or(0);
        self.last_update_ns.store(ns, Ordering::SeqCst);
        ns
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct RgbHealthStatus {
    pub temperature_celsius: Option<f32>,
    pub clock_speed_mhz: Option<u32>,
    pub fan_speed_percent: Option<u8>,
    pub power_usage_watts: Option<f32>,
}

impl Default for RgbHealthStatus {
    fn default() -> Self {
        Self {
            temperature_celsius: None,
            clock_speed_mhz: None,
            fan_speed_percent: None,
            power_usage_watts: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RgbTelemetryMetrics {
    pub updates_per_second: f64,
    pub avg_update_latency_us: f64,
    pub max_temperature_celsius: Option<f32>,
    pub min_clock_speed_mhz: Option<u32>,
}

impl Default for RgbTelemetryMetrics {
    fn default() -> Self {
        Self {
            updates_per_second: 0.0,
            avg_update_latency_us: 0.0,
            max_temperature_celsius: None,
            min_clock_speed_mhz: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_telemetry_initialization() {
        let config = RgbTelemetryConfig::default();
        let reader = RgbTelemetryReader::init(&config).unwrap();
        assert!(!reader.is_active());
    }

    #[test]
    fn test_frame_counter_increment() {
        let reader = RgbTelemetryReader::default();
        let f1 = reader.increment_frame();
        let f2 = reader.increment_frame();
        assert_eq!(f2, f1 + 1);
    }

    #[test]
    fn test_rgb_metrics_default() {
        let metrics = RgbTelemetryMetrics::default();
        assert_eq!(metrics.updates_per_second, 0.0);
    }
}
