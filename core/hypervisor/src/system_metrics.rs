/// System Metrics - GPU and Thermal Monitoring
///
/// Provides real-time GPU load, thermal status, and resource metrics
/// for adaptive throttling and performance management.
use std::fs;
use std::path::Path;

/// GPU load measurement (0.0 = idle, 1.0 = fully loaded)
#[derive(Debug, Clone, Copy)]
pub struct GpuMetrics {
    pub load: f64,
    pub temperature: f64,  // degrees Celsius
    pub memory_used: u64,  // bytes
    pub memory_total: u64, // bytes
    pub power_draw: f64,   // watts
}

impl Default for GpuMetrics {
    fn default() -> Self {
        Self {
            load: 0.0,
            temperature: 25.0,
            memory_used: 0,
            memory_total: 0,
            power_draw: 0.0,
        }
    }
}

/// CPU thermal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalStatus {
    Cool,     // < 50°C
    Normal,   // 50-75°C
    Warm,     // 75-85°C
    Hot,      // 85-95°C
    Critical, // > 95°C
    Unknown,  // sensor unavailable
}

impl ThermalStatus {
    pub fn from_temperature(celsius: f64) -> Self {
        match celsius {
            c if c < 50.0 => ThermalStatus::Cool,
            c if c < 75.0 => ThermalStatus::Normal,
            c if c < 85.0 => ThermalStatus::Warm,
            c if c < 95.0 => ThermalStatus::Hot,
            c if c >= 95.0 => ThermalStatus::Critical,
            _ => ThermalStatus::Unknown,
        }
    }

    pub fn should_throttle(&self) -> bool {
        // Unknown means the sensor couldn't be read, not that the system is
        // safe: it should not be treated as verified-fine the way Cool/Normal
        // are (owner decision, C65 review). Warm's pre-existing exclusion
        // from this set is unrelated to this fix and is left as-is.
        matches!(
            self,
            ThermalStatus::Hot | ThermalStatus::Critical | ThermalStatus::Unknown
        )
    }

    pub fn throttle_factor(&self) -> f64 {
        match self {
            ThermalStatus::Cool => 1.0,
            ThermalStatus::Normal => 1.0,
            ThermalStatus::Warm => 0.9,
            ThermalStatus::Hot => 0.7,
            ThermalStatus::Critical => 0.5,
            // Unmeasured is not the same as measured-and-fine: apply the same
            // mild caution as Warm rather than running unthrottled on a
            // system we can't actually verify is safe.
            ThermalStatus::Unknown => 0.9,
        }
    }
}

/// System thermal metrics
#[derive(Debug, Clone)]
pub struct ThermalMetrics {
    pub cpu_temperature: f64, // degrees Celsius
    pub cpu_status: ThermalStatus,
    /// `false` when `cpu_temperature` is a placeholder because no real
    /// sensor was read (e.g. no Windows thermal API implementation, or a
    /// hwmon read failure), rather than an actual measurement.
    pub cpu_measured: bool,
    pub gpu_temperature: f64,
    pub gpu_status: ThermalStatus,
    /// `false` when `gpu_temperature` is a placeholder (NVML unavailable or
    /// the `gpu-metrics` feature is disabled), rather than an actual
    /// measurement.
    pub gpu_measured: bool,
    pub max_temperature: f64,
    pub throttling_active: bool,
}

impl Default for ThermalMetrics {
    fn default() -> Self {
        Self {
            cpu_temperature: 25.0,
            cpu_status: ThermalStatus::Unknown,
            cpu_measured: false,
            gpu_temperature: 25.0,
            gpu_status: ThermalStatus::Unknown,
            gpu_measured: false,
            max_temperature: 25.0,
            throttling_active: false,
        }
    }
}

/// System Metrics Collector
pub struct SystemMetricsCollector {
    use_nvml: bool,  // NVIDIA GPU monitoring available
    use_hwmon: bool, // Linux hwmon thermal sensors available
    _nvml_device_index: u32,
}

impl SystemMetricsCollector {
    pub fn new() -> Self {
        // Check if NVML (NVIDIA GPU) is available
        let use_nvml = Self::check_nvml_available();

        // Check if hwmon (Linux thermal sensors) is available
        let use_hwmon = Self::check_hwmon_available();

        Self {
            use_nvml,
            use_hwmon,
            _nvml_device_index: 0,
        }
    }

