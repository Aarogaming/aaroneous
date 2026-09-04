/// Relativistic computation primitives.
///
/// Lorentz clock synchronization, Minkowski spacetime metrics,
/// geodesic curvature for hotspot navigation, and light-cone
/// causality enforcement.
// ── 1. Lorentz Clock Transformation ──────────────────────────────────
// Async clock counters for multi-speed processing loops.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct LorentzClock {
    pub local_rate: f64,
    pub ref_rate: f64,
    pub local_ticks: u64,
    pub ref_ticks: u64,
    pub gamma: f64,
}

impl LorentzClock {
    pub fn new(local_rate: f64, ref_rate: f64) -> Self {
        let gamma = if ref_rate > 0.0 {
            local_rate / ref_rate
        } else {
            1.0
        };
        LorentzClock {
            local_rate,
            ref_rate,
            local_ticks: 0,
            ref_ticks: 0,
            gamma,
        }
    }

    /// Advance local tick; returns dilated reference tick equivalent.
    pub fn tick(&mut self) -> u64 {
        self.local_ticks += 1;
        self.ref_ticks = (self.local_ticks as f64 * self.gamma) as u64;
        self.ref_ticks
    }

    /// Sync this clock to a reference clock via Lorentz-like transform.
    /// dt_local = gamma * dt_ref  (time dilation)
    pub fn sync(&mut self, ref_dt: f64) -> f64 {
        let local_dt = self.gamma * ref_dt;
        self.local_ticks += local_dt as u64;
        self.ref_ticks += ref_dt as u64;
        local_dt
    }

    pub fn local_elapsed(&self) -> f64 {
        self.local_ticks as f64 / self.local_rate
    }
    pub fn ref_elapsed(&self) -> f64 {
        self.ref_ticks as f64 / self.ref_rate
    }
}

// ── 2. Minkowski Space Vector Metrics ────────────────────────────────
// Spacetime similarity: s² = Δx² + Δy² + Δz² - c²Δt².

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpacetimeEvent {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub t: f64,
}

#[derive(Debug, Clone)]
pub struct MinkowskiMetric {
    pub c: f64,
}

impl MinkowskiMetric {
    pub fn new(c: f64) -> Self {
        MinkowskiMetric { c }
    }

    /// Squared spacetime interval between two events.
    /// s² > 0  → space-like (disconnected)
    /// s² = 0  → light-like (causally connected at c)
    /// s² < 0  → time-like (causally connected)
    pub fn interval_sq(&self, a: &SpacetimeEvent, b: &SpacetimeEvent) -> f64 {
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let dz = a.z - b.z;
        let dt = a.t - b.t;
        dx * dx + dy * dy + dz * dz - self.c * self.c * dt * dt
    }

    pub fn is_timelike(&self, a: &SpacetimeEvent, b: &SpacetimeEvent) -> bool {
        self.interval_sq(a, b) < 0.0
    }

    pub fn is_spacelike(&self, a: &SpacetimeEvent, b: &SpacetimeEvent) -> bool {
        self.interval_sq(a, b) > 0.0
    }

    pub fn is_lightlike(&self, a: &SpacetimeEvent, b: &SpacetimeEvent) -> bool {
        self.interval_sq(a, b).abs() < 1e-9
    }
}

// ── 3. Geodesic Tensor Curvature ─────────────────────────────────────
// Warps a flat vector field so searches slide toward density hot-spots.

#[derive(Debug, Clone)]
pub struct GeodesicField {
    pub curvature: Vec<f64>,
    pub dimensions: usize,
}

impl GeodesicField {
    pub fn new(dimensions: usize) -> Self {
        GeodesicField {
            curvature: vec![0.0; dimensions * dimensions],
            dimensions,
        }
    }

    /// Set curvature component at (i, j).
    pub fn set_curvature(&mut self, i: usize, j: usize, val: f64) {
        if i < self.dimensions && j < self.dimensions {
            self.curvature[i * self.dimensions + j] = val;
        }
    }

    /// Apply curvature: deflect a position vector toward density hot-spots.
    /// result[i] = sum_j(curvature[i][j] * position[j])
    pub fn slide(&self, position: &[f64]) -> Vec<f64> {
        let n = self.dimensions.min(position.len());
        let mut result = vec![0.0; n];
        for (i, res) in result.iter_mut().enumerate().take(n) {
            let row = &self.curvature[i * self.dimensions..i * self.dimensions + n];
            *res = row.iter().zip(position.iter().take(n)).map(|(c, p)| c * p).sum();
        }
        result
    }

    /// Train curvature from a set of points (simple Hebbian-like update).
    /// Moves curvature to align with the covariance of observed points.
    pub fn train(&mut self, points: &[Vec<f64>], lr: f64) {
        let n = self.dimensions;
        let mut cov = vec![0.0; n * n];
        let count = points.len() as f64;
        for point in points {
            for i in 0..n.min(point.len()) {
                for j in 0..n.min(point.len()) {
                    cov[i * n + j] += point[i] * point[j];
                }
            }
        }
        let count_safe = count.max(1.0);
        for (c, curv) in cov.iter_mut().zip(&mut self.curvature).take(n * n) {
            *c /= count_safe;
            *curv += lr * (*c - *curv);
        }
    }
}

// ── 4. Light-Cone Causality ─────────────────────────────────────────
// Bitmask dependency: actions only fire if prerequisites sit in the
// forward (future) light cone.

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CausalMask {
    pub bits: u64,
    pub spacetime: SpacetimeEvent,
}

#[derive(Debug, Clone)]
pub struct LightConeCausality {
    pub metric: MinkowskiMetric,
    pub events: Vec<CausalMask>,
}

