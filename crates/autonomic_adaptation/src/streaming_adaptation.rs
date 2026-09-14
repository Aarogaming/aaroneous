// crates/autonomic_adaptation/src/streaming_adaptation.rs
//! Streaming Self-Correction & Autonomous Pacing Regulation.
//!
//! Provides real-time thermodynamic pacing regulation, non-blocking telemetry evaluation,
//! and streaming drift self-correction for the hypervisor supervisory loop.
//!
//! Enforces:
//! 1. Zero-Heap Hot Paths: Pure stack calculations over numeric scalars and `Duration`.
//! 2. Zero Panics: Structured `Result<T, AdaptationError>` return types without `.unwrap()` or `.expect()`.
//! 3. Thread Starvation Prevention: Hard min/max interval bounding ensuring loop liveness even under extreme thermal throttle.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

/// Error variants encountered during streaming adaptation and pacing regulation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdaptationError {
    #[error("Thermodynamic telemetry contains invalid NaN or infinite values: {0}")]
    InvalidTelemetry(&'static str),
    #[error("Configuration invariant violated: {0}")]
    InvalidConfiguration(&'static str),
}

/// Point-in-time thermodynamic hardware telemetry input.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThermodynamicTelemetry {
    /// Host CPU load percentage (0.0 to 100.0)
    pub cpu_load_pct: f32,
    /// Host CPU temperature in degrees Celsius
    pub cpu_temp_c: f32,
    /// Host GPU temperature in degrees Celsius
    pub gpu_temp_c: f32,
    /// Dedicated GPU VRAM currently in use (bytes)
    pub vram_used_bytes: u64,
    /// Total available GPU VRAM (bytes)
    pub vram_total_bytes: u64,
    /// System RAM memory pressure percentage (0.0 to 100.0)
    pub memory_pressure_pct: f32,
}

impl Default for ThermodynamicTelemetry {
    fn default() -> Self {
        Self {
            cpu_load_pct: 10.0,
            cpu_temp_c: 45.0,
            gpu_temp_c: 42.0,
            vram_used_bytes: 1024 * 1024 * 1024,      // 1 GB
            vram_total_bytes: 8 * 1024 * 1024 * 1024, // 8 GB
            memory_pressure_pct: 25.0,
        }
    }
}

impl ThermodynamicTelemetry {
    /// Calculate current VRAM utilization percentage (0.0 to 100.0).
    pub fn vram_pressure_pct(&self) -> f32 {
        if self.vram_total_bytes == 0 {
            0.0
        } else {
            ((self.vram_used_bytes as f64 / self.vram_total_bytes as f64) * 100.0) as f32
        }
    }

    /// Return peak system temperature across CPU and GPU.
    pub fn max_temperature(&self) -> f32 {
        self.cpu_temp_c.max(self.gpu_temp_c)
    }

    /// Calculate instantaneous composite hardware stress index (normalized 0.0 to 1.0).
    ///
    /// Weighted heuristic:
    /// - 45% Thermal headroom: 50°C (0.0) to 95°C (1.0)
    /// - 35% VRAM pressure: 0% to 100%
    /// - 20% CPU & Memory pressure: average load 0% to 100%
    pub fn composite_stress_index(&self) -> f32 {
        let max_t = self.max_temperature();
        let thermal_component = ((max_t - 50.0) / 45.0).clamp(0.0, 1.0);
        let vram_component = (self.vram_pressure_pct() / 100.0).clamp(0.0, 1.0);
        let cpu_component = (self.cpu_load_pct / 100.0).clamp(0.0, 1.0);
        let mem_component = (self.memory_pressure_pct / 100.0).clamp(0.0, 1.0);
        let sys_load = (cpu_component + mem_component) * 0.5;

        (0.45 * thermal_component + 0.35 * vram_component + 0.20 * sys_load).clamp(0.0, 1.0)
    }

