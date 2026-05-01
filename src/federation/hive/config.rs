/// Federation-level configuration.
///
/// Currently this is a thin wrapper around per-host settings, but it's the
/// natural place for future cross-cutting policy (resource caps, health
/// thresholds, telemetry endpoints).

use crate::federation::host::HostConfig;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cross-cutting federation configuration.
///
/// The defaults are reasonable for a development environment. Production
/// deployments will typically tune `default_checkpoint_interval` based on
/// expected execution rate and acceptable data loss on crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Default checkpoint cadence for hosts that don't override it.
    /// `Duration::ZERO` disables automatic checkpointing - callers must
    /// drive `checkpoint_all()` themselves.
    #[serde(with = "duration_ms")]
    pub default_checkpoint_interval: Duration,

    /// Whether to log every checkpoint at INFO (otherwise DEBUG).
    pub verbose_checkpoints: bool,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            default_checkpoint_interval: Duration::from_secs(30),
            verbose_checkpoints: false,
        }
    }
}

impl FederationConfig {
    /// Manual-checkpoint-only config (no auto-save loops)
    pub fn manual_only() -> Self {
        Self {
            default_checkpoint_interval: Duration::ZERO,
            verbose_checkpoints: false,
        }
    }

    /// Convert to a per-host `HostConfig` for hosts that adopt the federation default
    pub fn to_host_config(&self) -> HostConfig {
        HostConfig {
            checkpoint_interval: self.default_checkpoint_interval,
            verbose_checkpoints: self.verbose_checkpoints,
        }
    }
}

mod duration_ms {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(d.as_millis() as u64)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_30s() {
        let c = FederationConfig::default();
        assert_eq!(c.default_checkpoint_interval, Duration::from_secs(30));
        assert!(!c.verbose_checkpoints);
    }

    #[test]
    fn test_manual_only_zero_interval() {
        let c = FederationConfig::manual_only();
        assert_eq!(c.default_checkpoint_interval, Duration::ZERO);
    }

    #[test]
    fn test_to_host_config() {
        let c = FederationConfig {
            default_checkpoint_interval: Duration::from_secs(60),
            verbose_checkpoints: true,
        };
        let h = c.to_host_config();
        assert_eq!(h.checkpoint_interval, Duration::from_secs(60));
        assert!(h.verbose_checkpoints);
    }

    #[test]
    fn test_serde_round_trip() {
        let c = FederationConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: FederationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.default_checkpoint_interval, c.default_checkpoint_interval);
    }
}
