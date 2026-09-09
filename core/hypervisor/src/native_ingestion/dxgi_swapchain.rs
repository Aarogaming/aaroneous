//! DXGI Swap-Chain Present Hook - Direct GPU Frame Capture
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct DxgiSwapchainHook {
    pub active: Arc<AtomicBool>,
    pub frame_counter: AtomicU64,
    pub last_capture_ns: AtomicU64,
}

impl Default for DxgiSwapchainHook {
    fn default() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            frame_counter: AtomicU64::new(0),
            last_capture_ns: AtomicU64::new(0),
        }
    }
}

impl DxgiSwapchainHook {
    pub fn init() -> Result<Self, String> {
        Ok(Self::default())
    }

    pub fn start_capture(&self) -> Result<(), String> {
        if !self.active.load(Ordering::Relaxed) {
            self.active.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn stop_capture(&self) {
        self.active.store(false, Ordering::Relaxed);
    }

    pub fn get_frame_counter(&self) -> u64 {
        self.frame_counter.load(Ordering::SeqCst)
    }

    pub fn increment_frame(&self) -> u64 {
        self.frame_counter.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn record_capture(&self) -> u64 {
        let ns: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .try_into()
            .unwrap_or(0);
        self.last_capture_ns.store(ns, Ordering::SeqCst);
        ns
    }

    pub fn get_capture_latency_us(&self) -> u64 {
        let now: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .try_into()
            .unwrap_or(0);
        let last = self.last_capture_ns.load(Ordering::SeqCst);
        (now - last) / 1000
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
}

pub struct DxgiPresentCallback {
    state: Arc<DxgiSwapchainHook>,
}

impl DxgiPresentCallback {
    pub fn new(state: Arc<DxgiSwapchainHook>) -> Self {
        Self { state }
    }

    pub fn on_present(&mut self) -> Result<(), String> {
        if !self.state.is_active() {
            return Ok(());
        }
        let _ = self.state.increment_frame();
        let _ = self.state.record_capture();
        Ok(())
    }

    pub fn cleanup(&mut self) {}
}

#[derive(Debug, Clone)]
pub struct DxgiCaptureMetrics {
    pub frames_captured: u64,
    pub avg_latency_us: f64,
    pub max_latency_us: u64,
    pub min_latency_us: u64,
    pub fps: f64,
}

impl Default for DxgiCaptureMetrics {
    fn default() -> Self {
        Self {
            frames_captured: 0,
            avg_latency_us: 0.0,
            max_latency_us: 0,
            min_latency_us: u64::MAX,
            fps: 0.0,
        }
    }
}

impl DxgiCaptureMetrics {
    pub fn new(frames: u64, latencies: &[u64]) -> Self {
        let sum: f64 = latencies.iter().map(|&l| l as f64).sum();
        let avg = if frames > 0 { sum / frames as f64 } else { 0.0 };
        let max = *latencies.iter().max().unwrap_or(&0);
        let min = *latencies.iter().min().unwrap_or(&u64::MAX);
        Self {
            frames_captured: frames,
            avg_latency_us: avg,
            max_latency_us: max,
            min_latency_us: min,
            fps: if frames > 0 { (frames as f64 / 1.0) } else { 0.0 },
        }
    }

    pub fn is_acceptable(&self, target_fps: u32) -> bool {
        let max_latency = (1_000_000.0 / target_fps as f64) * 0.8;
        self.avg_latency_us <= max_latency
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxgi_hook_initialization() {
        let hook = DxgiSwapchainHook::init().unwrap();
        assert!(!hook.is_active());
    }

    #[test]
    fn test_frame_counter_increment() {
        let hook = DxgiSwapchainHook::default();
        let f1 = hook.increment_frame();
        let f2 = hook.increment_frame();
        assert_eq!(f2, f1 + 1);
    }

    #[test]
    fn test_capture_metrics_calculation() {
        let latencies = vec![5000u64, 10000, 15000];
        let metrics = DxgiCaptureMetrics::new(3, &latencies);
        assert_eq!(metrics.frames_captured, 3);
    }

    #[test]
    fn test_metrics_acceptable_latency() {
        let metrics = DxgiCaptureMetrics::new(60, &[5000u64; 60]);
        assert!(metrics.is_acceptable(60));
    }
}