impl LightConeCausality {
    pub fn new(c: f64) -> Self {
        LightConeCausality {
            metric: MinkowskiMetric::new(c),
            events: Vec::new(),
        }
    }

    /// Register a dependency: action `dependent` requires `prerequisite`
    /// to lie in its past light cone.
    pub fn add_dependency(&mut self, bits: u64, event: SpacetimeEvent) {
        self.events.push(CausalMask {
            bits,
            spacetime: event,
        });
    }

    /// Check if all prerequisites for `bits` are satisfied given a trigger event.
    /// Returns true only if every required dependency sits at an earlier time
    /// (time-like or light-like separated) and shares masked bits.
    pub fn can_fire(&self, action_bits: u64, trigger: &SpacetimeEvent) -> bool {
        for dep in &self.events {
            if dep.bits & action_bits != 0 {
                // This dependency is required: must be time-like or light-like
                // AND in the past (dep.t < trigger.t)
                if dep.spacetime.t >= trigger.t {
                    return false; // in the future — cannot cause
                }
                if self.metric.is_spacelike(&dep.spacetime, trigger) {
                    return false; // space-like separated — no causal link
                }
            }
        }
        true
    }

    /// Register a CausalMask for future checks.
    pub fn register(&mut self, mask: CausalMask) {
        self.events.push(mask);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lorentz_clock_basic() {
        let mut clock = LorentzClock::new(100.0, 50.0); // local twice as fast
        assert!((clock.gamma - 2.0).abs() < 1e-9);
        let ref_tick = clock.tick();
        assert_eq!(ref_tick, 2); // gamma=2, local=1 → ref=2
    }

    #[test]
    fn test_lorentz_clock_sync() {
        let mut clock = LorentzClock::new(100.0, 100.0);
        let _local = clock.sync(5.0);
        assert_eq!(clock.local_ticks, 5);
        assert_eq!(clock.ref_ticks, 5);
    }

    #[test]
    fn test_lorentz_clock_dilation() {
        let mut clock = LorentzClock::new(200.0, 100.0); // gamma=2
        let _ = clock.tick();
        assert_eq!(clock.local_ticks, 1);
        assert_eq!(clock.ref_ticks, 2);
    }

    #[test]
    fn test_minkowski_timelike() {
        let metric = MinkowskiMetric::new(1.0);
        let a = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
        };
        let b = SpacetimeEvent {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            t: 2.0,
        }; // Δt=2, Δx=1 → 1-4=-3
        assert!(metric.is_timelike(&a, &b));
        assert!(!metric.is_spacelike(&a, &b));
    }

    #[test]
    fn test_minkowski_spacelike() {
        let metric = MinkowskiMetric::new(1.0);
        let a = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
        };
        let b = SpacetimeEvent {
            x: 10.0,
            y: 0.0,
            z: 0.0,
            t: 1.0,
        }; // Δt=1, Δx=10 → 100-1=99
        assert!(metric.is_spacelike(&a, &b));
    }

    #[test]
    fn test_minkowski_lightlike() {
        let metric = MinkowskiMetric::new(1.0);
        let a = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
        };
        let b = SpacetimeEvent {
            x: 3.0,
            y: 0.0,
            z: 0.0,
            t: 3.0,
        }; // Δx=3, Δt=3 → 9-9=0
        assert!(metric.is_lightlike(&a, &b));
    }

    #[test]
    fn test_geodesic_slide() {
        let mut field = GeodesicField::new(2);
        field.set_curvature(0, 0, 1.0);
        field.set_curvature(0, 1, 0.5);
        field.set_curvature(1, 0, 0.0);
        field.set_curvature(1, 1, 1.0);
        let result = field.slide(&[2.0, 3.0]);
        assert!((result[0] - 3.5).abs() < 1e-9);
        assert!((result[1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_geodesic_train() {
        let mut field = GeodesicField::new(2);
        let points = vec![vec![1.0, 2.0], vec![2.0, 4.0], vec![3.0, 6.0]];
        field.train(&points, 0.1);
        // After training, curvature should reflect covariance
        assert!(field.curvature[0] > 0.0); // positive correlation
    }

    #[test]
    fn test_lightcone_can_fire_past() {
        let mut lc = LightConeCausality::new(1.0);
        let dep = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 1.0,
        };
        lc.add_dependency(0b0001, dep);
        let trigger = SpacetimeEvent {
            x: 0.5,
            y: 0.0,
            z: 0.0,
            t: 3.0,
        };
        assert!(lc.can_fire(0b0001, &trigger));
    }

    #[test]
    fn test_lightcone_blocked_future() {
        let mut lc = LightConeCausality::new(1.0);
        let dep = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 10.0,
        };
        lc.add_dependency(0b0001, dep);
        let trigger = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 3.0,
        };
        assert!(!lc.can_fire(0b0001, &trigger));
    }

    #[test]
    fn test_lightcone_blocked_spacelike() {
        let mut lc = LightConeCausality::new(1.0);
        let dep = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 1.0,
        };
        lc.add_dependency(0b0001, dep);
        let trigger = SpacetimeEvent {
            x: 100.0,
            y: 0.0,
            z: 0.0,
            t: 2.0,
        };
        assert!(!lc.can_fire(0b0001, &trigger));
    }

    #[test]
    fn test_lightcone_unrelated_bits() {
        let mut lc = LightConeCausality::new(1.0);
        let dep = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 1.0,
        };
        lc.add_dependency(0b0010, dep);
        let trigger = SpacetimeEvent {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 2.0,
        };
        // Action bits 0b0001 don't require dependency 0b0010
        assert!(lc.can_fire(0b0001, &trigger));
    }
}