    fn check_nvml_available() -> bool {
        #[cfg(feature = "gpu-metrics")]
        {
            nvml_wrapper::Nvml::init().is_ok()
        }
        #[cfg(not(feature = "gpu-metrics"))]
        {
            false
        }
    }

    fn check_hwmon_available() -> bool {
        // Check if Linux hwmon thermal zone exists
        Path::new("/sys/class/thermal/thermal_zone0/temp").exists()
    }

    /// Get current GPU metrics
    pub fn get_gpu_metrics(&self) -> GpuMetrics {
        if self.use_nvml {
            self.get_gpu_metrics_nvidia()
        } else {
            GpuMetrics::default()
        }
    }

    fn get_gpu_metrics_nvidia(&self) -> GpuMetrics {
        #[cfg(feature = "gpu-metrics")]
        {
            if let Ok(nvml) = nvml_wrapper::Nvml::init() {
                if let Ok(device) = nvml.device_by_index(self._nvml_device_index) {
                    let utilization = device.utilization_rates().ok();
                    let gpu_load = utilization
                        .as_ref()
                        .map(|u| u.gpu as f64 / 100.0)
                        .unwrap_or(0.5);

                    let temp = device
                        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                        .unwrap_or(45) as f64;

                    let mem = device.memory_info().ok();
                    let memory_used = mem.as_ref().map(|m| m.used).unwrap_or(2_000_000_000);
                    let memory_total = mem.as_ref().map(|m| m.total).unwrap_or(8_000_000_000);

                    let power = device
                        .power_usage()
                        .ok()
                        .map(|p| p as f64 / 1000.0)
                        .unwrap_or(150.0);

                    return GpuMetrics {
                        load: gpu_load,
                        temperature: temp,
                        memory_used,
                        memory_total,
                        power_draw: power,
                    };
                }
            }
        }
        // Fallback when feature disabled or NVML unavailable
        GpuMetrics {
            load: 0.5,
            temperature: 45.0,
            memory_used: 2_000_000_000,
            memory_total: 8_000_000_000,
            power_draw: 150.0,
        }
    }

    /// Get current thermal metrics
    pub fn get_thermal_metrics(&self) -> ThermalMetrics {
        const UNMEASURED_PLACEHOLDER_C: f64 = 25.0;

        let cpu_reading = self.get_cpu_temperature();
        let gpu_reading = self.get_gpu_temperature();
        let cpu_measured = cpu_reading.is_some();
        let gpu_measured = gpu_reading.is_some();
        let cpu_temp = cpu_reading.unwrap_or(UNMEASURED_PLACEHOLDER_C);
        let gpu_temp = gpu_reading.unwrap_or(UNMEASURED_PLACEHOLDER_C);
        let max_temp = cpu_temp.max(gpu_temp);

        // An unmeasured reading is reported as Unknown rather than derived
        // from the placeholder temperature, so a missing sensor can never be
        // silently mistaken for a real "everything is cool" measurement.
        let cpu_status = if cpu_measured {
            ThermalStatus::from_temperature(cpu_temp)
        } else {
            ThermalStatus::Unknown
        };
        let gpu_status = if gpu_measured {
            ThermalStatus::from_temperature(gpu_temp)
        } else {
            ThermalStatus::Unknown
        };

        let throttling_active = cpu_status.should_throttle() || gpu_status.should_throttle();

        ThermalMetrics {
            cpu_temperature: cpu_temp,
            cpu_status,
            cpu_measured,
            gpu_temperature: gpu_temp,
            gpu_status,
            gpu_measured,
            max_temperature: max_temp,
            throttling_active,
        }
    }

    /// Get CPU temperature from hwmon (Linux) or Windows APIs.
    /// `None` when no real sensor reading is available.
    pub fn get_cpu_temperature(&self) -> Option<f64> {
        if self.use_hwmon {
            self.get_cpu_temperature_hwmon()
        } else {
            self.get_cpu_temperature_windows()
        }
    }