    /// Validate that all telemetry scalar metrics are finite numbers.
    pub fn validate(&self) -> Result<(), AdaptationError> {
        if !self.cpu_load_pct.is_finite() {
            return Err(AdaptationError::InvalidTelemetry(
                "cpu_load_pct is non-finite",
            ));
        }
        if !self.cpu_temp_c.is_finite() {
            return Err(AdaptationError::InvalidTelemetry(
                "cpu_temp_c is non-finite",
            ));
        }
        if !self.gpu_temp_c.is_finite() {
            return Err(AdaptationError::InvalidTelemetry(
                "gpu_temp_c is non-finite",
            ));
        }
        if !self.memory_pressure_pct.is_finite() {
            return Err(AdaptationError::InvalidTelemetry(
                "memory_pressure_pct is non-finite",
            ));
        }
        Ok(())
    }
}

/// Operational tier regulating task scheduling pacing and telemetry decimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacingTier {
    /// Full performance target: baseline cadence (e.g. 120 FPS / 8ms, or 10ms - 16ms).
    Nominal,
    /// Moderate load: 2.0x cadence backoff, decimation factor 2.
    MetabolicThrottle,
    /// Severe thermal (>80°C) or VRAM (>85%) pressure: 4.0x cadence backoff, decimation factor 4.
    ThermalCritical,
    /// Emergency limit (>90°C): 10.0x cadence backoff, safety locks engaged, full deferral.
    DormantPreservation,
}

impl PacingTier {
    /// Integer cadence multiplier over baseline interval.
    pub fn multiplier(&self) -> u32 {
        match self {
            Self::Nominal => 1,
            Self::MetabolicThrottle => 2,
            Self::ThermalCritical => 4,
            Self::DormantPreservation => 10,
        }
    }

    /// Sampling decimation factor for periodic expensive background scans.
    pub fn decimation_factor(&self) -> u32 {
        match self {
            Self::Nominal => 1,
            Self::MetabolicThrottle => 2,
            Self::ThermalCritical => 4,
            Self::DormantPreservation => 8,
        }
    }

    /// Recommended task deferral probability for non-critical work (0.0 to 1.0).
    pub fn task_deferral_probability(&self) -> f32 {
        match self {
            Self::Nominal => 0.0,
            Self::MetabolicThrottle => 0.25,
            Self::ThermalCritical => 0.75,
            Self::DormantPreservation => 1.0,
        }
    }
}

/// Dynamic pacing decision emitted per adaptation cycle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PacingDecision {
    /// Target wall-clock duration between execution ticks
    pub target_cadence: Duration,
    /// Active operational pacing tier
    pub throttle_tier: PacingTier,
    /// Telemetry and background work decimation factor
    pub decimation_factor: u32,
    /// Probability to defer non-critical task batches
    pub task_deferral_probability: f32,
    /// Exponentially smoothed composite hardware stress (0.0 to 1.0)
    pub composite_stress: f32,
}

/// Configuration parameters for the autonomous pacing regulator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PacingConfig {
    /// Normal unthrottled loop cadence (e.g. 10ms or 16ms)
    pub baseline_interval: Duration,
    /// Absolute lower bound for execution ticks (prevent spinning)
    pub min_interval: Duration,
    /// Absolute upper bound for execution ticks (prevent starvation/unresponsiveness)
    pub max_interval: Duration,
    /// Thermal threshold for entering `MetabolicThrottle` (Celsius)
    pub thermal_warning_c: f32,
    /// Thermal threshold for entering `ThermalCritical` (Celsius)
    pub thermal_critical_c: f32,
    /// Thermal threshold for entering `DormantPreservation` (Celsius)
    pub thermal_emergency_c: f32,
    /// VRAM utilization threshold for entering `MetabolicThrottle` (%)
    pub vram_warning_pct: f32,
    /// VRAM utilization threshold for entering `ThermalCritical` (%)
    pub vram_critical_pct: f32,
    /// Exponential Weighted Moving Average smoothing coefficient (0.0 to 1.0)
    pub ewma_alpha: f32,
}

