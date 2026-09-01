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
        matches!(self, ThermalStatus::Hot | ThermalStatus::Critical)
    }

    pub fn throttle_factor(&self) -> f64 {
        match self {
            ThermalStatus::Cool => 1.0,
            ThermalStatus::Normal => 1.0,
            ThermalStatus::Warm => 0.9,
            ThermalStatus::Hot => 0.7,
            ThermalStatus::Critical => 0.5,
            ThermalStatus::Unknown => 1.0,
        }
    }
}

/// System thermal metrics
#[derive(Debug, Clone)]
pub struct ThermalMetrics {
    pub cpu_temperature: f64, // degrees Celsius
    pub cpu_status: ThermalStatus,
    pub gpu_temperature: f64,
    pub gpu_status: ThermalStatus,
    pub max_temperature: f64,
    pub throttling_active: bool,
}

impl Default for ThermalMetrics {
    fn default() -> Self {
        Self {
            cpu_temperature: 25.0,
            cpu_status: ThermalStatus::Normal,
            gpu_temperature: 25.0,
            gpu_status: ThermalStatus::Normal,
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
                    let gpu_load = utilization.as_ref().map(|u| u.gpu as f64 / 100.0).unwrap_or(0.5);

                    let temp = device
                        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                        .unwrap_or(45) as f64;

                    let mem = device.memory_info().ok();
                    let memory_used = mem.as_ref().map(|m| m.used).unwrap_or(2_000_000_000);
                    let memory_total = mem.as_ref().map(|m| m.total).unwrap_or(8_000_000_000);

                    let power = device.power_usage().ok().map(|p| p as f64 / 1000.0).unwrap_or(150.0);

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
        let cpu_temp = self.get_cpu_temperature();
        let gpu_temp = self.get_gpu_temperature();
        let max_temp = cpu_temp.max(gpu_temp);

        let cpu_status = ThermalStatus::from_temperature(cpu_temp);
        let gpu_status = ThermalStatus::from_temperature(gpu_temp);

        let throttling_active = cpu_status.should_throttle() || gpu_status.should_throttle();

        ThermalMetrics {
            cpu_temperature: cpu_temp,
            cpu_status,
            gpu_temperature: gpu_temp,
            gpu_status,
            max_temperature: max_temp,
            throttling_active,
        }
    }

    /// Get CPU temperature from hwmon (Linux) or Windows APIs
    pub fn get_cpu_temperature(&self) -> f64 {
        if self.use_hwmon {
            self.get_cpu_temperature_hwmon()
        } else {
            self.get_cpu_temperature_windows()
        }
    }

    fn get_cpu_temperature_hwmon(&self) -> f64 {
        // Read from /sys/class/thermal/thermal_zone0/temp
        // Returns temperature in millidegrees Celsius
        match fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
            Ok(content) => {
                let temp_millidegrees: f64 = content.trim().parse().unwrap_or(25000.0);
                temp_millidegrees / 1000.0 // Convert to Celsius
            }
            Err(_) => 25.0, // Default fallback
        }
    }

    fn get_cpu_temperature_windows(&self) -> f64 {
        // Production Windows: Use WMI or Windows thermal APIs
        // For now, return default value
        // Example would use winapi or wmi crate
        25.0
    }

    /// Get GPU temperature
    pub fn get_gpu_temperature(&self) -> f64 {
        if self.use_nvml {
            self.get_gpu_temperature_nvidia()
        } else {
            25.0 // Default fallback
        }
    }

    fn get_gpu_temperature_nvidia(&self) -> f64 {
        #[cfg(feature = "gpu-metrics")]
        {
            if let Ok(nvml) = nvml_wrapper::Nvml::init() {
                if let Ok(device) = nvml.device_by_index(self._nvml_device_index) {
                    return device
                        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                        .unwrap_or(45) as f64;
                }
            }
        }
        45.0
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
            ThermalStatus::Warm => pressure += 0.1,
            ThermalStatus::Hot => pressure += 0.3,
            ThermalStatus::Critical => pressure += 0.5,
            ThermalStatus::Unknown => pressure += 0.0,
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
    }

    #[test]
    fn test_throttle_factors() {
        assert_eq!(ThermalStatus::Cool.throttle_factor(), 1.0);
        assert_eq!(ThermalStatus::Normal.throttle_factor(), 1.0);
        assert_eq!(ThermalStatus::Warm.throttle_factor(), 0.9);
        assert_eq!(ThermalStatus::Hot.throttle_factor(), 0.7);
        assert_eq!(ThermalStatus::Critical.throttle_factor(), 0.5);
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
    fn test_thermal_emergency_detection() {
        let collector = SystemMetricsCollector::new();
        // Should not be in emergency under normal conditions
        assert!(!collector.is_thermal_emergency());
    }
}