    fn get_cpu_temperature_hwmon(&self) -> Option<f64> {
        // Read from /sys/class/thermal/thermal_zone0/temp
        // Returns temperature in millidegrees Celsius
        let content = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp").ok()?;
        let temp_millidegrees: f64 = content.trim().parse().ok()?;
        Some(temp_millidegrees / 1000.0) // Convert to Celsius
    }

    fn get_cpu_temperature_windows(&self) -> Option<f64> {
        // No Windows thermal API implementation yet (would use the `wmi` or
        // `winapi` crate to read an ACPI thermal zone, where populated).
        // Returning None rather than a fabricated value keeps callers from
        // mistaking an unmeasured system for a genuinely cool one.
        None
    }

    /// Get GPU temperature. `None` when no real sensor reading is available
    /// (NVML unavailable, or the `gpu-metrics` feature is disabled).
    pub fn get_gpu_temperature(&self) -> Option<f64> {
        if self.use_nvml {
            self.get_gpu_temperature_nvidia()
        } else {
            None
        }
    }

    fn get_gpu_temperature_nvidia(&self) -> Option<f64> {
        #[cfg(feature = "gpu-metrics")]
        {
            let nvml = nvml_wrapper::Nvml::init().ok()?;
            let device = nvml.device_by_index(self._nvml_device_index).ok()?;
            let temp = device
                .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                .ok()?;
            return Some(temp as f64);
        }
        #[cfg(not(feature = "gpu-metrics"))]
        None
    }

    /// Determine if system should throttle compute
    pub fn should_throttle(&self) -> bool {
        let thermal = self.get_thermal_metrics();
        thermal.throttling_active
    }

    /// Get throttle factor (0.0-1.0) to reduce workload
    pub fn get_throttle_factor(&self) -> f64 {
        let thermal = self.get_thermal_metrics();

        // Use the more restrictive throttle factor from CPU or GPU
        let cpu_factor = thermal.cpu_status.throttle_factor();
        let gpu_factor = thermal.gpu_status.throttle_factor();

        cpu_factor.min(gpu_factor)
    }

    /// Check if thermal situation requires immediate action
    pub fn is_thermal_emergency(&self) -> bool {
        let thermal = self.get_thermal_metrics();
        thermal.cpu_status == ThermalStatus::Critical
            || thermal.gpu_status == ThermalStatus::Critical
    }

    // FIX #5: NEW - Load-based backpressure methods
    /// Check if system should reject new tasks (backpressure)
    pub fn should_reject_new_tasks(&self) -> bool {
        let thermal = self.get_thermal_metrics();
        let gpu = self.get_gpu_metrics();

        // Reject if:
        // 1. Thermal critical
        if thermal.cpu_status == ThermalStatus::Critical {
            println!("[SystemMetrics] FIX #5 BACKPRESSURE: CPU thermal critical - rejecting tasks");
            return true;
        }

        // 2. GPU critical
        if thermal.gpu_status == ThermalStatus::Critical {
            println!("[SystemMetrics] FIX #5 BACKPRESSURE: GPU thermal critical - rejecting tasks");
            return true;
        }

        // 3. GPU memory > 85%
        if gpu.memory_total > 0 {
            let mem_percent = (gpu.memory_used as f64 / gpu.memory_total as f64) * 100.0;
            if mem_percent > 85.0 {
                println!(
                    "[SystemMetrics] FIX #5 BACKPRESSURE: GPU memory {:.1}% - rejecting tasks",
                    mem_percent
                );
                return true;
            }
        }

        // 4. GPU load > 95%
        if gpu.load > 0.95 {
            println!(
                "[SystemMetrics] FIX #5 BACKPRESSURE: GPU load {:.1}% - rejecting tasks",
                gpu.load * 100.0
            );
            return true;
        }

        false // Safe to accept
    }