impl Default for PacingConfig {
    fn default() -> Self {
        Self {
            baseline_interval: Duration::from_millis(16), // ~60 Hz
            min_interval: Duration::from_millis(4),       // ~250 Hz cap
            max_interval: Duration::from_millis(500),     // 2 Hz floor (guarantees liveness)
            thermal_warning_c: 75.0,
            thermal_critical_c: 85.0,
            thermal_emergency_c: 95.0,
            vram_warning_pct: 75.0,
            vram_critical_pct: 90.0,
            ewma_alpha: 0.25,
        }
    }
}

impl PacingConfig {
    /// Construct a configuration using a specified baseline interval.
    pub fn default_with_baseline(baseline: Duration) -> Self {
        Self {
            baseline_interval: baseline,
            min_interval: baseline.min(Duration::from_millis(4)),
            max_interval: (baseline * 10).max(Duration::from_millis(500)),
            ..Self::default()
        }
    }

    /// Validate configuration invariants.
    pub fn validate(&self) -> Result<(), AdaptationError> {
        if self.min_interval > self.baseline_interval {
            return Err(AdaptationError::InvalidConfiguration(
                "min_interval cannot exceed baseline_interval",
            ));
        }
        if self.baseline_interval > self.max_interval {
            return Err(AdaptationError::InvalidConfiguration(
                "baseline_interval cannot exceed max_interval",
            ));
        }
        if self.thermal_warning_c >= self.thermal_critical_c {
            return Err(AdaptationError::InvalidConfiguration(
                "thermal_warning_c must be strictly less than thermal_critical_c",
            ));
        }
        if self.thermal_critical_c >= self.thermal_emergency_c {
            return Err(AdaptationError::InvalidConfiguration(
                "thermal_critical_c must be strictly less than thermal_emergency_c",
            ));
        }
        if self.vram_warning_pct >= self.vram_critical_pct {
            return Err(AdaptationError::InvalidConfiguration(
                "vram_warning_pct must be strictly less than vram_critical_pct",
            ));
        }
        if !(0.0..=1.0).contains(&self.ewma_alpha) {
            return Err(AdaptationError::InvalidConfiguration(
                "ewma_alpha must be between 0.0 and 1.0",
            ));
        }
        Ok(())
    }
}

/// Autonomous pacing regulator maintaining thermodynamic state and cadence modulation.
#[derive(Debug, Clone)]
pub struct AutonomousPacingRegulator {
    config: PacingConfig,
    smoothed_stress: f32,
    current_cadence: Duration,
    current_tier: PacingTier,
    cycle_count: u64,
}

impl AutonomousPacingRegulator {
    /// Create a new regulator with specified configuration.
    pub fn new(config: PacingConfig) -> Result<Self, AdaptationError> {
        config.validate()?;
        let cadence = config.baseline_interval;
        Ok(Self {
            config,
            smoothed_stress: 0.0,
            current_cadence: cadence,
            current_tier: PacingTier::Nominal,
            cycle_count: 0,
        })
    }

    /// Create default regulator using a baseline loop cadence.
    pub fn default_with_baseline(baseline: Duration) -> Self {
        let config = PacingConfig::default_with_baseline(baseline);
        Self {
            config,
            smoothed_stress: 0.0,
            current_cadence: baseline,
            current_tier: PacingTier::Nominal,
            cycle_count: 0,
        }
    }

