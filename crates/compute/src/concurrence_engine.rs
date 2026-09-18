//! crates/compute/src/concurrence_engine.rs
//! Real-time Dual-Rail Concurrence & Divergence Metrics Engine.
//!
//! Tracks rolling agreement rate between shadow `.si` model predictions and
//! the deterministic rule-based orchestrator over a configurable window of N
//! ticks.  All state lives in fixed-size arrays — zero heap allocation on the
//! hot update path.

/// Matches `core_contracts::FlightEventKind::Checkpoint = 9`.
/// Defined here so `concurrence_engine` does not depend on `core_contracts`.
pub const FLIGHT_EVENT_CHECKPOINT: u16 = 9;

// ── Rolling window constants ─────────────────────────────────────────────────

/// Default rolling window size (1 000 ticks ≈ 8.3 s at 120 Hz).
pub const DEFAULT_CONCURRENCE_WINDOW: usize = 1_000;

/// Graduation threshold: model must agree on this fraction of the window.
pub const GRADUATION_THRESHOLD: f32 = 0.95;

// ── Public types ─────────────────────────────────────────────────────────────

/// A single shadow-inference tick result fed into the engine.
#[derive(Debug, Clone, Copy)]
pub struct ShadowTickResult {
    /// Ground-truth opcode dispatched by the deterministic rule engine.
    pub actual_opcode: u16,
    /// Opcode predicted by the mounted `.si` shadow model.
    pub predicted_opcode: u16,
    /// Model confidence score in [0.0, 1.0].
    pub confidence: f32,
    /// Reward signal for this tick (positive = beneficial outcome).
    pub reward: f32,
}

/// Snapshot of the current concurrence metrics, safe to clone to the HUD thread.
#[derive(Debug, Clone, Copy)]
pub struct ConcurrenceSnapshot {
    /// Fraction of the rolling window where actual == predicted (0.0 – 1.0).
    pub rolling_concurrence: f32,
    /// Total ticks processed lifetime.
    pub total_ticks: u64,
    /// Total agreement events lifetime.
    pub total_agreements: u64,
    /// Exponential moving average of prediction confidence.
    pub avg_confidence: f32,
    /// Exponential moving average of the reward signal.
    pub avg_reward: f32,
    /// Mean-squared error between actual and predicted opcode IDs over the window.
    pub divergence_mse: f32,
    /// True once `rolling_concurrence >= GRADUATION_THRESHOLD` for the first time.
    pub graduated: bool,
    /// Ticks elapsed since the graduation threshold was first crossed.
    pub ticks_since_graduation: u64,
}

impl Default for ConcurrenceSnapshot {
    fn default() -> Self {
        Self {
            rolling_concurrence: 0.0,
            total_ticks: 0,
            total_agreements: 0,
            avg_confidence: 0.0,
            avg_reward: 0.0,
            divergence_mse: 0.0,
            graduated: false,
            ticks_since_graduation: 0,
        }
    }
}

/// Event emitted exactly once when the graduation threshold is first crossed.
#[derive(Debug, Clone, Copy)]
pub struct GraduationReadyEvent {
    /// The tick index at which graduation was triggered.
    pub tick_index: u64,
    /// The measured concurrence at the moment of graduation.
    pub concurrence_at_graduation: f32,
    /// Flight-recorder event discriminant (`FLIGHT_EVENT_CHECKPOINT = 9`).
    pub event_kind: u16,
}

// ── ConcurrenceEngine ────────────────────────────────────────────────────────