    /// Get backpressure level (0.0-1.0, where 1.0 = full rejection)
    pub fn get_backpressure_level(&self) -> f64 {
        let thermal = self.get_thermal_metrics();
        let gpu = self.get_gpu_metrics();

        let mut pressure: f64 = 0.0;

        // Thermal pressure (0-0.5)
        match thermal.cpu_status {
            ThermalStatus::Cool | ThermalStatus::Normal => pressure += 0.0,
            // Unmeasured gets the same backpressure contribution as Warm,
            // consistent with should_throttle()/throttle_factor() treating an
            // unreadable sensor as mild caution rather than a clean bill of
            // health (owner decision, C65 review).
            ThermalStatus::Warm | ThermalStatus::Unknown => pressure += 0.1,
            ThermalStatus::Hot => pressure += 0.3,
            ThermalStatus::Critical => pressure += 0.5,
        }

        // GPU memory pressure (0-0.3)
        if gpu.memory_total > 0 {
            let mem_percent = gpu.memory_used as f64 / gpu.memory_total as f64;
            if mem_percent > 0.85 {
                pressure += 0.3; // Critical
            } else if mem_percent > 0.75 {
                pressure += 0.2; // High
            } else if mem_percent > 0.65 {
                pressure += 0.1; // Moderate
            }
        }

        // GPU load pressure (0-0.2)
        if gpu.load > 0.90 {
            pressure += 0.2;
        } else if gpu.load > 0.75 {
            pressure += 0.1;
        }

        pressure.min(1.0_f64)
    }

    /// Get recommended task deferral probability (0.0-1.0)
    pub fn get_deferral_probability(&self) -> f64 {
        self.get_backpressure_level()
    }
}

impl Default for SystemMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thermal_status_from_temperature() {
        assert_eq!(ThermalStatus::from_temperature(30.0), ThermalStatus::Cool);
        assert_eq!(ThermalStatus::from_temperature(60.0), ThermalStatus::Normal);
        assert_eq!(ThermalStatus::from_temperature(80.0), ThermalStatus::Warm);
        assert_eq!(ThermalStatus::from_temperature(90.0), ThermalStatus::Hot);
        assert_eq!(
            ThermalStatus::from_temperature(100.0),
            ThermalStatus::Critical
        );
    }

    #[test]
    fn test_thermal_status_should_throttle() {
        assert!(!ThermalStatus::Cool.should_throttle());
        assert!(!ThermalStatus::Normal.should_throttle());
        assert!(!ThermalStatus::Warm.should_throttle());
        assert!(ThermalStatus::Hot.should_throttle());
        assert!(ThermalStatus::Critical.should_throttle());
        // An unreadable sensor is not the same as a verified-safe one: it
        // must not be treated as no-throttle (owner decision, C65 review).
        assert!(ThermalStatus::Unknown.should_throttle());
    }

    #[test]
    fn test_throttle_factors() {
        assert_eq!(ThermalStatus::Cool.throttle_factor(), 1.0);
        assert_eq!(ThermalStatus::Normal.throttle_factor(), 1.0);
        assert_eq!(ThermalStatus::Warm.throttle_factor(), 0.9);
        assert_eq!(ThermalStatus::Hot.throttle_factor(), 0.7);
        assert_eq!(ThermalStatus::Critical.throttle_factor(), 0.5);
        // Unmeasured gets Warm-equivalent caution, not a free pass.
        assert_eq!(ThermalStatus::Unknown.throttle_factor(), 0.9);
    }

    #[test]
    fn test_metrics_collector_creation() {
        let collector = SystemMetricsCollector::new();

        let gpu = collector.get_gpu_metrics();
        assert!(gpu.load >= 0.0 && gpu.load <= 1.0);

        let thermal = collector.get_thermal_metrics();
        assert!(thermal.cpu_temperature > 0.0);
    }

    #[test]
    fn test_unmeasured_temperature_reports_unknown_not_a_fake_reading() {
        // Without hwmon (e.g. on Windows, or a machine with no
        // /sys/class/thermal/thermal_zone0) and without the gpu-metrics
        // feature, neither sensor is real. The collector must say so via
        // Unknown/*_measured rather than silently reporting a placeholder
        // temperature as a genuine "everything is cool" reading.
        let collector = SystemMetricsCollector::new();
        let thermal = collector.get_thermal_metrics();

        if !thermal.cpu_measured {
            assert_eq!(thermal.cpu_status, ThermalStatus::Unknown);
        }
        if !thermal.gpu_measured {
            assert_eq!(thermal.gpu_status, ThermalStatus::Unknown);
        }
    }

    #[test]
    fn test_thermal_emergency_detection() {
        let collector = SystemMetricsCollector::new();
        // Should not be in emergency under normal conditions
        assert!(!collector.is_thermal_emergency());
    }
}
