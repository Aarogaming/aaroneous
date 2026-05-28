/// Configuration for `SpecialistHost`

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How aggressively the host checkpoints learning state to persistence.
///
/// Choosing a value is a trade-off:
/// - **Short interval (e.g., 1s)**: Minimal data loss on crash, more SQLite I/O
/// - **Long interval (e.g., 5min)**: Less I/O, more learning lost on crash
/// - **Zero**: No background task; caller drives checkpointing manually
///
/// For most deployments, 30-60 seconds is a reasonable default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostConfig {
    /// Time between automatic checkpoints. `Duration::ZERO` disables the
    /// background loop entirely, in which case callers must call
    /// `host.checkpoint_now()` themselves.
    #[serde(with = "humantime_serde")]
    pub checkpoint_interval: Duration,
    /// If true, log every checkpoint at INFO level (otherwise DEBUG).
    /// Useful for diagnosing learning persistence issues.
    pub verbose_checkpoints: bool,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval: Duration::from_secs(30),
            verbose_checkpoints: false,
        }
    }
}

impl HostConfig {
    /// Create a config that does no automatic checkpointing.
    /// Use when callers prefer to drive checkpointing manually based on
    /// application events (e.g., "checkpoint after every N executions").
    pub fn manual_only() -> Self {
        Self {
            checkpoint_interval: Duration::ZERO,
            verbose_checkpoints: false,
        }
    }

    /// Create a config with a custom checkpoint interval.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            checkpoint_interval: interval,
            verbose_checkpoints: false,
        }
    }
}

// We use humantime_serde so configs can use strings like "30s" or "5m"
// in TOML/JSON. Fall back to a custom impl since humantime_serde may not be
// available in this workspace.
mod humantime_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(d: &Duration, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as total milliseconds for portability
        s.serialize_u64(d.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let ms = u64::deserialize(d)?;
        Ok(Duration::from_millis(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_30s() {
        let c = HostConfig::default();
        assert_eq!(c.checkpoint_interval, Duration::from_secs(30));
        assert!(!c.verbose_checkpoints);
    }

    #[test]
    fn test_manual_only_is_zero() {
        let c = HostConfig::manual_only();
        assert_eq!(c.checkpoint_interval, Duration::ZERO);
    }

    #[test]
    fn test_with_interval() {
        let c = HostConfig::with_interval(Duration::from_secs(60));
        assert_eq!(c.checkpoint_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_serde_round_trip() {
        let c = HostConfig::with_interval(Duration::from_secs(30));
        let json = serde_json::to_string(&c).unwrap();
        let recovered: HostConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.checkpoint_interval, c.checkpoint_interval);
        assert_eq!(recovered.verbose_checkpoints, c.verbose_checkpoints);
    }
}
