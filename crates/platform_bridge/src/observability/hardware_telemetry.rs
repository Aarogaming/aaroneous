use anyhow::Result;
use tracing::{info, warn};

#[cfg(feature = "gpu-metrics")]
use nvml_wrapper::Nvml;

/// CONSUMER-05: Raw Hardware Telemetry Extraction
/// Connects to NVIDIA Management Library (NVML) to extract deep VRAM, Temp, and Compute telemetry
/// directly into the SI Engine for adaptive capacity routing.
pub struct HardwareTelemetryExtractor {
    #[cfg(feature = "gpu-metrics")]
    nvml_instance: Option<Nvml>,
}

impl HardwareTelemetryExtractor {
    pub fn new() -> Self {
        #[cfg(feature = "gpu-metrics")]
        {
            let nvml_instance = match Nvml::init() {
                Ok(nvml) => {
                    info!("NVML Initialized successfully for Hardware Telemetry.");
                    Some(nvml)
                }
                Err(e) => {
                    warn!("Failed to initialize NVML (No NVIDIA GPU or drivers missing): {}", e);
                    None
                }
            };

            Self { nvml_instance }
        }

        #[cfg(not(feature = "gpu-metrics"))]
        {
            Self {}
        }
    }

    /// Fetches the VRAM usage (Used / Total in MB) for the primary GPU (Device 0)
    pub fn fetch_vram_usage(&self) -> Result<(u64, u64)> {
        #[cfg(feature = "gpu-metrics")]
        {
            if let Some(nvml) = &self.nvml_instance {
                let device = nvml.device_by_index(0)?;
                let memory = device.memory_info()?;
                // Convert bytes to MB
                let used = memory.used / 1024 / 1024;
                let total = memory.total / 1024 / 1024;
                return Ok((used, total));
            }
        }
        anyhow::bail!("NVML not available or gpu-metrics feature disabled.")
    }

    /// Fetches the current GPU temperature in Celsius
    pub fn fetch_temperature(&self) -> Result<u32> {
        #[cfg(feature = "gpu-metrics")]
        {
            if let Some(nvml) = &self.nvml_instance {
                let device = nvml.device_by_index(0)?;
                let temp = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)?;
                return Ok(temp);
            }
        }
        anyhow::bail!("NVML not available or gpu-metrics feature disabled.")
    }
}