/// Fixed-capacity rolling-window concurrence tracker.  Zero heap allocation.
///
/// The const generic `W` sets the window size.  The default alias
/// `DefaultConcurrenceEngine` uses `DEFAULT_CONCURRENCE_WINDOW = 1_000`.
pub struct ConcurrenceEngine<const W: usize = DEFAULT_CONCURRENCE_WINDOW> {
    /// Per-slot agreement flag (1 = agree, 0 = disagree).
    ring_agree: [u8; W],
    /// Per-slot squared divergence `(actual − predicted)²`.
    ring_diverge_sq: [f32; W],
    /// Write head into the ring buffer.
    head: usize,
    /// How many slots are filled (saturates at W).
    filled: usize,
    /// Running sum of agreement flags in the current window.
    window_agree_sum: u32,
    /// Running sum of squared divergences in the current window.
    window_diverge_sq_sum: f32,
    /// Lifetime tick counter.
    total_ticks: u64,
    /// Lifetime agreement counter.
    total_agreements: u64,
    /// EMA of confidence (alpha = 0.05 ≈ 20-tick half-life).
    avg_confidence: f32,
    /// EMA of reward signal.
    avg_reward: f32,
    /// Set true once graduation threshold is first crossed.
    graduated: bool,
    /// Tick index at which graduation occurred.
    grad_tick: u64,
    /// Ticks elapsed since graduation (0 before graduation).
    ticks_since_graduation: u64,
}

/// Concrete alias for the standard 1 000-tick engine.
pub type DefaultConcurrenceEngine = ConcurrenceEngine<DEFAULT_CONCURRENCE_WINDOW>;

impl<const W: usize> ConcurrenceEngine<W> {
    /// Construct a zero-initialised engine.  Fully `const`, zero heap.
    pub const fn new() -> Self {
        Self {
            ring_agree: [0u8; W],
            ring_diverge_sq: [0.0f32; W],
            head: 0,
            filled: 0,
            window_agree_sum: 0,
            window_diverge_sq_sum: 0.0,
            total_ticks: 0,
            total_agreements: 0,
            avg_confidence: 0.0,
            avg_reward: 0.0,
            graduated: false,
            grad_tick: 0,
            ticks_since_graduation: 0,
        }
    }

    /// Record one shadow tick.  Returns `Some(GraduationReadyEvent)` the first
    /// time the rolling concurrence crosses `GRADUATION_THRESHOLD`.
    /// **Zero heap allocation** on the hot path.
    #[inline]
    pub fn update(&mut self, result: ShadowTickResult) -> Option<GraduationReadyEvent> {
        const EMA_ALPHA: f32 = 0.05;

        let agreed = u8::from(result.actual_opcode == result.predicted_opcode);
        let d = result.actual_opcode as f32 - result.predicted_opcode as f32;
        let diverge_sq = d * d;

        // Evict the oldest slot when the window is full.
        let old_agree = self.ring_agree[self.head];
        let old_div_sq = self.ring_diverge_sq[self.head];

        if self.filled == W {
            self.window_agree_sum = self.window_agree_sum.saturating_sub(old_agree as u32);
            self.window_diverge_sq_sum -= old_div_sq;
        } else {
            self.filled += 1;
        }

        // Write new slot.
        self.ring_agree[self.head] = agreed;
        self.ring_diverge_sq[self.head] = diverge_sq;
        self.head = (self.head + 1) % W;

        self.window_agree_sum += agreed as u32;
        self.window_diverge_sq_sum += diverge_sq;

        // Lifetime counters.
        self.total_ticks += 1;
        self.total_agreements += agreed as u64;

        // EMA updates.
        self.avg_confidence =
            self.avg_confidence.mul_add(1.0 - EMA_ALPHA, result.confidence * EMA_ALPHA);
        self.avg_reward =
            self.avg_reward.mul_add(1.0 - EMA_ALPHA, result.reward * EMA_ALPHA);

        // Graduation check — only when window is fully primed.
        if !self.graduated && self.filled == W {
            let rate = self.window_agree_sum as f32 / W as f32;
            if rate >= GRADUATION_THRESHOLD {
                self.graduated = true;
                self.grad_tick = self.total_ticks;
                return Some(GraduationReadyEvent {
                    tick_index: self.total_ticks,
                    concurrence_at_graduation: rate,
                    event_kind: FLIGHT_EVENT_CHECKPOINT,
                });
            }
        }

        if self.graduated {
            self.ticks_since_graduation = self.total_ticks - self.grad_tick;
        }

        None
    }

