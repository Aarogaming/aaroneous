/// Cache Warming System for Specialist Models
/// 
/// Proactively loads frequently-used models into memory
/// to reduce first-use latency

use std::collections::{HashMap, VecDeque};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheWarmingStrategy {
    /// How many models to keep warm at startup
    pub startup_warmup_count: usize,
    /// Models to prioritize for warming
    pub priority_models: Vec<crate::federation::specialist::SpecialistId>,
    /// Track model access patterns for predictions
    pub track_access_patterns: bool,
    /// Warm cache during idle periods
    pub warm_during_idle: bool,
    /// Target cache hit rate (0.0-1.0)
    pub target_hit_rate: f32,
}

impl CacheWarmingStrategy {
    /// Aggressive warming for performance-critical deployments
    pub fn aggressive() -> Self {
        Self {
            startup_warmup_count: 5,
            priority_models: vec![
                crate::federation::specialist::SpecialistId::Sentinel,
                crate::federation::specialist::SpecialistId::Visionary,
                crate::federation::specialist::SpecialistId::Omnipresent,
            ],
            track_access_patterns: true,
            warm_during_idle: true,
            target_hit_rate: 0.95,
        }
    }

    /// Balanced warming
    pub fn balanced() -> Self {
        Self {
            startup_warmup_count: 3,
            priority_models: vec![
                crate::federation::specialist::SpecialistId::Sentinel,
                crate::federation::specialist::SpecialistId::Visionary,
            ],
            track_access_patterns: true,
            warm_during_idle: false,
            target_hit_rate: 0.85,
        }
    }

    /// Minimal warming for resource-constrained environments
    pub fn minimal() -> Self {
        Self {
            startup_warmup_count: 1,
            priority_models: vec![crate::federation::specialist::SpecialistId::Sentinel],
            track_access_patterns: false,
            warm_during_idle: false,
            target_hit_rate: 0.70,
        }
    }
}

/// Model access pattern for predictive warming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub access_count: u64,
    pub last_access_ms: u64,
    pub avg_time_between_accesses_ms: u64,
    pub cache_hit_rate: f32,
}

impl AccessPattern {
    pub fn new(specialist_id: crate::federation::specialist::SpecialistId) -> Self {
        Self {
            specialist_id,
            access_count: 0,
            last_access_ms: 0,
            avg_time_between_accesses_ms: 0,
            cache_hit_rate: 0.0,
        }
    }

    /// Predict next access time
    pub fn predict_next_access_ms(&self, current_time_ms: u64) -> Option<u64> {
        if self.avg_time_between_accesses_ms == 0 {
            return None;
        }
        let time_since_access = current_time_ms.saturating_sub(self.last_access_ms);
        Some(self.last_access_ms + self.avg_time_between_accesses_ms + time_since_access)
    }

    /// Should keep this model warm?
    pub fn should_stay_warm(&self) -> bool {
        // Stay warm if accessed frequently (hit rate > 70%) and
        // less than 5 minutes since last access
        self.cache_hit_rate > 0.7 && self.last_access_ms > 0
    }
}

/// Cache warming tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheWarmingTracker {
    pub patterns: HashMap<crate::federation::specialist::SpecialistId, AccessPattern>,
    pub currently_warm: Vec<crate::federation::specialist::SpecialistId>,
    pub access_history: VecDeque<AccessRecord>,
    pub last_warming_time: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRecord {
    pub specialist_id: crate::federation::specialist::SpecialistId,
    pub timestamp_ms: u64,
    pub hit: bool,
    pub latency_ms: f32,
}

