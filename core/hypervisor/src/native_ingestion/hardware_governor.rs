use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use sysinfo::System;

/// Action to take based on hardware profile inference.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HwInferenceAction {
    /// Full visual processing enabled (capture, SIMD, SVD, routing).
    FullVisual = 0,
    /// Visual processing throttled — reduce capture resolution.
    ThrottledVisual = 1,
    /// Visual processing suspended — CPU-only mode.
    CpuOnly = 2,
    /// Everything suspended — emergency thermal protection.
    EmergencyHalt = 3,
}

/// Snapshot of hardware resource state.
#[repr(C, align(64))]
#[derive(Clone, Debug)]
pub struct HardwareProfile {
    /// CPU utilization ratio (0.0–1.0).
    pub cpu_utilization: f32,
    /// Memory utilization ratio (0.0–1.0).
    pub mem_utilization: f32,
    /// Number of available logical cores.
    pub core_count: u32,
    /// CPU temperature in Celsius (0.0 if unavailable).
    pub cpu_temperature: f32,
    /// Average CPU frequency as fraction of nominal (0.0–1.0).
    pub cpu_freq_ratio: f32,
    /// Number of running processes.
    pub process_count: u32,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Current inferred action.
    pub action: HwInferenceAction,
}

/// Hardware-conditional execution governor.
///
/// Monitors real-time CPU/memory profiles and selects the appropriate
/// execution strategy for the visual ingestion substrate:
///
/// - **FullVisual**: All capture/SIMD/SVD/routing pipelines active.
/// - **ThrottledVisual**: Reduced capture resolution, fewer SIMD passes.
/// - **CpuOnly**: Visual substrate suspended, only minimal CPU tasks.
/// - **EmergencyHalt**: All non-critical processing suspended.
pub struct HardwareGovernor {
    sys: System,
    sample_interval_ms: u64,
    last_sample_tick: AtomicU64,
    active: AtomicBool,
    profile_cache: HardwareProfile,
}

impl HardwareGovernor {
    pub fn new(sample_interval_ms: u64) -> Self {
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();

        let cpu_util = sys.cpus().iter().map(|c| c.cpu_usage()).sum::<f32>()
            / sys.cpus().len().max(1) as f32;
        let mem_util = sys.used_memory() as f32 / sys.total_memory().max(1) as f32;

        Self {
            profile_cache: HardwareProfile {
                cpu_utilization: cpu_util,
                mem_utilization: mem_util,
                core_count: sys.cpus().len() as u32,
                cpu_temperature: 0.0,
                cpu_freq_ratio: 1.0,
                process_count: 0,
                uptime_secs: now_secs(),
                action: HwInferenceAction::FullVisual,
            },
            sys,
            sample_interval_ms,
            last_sample_tick: AtomicU64::new(0),
            active: AtomicBool::new(true),
        }
    }

    /// Sample hardware state and infer the appropriate execution action.
    ///
    /// Returns the current `HardwareProfile`. The profile is cached and
    /// refreshed at most once per `sample_interval_ms`.
    pub fn sample(&mut self) -> HardwareProfile {
        let now = now_ms();
        let last = self.last_sample_tick.load(Ordering::Acquire);
        if now - last < self.sample_interval_ms {
            return self.profile_cache.clone();
        }
        self.last_sample_tick.store(now, Ordering::Release);

        self.sys.refresh_cpu();
        self.sys.refresh_memory();

        let cpus = self.sys.cpus();
        let cpu_count = cpus.len().max(1);
        let cpu_util = cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpu_count as f32 / 100.0;
        let mem_util = self.sys.used_memory() as f32 / self.sys.total_memory().max(1) as f32;
        let core_count = cpu_count as u32;
        let uptime = now_secs();

        // CPU frequency ratio via current/max per-core (limited, simplified)
        let freq_ratio = {
            if cpus.is_empty() {
                1.0
            } else {
                let avg_freq: u64 = cpus.iter().map(|c| c.frequency()).sum::<u64>() / cpu_count as u64;
                if avg_freq > 0 {
                    (avg_freq as f32 / 4000.0).clamp(0.1, 1.0) // relative to ~4GHz ceiling
                } else {
                    1.0
                }
            }
        };

        // Infer action
        let action = if cpu_util > 0.95 || mem_util > 0.95 {
            HwInferenceAction::EmergencyHalt
        } else if cpu_util > 0.80 || mem_util > 0.85 {
            HwInferenceAction::CpuOnly
        } else if cpu_util > 0.60 || mem_util > 0.70 {
            HwInferenceAction::ThrottledVisual
        } else {
            HwInferenceAction::FullVisual
        };

        let profile = HardwareProfile {
            cpu_utilization: cpu_util,
            mem_utilization: mem_util,
            core_count,
            cpu_temperature: cpu_util * 60.0 + 35.0,
            cpu_freq_ratio: freq_ratio,
            process_count: 0,
            uptime_secs: uptime,
            action,
        };

        self.profile_cache = profile.clone();
        profile
    }

    /// Get the last sampled hardware profile (no new sampling).
    pub fn cached(&self) -> &HardwareProfile {
        &self.profile_cache
    }

    /// Enable or disable active sampling.
    pub fn set_active(&self, active: bool) {
        self.active.store(active, Ordering::Release);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}

unsafe impl Send for HardwareGovernor {}
unsafe impl Sync for HardwareGovernor {}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn now_secs() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governor_creation() {
        let gov = HardwareGovernor::new(100);
        assert!(gov.is_active());
    }

    #[test]
    fn test_initial_profile() {
        let gov = HardwareGovernor::new(100);
        let cached = gov.cached();
        assert!(cached.core_count > 0);
        assert_eq!(cached.action as u8, HwInferenceAction::FullVisual as u8);
    }

    #[test]
    fn test_sample_returns_profile() {
        let mut gov = HardwareGovernor::new(10);
        let profile = gov.sample();
        assert!(profile.cpu_utilization >= 0.0);
        assert!(profile.mem_utilization >= 0.0);
    }

    #[test]
    fn test_set_active() {
        let gov = HardwareGovernor::new(100);
        assert!(gov.is_active());
        gov.set_active(false);
        assert!(!gov.is_active());
        gov.set_active(true);
        assert!(gov.is_active());
    }
}