    /// Ingest telemetry and compute the next operational pacing decision.
    ///
    /// Evaluates thermodynamic stress, updates EWMA smoothing, chooses the active
    /// throttle tier, and bounds the target interval against starvation limits.
    pub fn compute_next_cadence(
        &mut self,
        telemetry: &ThermodynamicTelemetry,
    ) -> Result<PacingDecision, AdaptationError> {
        telemetry.validate()?;

        let instant_stress = telemetry.composite_stress_index();

        // Update EWMA smoothed stress
        if self.cycle_count == 0 {
            self.smoothed_stress = instant_stress;
        } else {
            self.smoothed_stress = (self.config.ewma_alpha * instant_stress)
                + ((1.0 - self.config.ewma_alpha) * self.smoothed_stress);
        }
        self.cycle_count = self.cycle_count.saturating_add(1);

        let max_temp = telemetry.max_temperature();
        let vram_pct = telemetry.vram_pressure_pct();

        // State machine transition logic
        let tier = if max_temp >= self.config.thermal_emergency_c {
            PacingTier::DormantPreservation
        } else if max_temp >= self.config.thermal_critical_c
            || vram_pct >= self.config.vram_critical_pct
            || self.smoothed_stress >= 0.85
        {
            PacingTier::ThermalCritical
        } else if max_temp >= self.config.thermal_warning_c
            || vram_pct >= self.config.vram_warning_pct
            || self.smoothed_stress >= 0.50
        {
            PacingTier::MetabolicThrottle
        } else {
            PacingTier::Nominal
        };

        // Calculate target interval using tier multiplier
        let unconstrained_interval = self
            .config
            .baseline_interval
            .saturating_mul(tier.multiplier());

        // Bounded interval preventing starvation or runaway frequency
        let target_cadence = unconstrained_interval
            .max(self.config.min_interval)
            .min(self.config.max_interval);

        self.current_cadence = target_cadence;
        self.current_tier = tier;

        Ok(PacingDecision {
            target_cadence,
            throttle_tier: tier,
            decimation_factor: tier.decimation_factor(),
            task_deferral_probability: tier.task_deferral_probability(),
            composite_stress: self.smoothed_stress,
        })
    }

    /// Current target cadence.
    pub fn current_cadence(&self) -> Duration {
        self.current_cadence
    }

    /// Current operational tier.
    pub fn current_tier(&self) -> PacingTier {
        self.current_tier
    }

    /// Current smoothed stress estimate.
    pub fn smoothed_stress(&self) -> f32 {
        self.smoothed_stress
    }

    /// Current total evaluation cycle count.
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count
    }

    /// Reset baseline interval dynamically.
    pub fn reset_baseline(&mut self, new_baseline: Duration) {
        self.config.baseline_interval = new_baseline;
        self.config.min_interval = new_baseline.min(Duration::from_millis(4));
        self.config.max_interval = (new_baseline * 10).max(Duration::from_millis(500));
        self.current_cadence = new_baseline.saturating_mul(self.current_tier.multiplier());
    }

    /// Access underlying configuration.
    pub fn config(&self) -> &PacingConfig {
        &self.config
    }
}

/// Streaming Self-Correction Filter.
///
/// Tracks loop clock drift between target cadence and actual tick execution time,
/// applying an integral correction offset to prevent cumulative timing skew.
#[derive(Debug, Clone)]
pub struct StreamingSelfCorrectionFilter {
    integral_error_us: i64,
    integral_gain: f32,
    max_integral_us: i64,
}

impl Default for StreamingSelfCorrectionFilter {
    fn default() -> Self {
        Self::new(0.1, 50_000) // 50ms max accumulated correction
    }
}

impl StreamingSelfCorrectionFilter {
    /// Create a new drift corrector with specified integral gain and max bias cap.
    pub fn new(integral_gain: f32, max_integral_us: i64) -> Self {
        Self {
            integral_error_us: 0,
            integral_gain: integral_gain.clamp(0.01, 1.0),
            max_integral_us: max_integral_us.abs(),
        }
    }

    /// Compute corrected sleep duration based on measured elapsed tick time vs target cadence.
    pub fn compute_sleep_duration(
        &mut self,
        measured_elapsed: Duration,
        target_cadence: Duration,
    ) -> Duration {
        let elapsed_us = measured_elapsed.as_micros() as i64;
        let target_us = target_cadence.as_micros() as i64;

        // Error: positive if tick took longer than target (lag), negative if faster
        let error_us = elapsed_us - target_us;

        // Accumulate integral error with saturation clamp
        self.integral_error_us =
            (self.integral_error_us + error_us).clamp(-self.max_integral_us, self.max_integral_us);

        // Correction offset
        let correction_us = (self.integral_error_us as f32 * self.integral_gain) as i64;

        // Remaining nominal sleep before correction
        let nominal_sleep_us = target_us.saturating_sub(elapsed_us);

        // Net corrected sleep time
        let net_sleep_us = (nominal_sleep_us - correction_us).max(0);

        Duration::from_micros(net_sleep_us as u64)
    }