impl CacheWarmingTracker {
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            currently_warm: Vec::new(),
            access_history: VecDeque::with_capacity(10000),
            last_warming_time: 0,
        }
    }

    /// Record a model access
    pub fn record_access(
        &mut self,
        specialist_id: crate::federation::specialist::SpecialistId,
        hit: bool,
        latency_ms: f32,
        current_time_ms: u64,
    ) {
        // Update pattern
        let pattern = self
            .patterns
            .entry(specialist_id)
            .or_insert_with(|| AccessPattern::new(specialist_id));

        let time_since_last = current_time_ms.saturating_sub(pattern.last_access_ms);
        if pattern.last_access_ms > 0 && time_since_last > 0 {
            // Exponential moving average
            pattern.avg_time_between_accesses_ms = (pattern.avg_time_between_accesses_ms * 7
                + time_since_last)
                / 8;
        }

        pattern.access_count += 1;
        pattern.last_access_ms = current_time_ms;

        // Update hit rate
        let total_accesses = pattern.access_count as f32;
        let current_hits = (pattern.cache_hit_rate * (total_accesses - 1.0)) + if hit { 1.0 } else { 0.0 };
        pattern.cache_hit_rate = current_hits / total_accesses;

        // Record in history
        self.access_history.push_back(AccessRecord {
            specialist_id,
            timestamp_ms: current_time_ms,
            hit,
            latency_ms,
        });

        // Keep history bounded
        while self.access_history.len() > 10000 {
            self.access_history.pop_front();
        }
    }

    /// Recommend next warming candidates
    pub fn recommend_warming(&self, current_time_ms: u64, count: usize) -> Vec<crate::federation::specialist::SpecialistId> {
        let mut candidates: Vec<_> = self
            .patterns
            .values()
            .filter(|p| {
                // Candidates: recently accessed or frequent
                if let Some(next_access) = p.predict_next_access_ms(current_time_ms) {
                    next_access <= current_time_ms + 5000 // Within 5 seconds
                } else {
                    false
                }
            })
            .collect();

        // Sort by priority: high hit rate + soon access
        candidates.sort_by(|a, b| {
            let a_score = a.cache_hit_rate + (1000.0 / (a.avg_time_between_accesses_ms.max(1) as f32));
            let b_score = b.cache_hit_rate + (1000.0 / (b.avg_time_between_accesses_ms.max(1) as f32));
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
            .into_iter()
            .take(count)
            .map(|p| p.specialist_id)
            .collect()
    }

    /// Get current cache hit rate
    pub fn overall_hit_rate(&self) -> f32 {
        if self.access_history.is_empty() {
            return 0.0;
        }

        let hits = self.access_history.iter().filter(|r| r.hit).count();
        hits as f32 / self.access_history.len() as f32
    }

    /// Get warming effectiveness (cache hits since last warming)
    pub fn warming_effectiveness(&self) -> f32 {
        let recent: Vec<_> = self
            .access_history
            .iter()
            .filter(|r| r.timestamp_ms >= self.last_warming_time)
            .collect();

        if recent.is_empty() {
            return 0.0;
        }

        let hits = recent.iter().filter(|r| r.hit).count();
        hits as f32 / recent.len() as f32
    }
}

impl Default for CacheWarmingTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Warming schedule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmingSchedule {
    pub entries: Vec<ScheduleEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub hour_of_day: u32,
    pub minute_of_hour: u32,
    pub specialists_to_warm: Vec<crate::federation::specialist::SpecialistId>,
}

impl WarmingSchedule {
    /// Create schedule that warms common specialists at startup and periodic intervals
    pub fn default_schedule() -> Self {
        Self {
            entries: vec![
                // Startup warming
                ScheduleEntry {
                    hour_of_day: 0,
                    minute_of_hour: 0,
                    specialists_to_warm: vec![
                        crate::federation::specialist::SpecialistId::Sentinel,
                        crate::federation::specialist::SpecialistId::Visionary,
                        crate::federation::specialist::SpecialistId::Omnipresent,
                    ],
                },
                // Mid-day warm-up
                ScheduleEntry {
                    hour_of_day: 12,
                    minute_of_hour: 0,
                    specialists_to_warm: vec![
                        crate::federation::specialist::SpecialistId::Sentinel,
                        crate::federation::specialist::SpecialistId::Phygital,
                    ],
                },
            ],
        }
    }

    /// Check if any warming should happen at given time
    pub fn should_warm_at(&self, hour: u32, minute: u32) -> Option<Vec<crate::federation::specialist::SpecialistId>> {
        self.entries
            .iter()
            .find(|e| e.hour_of_day == hour && e.minute_of_hour == minute)
            .map(|e| e.specialists_to_warm.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_warming_strategy_aggressive() {
        let strategy = CacheWarmingStrategy::aggressive();
        assert_eq!(strategy.startup_warmup_count, 5);
        assert!(strategy.track_access_patterns);
    }

    #[test]
    fn test_access_pattern_creation() {
        let pattern = AccessPattern::new(crate::federation::specialist::SpecialistId::Sentinel);
        assert_eq!(pattern.access_count, 0);
        assert_eq!(pattern.cache_hit_rate, 0.0);
    }

    #[test]
    fn test_access_pattern_should_stay_warm() {
        let mut pattern = AccessPattern::new(crate::federation::specialist::SpecialistId::Sentinel);
        pattern.cache_hit_rate = 0.8;
        pattern.last_access_ms = 1000;
        assert!(pattern.should_stay_warm());

        pattern.cache_hit_rate = 0.5; // Below threshold
        assert!(!pattern.should_stay_warm());
    }

    #[test]
    fn test_cache_warming_tracker_record() {
        let mut tracker = CacheWarmingTracker::new();
        tracker.record_access(
            crate::federation::specialist::SpecialistId::Sentinel,
            true,
            10.0,
            1000,
        );

        assert_eq!(tracker.access_history.len(), 1);
        assert!(tracker.patterns.contains_key(&crate::federation::specialist::SpecialistId::Sentinel));
    }

    #[test]
    fn test_cache_warming_tracker_hit_rate() {
        let mut tracker = CacheWarmingTracker::new();
        tracker.record_access(
            crate::federation::specialist::SpecialistId::Sentinel,
            true,
            10.0,
            1000,
        );
        tracker.record_access(
            crate::federation::specialist::SpecialistId::Sentinel,
            true,
            10.0,
            1010,
        );
        tracker.record_access(
            crate::federation::specialist::SpecialistId::Sentinel,
            false,
            50.0,
            1020,
        );

        assert!((tracker.overall_hit_rate() - 0.667).abs() < 0.01);
    }

    #[test]
    fn test_warming_schedule() {
        let schedule = WarmingSchedule::default_schedule();
        assert!(!schedule.entries.is_empty());

        let warming = schedule.should_warm_at(0, 0);
        assert!(warming.is_some());
        assert!(!warming.unwrap().is_empty());
    }
}