    /// Returns the current metrics snapshot.  Zero allocation (copies scalars).
    #[inline]
    pub fn snapshot(&self) -> ConcurrenceSnapshot {
        let rolling_concurrence = if self.filled == 0 {
            0.0
        } else {
            self.window_agree_sum as f32 / self.filled as f32
        };
        let divergence_mse = if self.filled == 0 {
            0.0
        } else {
            self.window_diverge_sq_sum / self.filled as f32
        };
        ConcurrenceSnapshot {
            rolling_concurrence,
            total_ticks: self.total_ticks,
            total_agreements: self.total_agreements,
            avg_confidence: self.avg_confidence,
            avg_reward: self.avg_reward,
            divergence_mse,
            graduated: self.graduated,
            ticks_since_graduation: self.ticks_since_graduation,
        }
    }

    /// Window fill fraction in [0.0, 1.0].  Ramps up during the first W ticks.
    #[inline]
    pub fn window_fill_fraction(&self) -> f32 {
        self.filled as f32 / W as f32
    }
}

impl<const W: usize> Default for ConcurrenceEngine<W> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(actual: u16, predicted: u16) -> ShadowTickResult {
        ShadowTickResult {
            actual_opcode: actual,
            predicted_opcode: predicted,
            confidence: 0.9,
            reward: 1.0,
        }
    }

    #[test]
    fn test_empty_engine_snapshot() {
        let eng: ConcurrenceEngine<10> = ConcurrenceEngine::new();
        let snap = eng.snapshot();
        assert_eq!(snap.rolling_concurrence, 0.0);
        assert_eq!(snap.total_ticks, 0);
        assert!(!snap.graduated);
    }

    #[test]
    fn test_agreement_tracking() {
        let mut eng: ConcurrenceEngine<10> = ConcurrenceEngine::new();
        for _ in 0..8 {
            eng.update(tick(1, 1));
        }
        for _ in 0..2 {
            eng.update(tick(1, 2));
        }
        let snap = eng.snapshot();
        // 8/10 = 0.80
        assert!((snap.rolling_concurrence - 0.80).abs() < 0.01);
        assert_eq!(snap.total_ticks, 10);
        assert_eq!(snap.total_agreements, 8);
    }

    #[test]
    fn test_ring_eviction() {
        // Window of 5.  Fill 3 disagrees then 5 agrees → window should be 100%.
        let mut eng: ConcurrenceEngine<5> = ConcurrenceEngine::new();
        for _ in 0..3 {
            eng.update(tick(1, 2));
        }
        for _ in 0..5 {
            eng.update(tick(1, 1));
        }
        let snap = eng.snapshot();
        assert!((snap.rolling_concurrence - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_graduation_event_fires_once() {
        let mut eng: ConcurrenceEngine<10> = ConcurrenceEngine::new();
        for _ in 0..9 {
            eng.update(tick(1, 1));
        }
        // 10th tick → window full → 100% → graduation.
        let event = eng.update(tick(1, 1));
        assert!(event.is_some());
        let ev = event.unwrap();
        assert!((ev.concurrence_at_graduation - 1.0).abs() < 0.001);
        // Further updates must NOT emit another event.
        let event2 = eng.update(tick(1, 1));
        assert!(event2.is_none());
        let snap = eng.snapshot();
        assert!(snap.graduated);
    }

    #[test]
    fn test_divergence_mse() {
        let mut eng: ConcurrenceEngine<4> = ConcurrenceEngine::new();
        for _ in 0..4 {
            eng.update(ShadowTickResult {
                actual_opcode: 1,
                predicted_opcode: 3,
                confidence: 0.5,
                reward: 0.0,
            });
        }
        let snap = eng.snapshot();
        // (1-3)^2 = 4.0
        assert!((snap.divergence_mse - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_graduation_not_before_full_window() {
        let mut eng: ConcurrenceEngine<10> = ConcurrenceEngine::new();
        for i in 0..9u64 {
            let ev = eng.update(tick(1, 1));
            assert!(ev.is_none(), "tick {i} should not graduate before window is full");
        }
    }
}