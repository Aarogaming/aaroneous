//! GPU monitoring module for platform observability.

/// GPU telemetry data structure.
#[derive(Debug, Clone, PartialEq)]
pub struct GpuTelemetry {
    pub temperature_c: u32,
    pub vram_used_mb: u64,
}

/// GPU memory monitor for querying and analyzing GPU status.
pub struct GpuMemoryMonitor;

impl GpuMemoryMonitor {
    /// Queries current GPU thermals and memory utilization.
    /// Returns None or fallback if GPU query is unavailable or command fails.
    pub async fn query_gpu_status() -> Option<GpuTelemetry> {
        // Simulate GPU query with fallback behavior
        // In production, this would interface with actual GPU drivers
        // For now, we provide a deterministic fallback for testing
        Some(GpuTelemetry {
            temperature_c: 65,
            vram_used_mb: 2048,
        })
    }

    /// Checks if thermals exceed max_temp threshold.
    pub fn is_throttled(telemetry: &GpuTelemetry, max_temp: u32) -> bool {
        telemetry.temperature_c >= max_temp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_throttled_below_threshold() {
        let telemetry = GpuTelemetry {
            temperature_c: 60,
            vram_used_mb: 1024,
        };
        assert!(!GpuMemoryMonitor::is_throttled(&telemetry, 70));
    }

    #[test]
    fn test_is_throttled_at_threshold() {
        let telemetry = GpuTelemetry {
            temperature_c: 70,
            vram_used_mb: 1024,
        };
        assert!(GpuMemoryMonitor::is_throttled(&telemetry, 70));
    }

    #[test]
    fn test_is_throttled_above_threshold() {
        let telemetry = GpuTelemetry {
            temperature_c: 85,
            vram_used_mb: 1024,
        };
        assert!(GpuMemoryMonitor::is_throttled(&telemetry, 70));
    }

    #[tokio::test]
    async fn test_query_gpu_status_returns_some() {
        let result = GpuMemoryMonitor::query_gpu_status().await;
        assert!(result.is_some());
    }

    #[test]
    fn test_gpu_telemetry_equality() {
        let telemetry1 = GpuTelemetry {
            temperature_c: 65,
            vram_used_mb: 2048,
        };
        let telemetry2 = GpuTelemetry {
            temperature_c: 65,
            vram_used_mb: 2048,
        };
        assert_eq!(telemetry1, telemetry2);
    }

    #[test]
    fn test_gpu_telemetry_debug() {
        let telemetry = GpuTelemetry {
            temperature_c: 65,
            vram_used_mb: 2048,
        };
        let debug_str = format!("{:?}", telemetry);
        assert!(debug_str.contains("GpuTelemetry"));
        assert!(debug_str.contains("temperature_c"));
        assert!(debug_str.contains("vram_used_mb"));
    }

    #[test]
    fn test_gpu_telemetry_clone() {
        let telemetry = GpuTelemetry {
            temperature_c: 65,
            vram_used_mb: 2048,
        };
        let cloned = telemetry.clone();
        assert_eq!(telemetry, cloned);
    }

    #[test]
    fn test_gpu_telemetry_different_values() {
        let telemetry1 = GpuTelemetry {
            temperature_c: 65,
            vram_used_mb: 2048,
        };
        let telemetry2 = GpuTelemetry {
            temperature_c: 70,
            vram_used_mb: 1024,
        };
        assert_ne!(telemetry1, telemetry2);
    }

    #[test]
    fn test_is_throttled_zero_max_temp() {
        let telemetry = GpuTelemetry {
            temperature_c: 0,
            vram_used_mb: 0,
        };
        assert!(GpuMemoryMonitor::is_throttled(&telemetry, 0));
    }

    #[test]
    fn test_is_throttled_high_temp() {
        let telemetry = GpuTelemetry {
            temperature_c: 100,
            vram_used_mb: 4096,
        };
        assert!(GpuMemoryMonitor::is_throttled(&telemetry, 90));
    }
}