    /// Reset accumulated drift state.
    pub fn reset(&mut self) {
        self.integral_error_us = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_stress_and_vram_calculation() {
        let mut telemetry = ThermodynamicTelemetry {
            cpu_load_pct: 20.0,
            cpu_temp_c: 45.0,
            gpu_temp_c: 45.0,
            vram_used_bytes: 4 * 1024 * 1024 * 1024,
            vram_total_bytes: 8 * 1024 * 1024 * 1024,
            memory_pressure_pct: 30.0,
        };

        assert_eq!(telemetry.vram_pressure_pct(), 50.0);
        assert_eq!(telemetry.max_temperature(), 45.0);
        let stress = telemetry.composite_stress_index();
        assert!(
            stress >= 0.0 && stress <= 0.5,
            "Expected low stress, got {}",
            stress
        );

        // Test VRAM saturation
        telemetry.vram_used_bytes = 8 * 1024 * 1024 * 1024;
        assert_eq!(telemetry.vram_pressure_pct(), 100.0);
        let high_stress = telemetry.composite_stress_index();
        assert!(high_stress > stress);
    }

    #[test]
    fn test_nominal_pacing_decision() {
        let config = PacingConfig {
            baseline_interval: Duration::from_millis(10),
            min_interval: Duration::from_millis(2),
            max_interval: Duration::from_millis(200),
            thermal_warning_c: 75.0,
            thermal_critical_c: 85.0,
            thermal_emergency_c: 95.0,
            vram_warning_pct: 75.0,
            vram_critical_pct: 90.0,
            ewma_alpha: 0.5,
        };

        let mut regulator = AutonomousPacingRegulator::new(config).expect("valid config");

        let nominal_telemetry = ThermodynamicTelemetry {
            cpu_load_pct: 15.0,
            cpu_temp_c: 48.0,
            gpu_temp_c: 50.0,
            vram_used_bytes: 2 * 1024 * 1024 * 1024,
            vram_total_bytes: 8 * 1024 * 1024 * 1024,
            memory_pressure_pct: 20.0,
        };

        let decision = regulator
            .compute_next_cadence(&nominal_telemetry)
            .expect("pacing calculation");

        assert_eq!(decision.throttle_tier, PacingTier::Nominal);
        assert_eq!(decision.target_cadence, Duration::from_millis(10));
        assert_eq!(decision.decimation_factor, 1);
        assert_eq!(decision.task_deferral_probability, 0.0);
    }

    #[test]
    fn test_thermal_throttle_and_critical_transitions() {
        let mut regulator =
            AutonomousPacingRegulator::default_with_baseline(Duration::from_millis(10));

        // 1. Moderate thermal warning (78°C)
        let warning_telemetry = ThermodynamicTelemetry {
            cpu_temp_c: 78.0,
            gpu_temp_c: 72.0,
            ..Default::default()
        };
        let decision1 = regulator
            .compute_next_cadence(&warning_telemetry)
            .expect("cadence");
        assert_eq!(decision1.throttle_tier, PacingTier::MetabolicThrottle);
        assert_eq!(decision1.target_cadence, Duration::from_millis(20)); // 2x baseline
        assert_eq!(decision1.decimation_factor, 2);
        assert_eq!(decision1.task_deferral_probability, 0.25);

        // 2. Critical thermal spike (88°C)
        let critical_telemetry = ThermodynamicTelemetry {
            cpu_temp_c: 88.0,
            gpu_temp_c: 82.0,
            ..Default::default()
        };
        let decision2 = regulator
            .compute_next_cadence(&critical_telemetry)
            .expect("cadence");
        assert_eq!(decision2.throttle_tier, PacingTier::ThermalCritical);
        assert_eq!(decision2.target_cadence, Duration::from_millis(40)); // 4x baseline
        assert_eq!(decision2.decimation_factor, 4);
        assert_eq!(decision2.task_deferral_probability, 0.75);

        // 3. Emergency preservation limit (96°C)
        let emergency_telemetry = ThermodynamicTelemetry {
            cpu_temp_c: 96.0,
            gpu_temp_c: 85.0,
            ..Default::default()
        };
        let decision3 = regulator
            .compute_next_cadence(&emergency_telemetry)
            .expect("cadence");
        assert_eq!(decision3.throttle_tier, PacingTier::DormantPreservation);
        assert_eq!(decision3.target_cadence, Duration::from_millis(100)); // 10x baseline
        assert_eq!(decision3.decimation_factor, 8);
        assert_eq!(decision3.task_deferral_probability, 1.0);
    }

    #[test]
    fn test_vram_saturation_triggers_critical() {
        let mut regulator =
            AutonomousPacingRegulator::default_with_baseline(Duration::from_millis(10));

        let vram_telemetry = ThermodynamicTelemetry {
            cpu_temp_c: 50.0,
            gpu_temp_c: 50.0,
            vram_used_bytes: 7500 * 1024 * 1024,
            vram_total_bytes: 8000 * 1024 * 1024, // 93.75%
            ..Default::default()
        };

        let decision = regulator
            .compute_next_cadence(&vram_telemetry)
            .expect("cadence");
        assert_eq!(decision.throttle_tier, PacingTier::ThermalCritical);
        assert_eq!(decision.target_cadence, Duration::from_millis(40));
    }

    #[test]
    fn test_thread_starvation_prevention_clamping() {
        let mut config = PacingConfig::default_with_baseline(Duration::from_millis(50));
        config.max_interval = Duration::from_millis(120); // strict upper bound
        let mut regulator = AutonomousPacingRegulator::new(config).expect("valid config");

        // Emergency would normally demand 10x (500ms), but max_interval must clamp it to 120ms
        let emergency_telemetry = ThermodynamicTelemetry {
            cpu_temp_c: 98.0,
            ..Default::default()
        };
        let decision = regulator
            .compute_next_cadence(&emergency_telemetry)
            .expect("cadence");
        assert_eq!(decision.target_cadence, Duration::from_millis(120));
    }

    #[test]
    fn test_pacing_recovery_to_nominal() {
        let mut regulator =
            AutonomousPacingRegulator::default_with_baseline(Duration::from_millis(10));

        // Spike temperature
        let hot = ThermodynamicTelemetry {
            cpu_temp_c: 90.0,
            ..Default::default()
        };
        let _ = regulator.compute_next_cadence(&hot).expect("cadence");
        assert_eq!(regulator.current_tier(), PacingTier::ThermalCritical);

        // System cools down back to 40°C
        let cool = ThermodynamicTelemetry {
            cpu_temp_c: 40.0,
            gpu_temp_c: 40.0,
            cpu_load_pct: 10.0,
            vram_used_bytes: 1024 * 1024,
            vram_total_bytes: 8 * 1024 * 1024 * 1024,
            memory_pressure_pct: 10.0,
        };

        // Multiple cool cycles dissipate EWMA smoothed stress
        for _ in 0..10 {
            let _ = regulator.compute_next_cadence(&cool).expect("cadence");
        }

        assert_eq!(regulator.current_tier(), PacingTier::Nominal);
        assert_eq!(regulator.current_cadence(), Duration::from_millis(10));
    }

    #[test]
    fn test_self_correction_filter_lag_compensation() {
        let mut filter = StreamingSelfCorrectionFilter::default();
        let target = Duration::from_millis(16);

        // If tick took 20ms (4ms lag), corrected sleep should be reduced to 0 to catch up
        let sleep = filter.compute_sleep_duration(Duration::from_millis(20), target);
        assert_eq!(sleep, Duration::from_millis(0));

        // If tick took 10ms (6ms faster than target), corrected sleep should be nominal (~6ms)
        let sleep2 = filter.compute_sleep_duration(Duration::from_millis(10), target);
        assert!(sleep2.as_millis() >= 5 && sleep2.as_millis() <= 7);
    }
}
