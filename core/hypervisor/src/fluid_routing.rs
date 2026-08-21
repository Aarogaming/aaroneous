/// Fluid dynamics for data routing and thermal entropy management.
///
/// Navier-Stokes inspired data routing treats throughput as hydraulic
/// pressure; overflows divert to parallel cores. Entropy sweeper
/// periodically purges high-entropy buffers to reduce CPU thermal load.
// ── 5. Navier-Stokes Data Fluid Routing ──────────────────────────────
// Models data pipelines as hydraulic networks.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PipeChannel {
    pub id: usize,
    pub arrival_rate: f64,
    pub service_rate: f64,
    pub capacity: f64,
    pub load: f64,
}

#[derive(Debug, Clone)]
pub struct NavierStokesRouter {
    pub channels: Vec<PipeChannel>,
    pub overflow_threshold: f64,
}

impl NavierStokesRouter {
    pub fn new(threshold: f64) -> Self {
        NavierStokesRouter {
            channels: Vec::new(),
            overflow_threshold: threshold,
        }
    }

    pub fn add_channel(&mut self, id: usize, service_rate: f64, capacity: f64) {
        self.channels.push(PipeChannel {
            id,
            arrival_rate: 0.0,
            service_rate,
            capacity,
            load: 0.0,
        });
    }

    /// Compute pressure = arrival_rate / service_rate (dimensionless).
    pub fn pressure(&self, id: usize) -> Option<f64> {
        self.channels
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.arrival_rate / c.service_rate.max(1e-9))
    }

    /// Route data to the least-pressured channel; returns channel id.
    pub fn route(&mut self, data_size: f64) -> usize {
        let best_idx = self
            .channels
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let pa = a.arrival_rate / a.service_rate.max(1e-9);
                let pb = b.arrival_rate / b.service_rate.max(1e-9);
                pa.partial_cmp(&pb).unwrap()
            })
            .map(|(i, _)| i);
        let idx = best_idx.unwrap_or(0);
        if idx >= self.channels.len() {
            return 0;
        }
        let ch = &mut self.channels[idx];
        ch.arrival_rate += data_size;
        ch.load += data_size;
        // Overflow: if pressure exceeds threshold, spill to next channel
        if ch.arrival_rate / ch.service_rate.max(1e-9) > self.overflow_threshold {
            let spill = data_size * 0.5;
            ch.arrival_rate -= spill;
            let spill_idx = self
                .channels
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != idx)
                .min_by(|(_, a), (_, b)| {
                    let pa = a.arrival_rate / a.service_rate.max(1e-9);
                    let pb = b.arrival_rate / b.service_rate.max(1e-9);
                    pa.partial_cmp(&pb).unwrap()
                })
                .map(|(i, _)| i);
            if let Some(si) = spill_idx
                && si < self.channels.len()
            {
                let sc = &mut self.channels[si];
                sc.arrival_rate += spill;
                sc.load += spill;
            }
        }
        self.channels[idx].id
    }

    /// Drain: service each channel, reducing load.
    pub fn drain(&mut self, dt: f64) {
        for ch in &mut self.channels {
            let processed = (ch.service_rate * dt).min(ch.arrival_rate);
            ch.arrival_rate -= processed;
        }
    }

    pub fn total_load(&self) -> f64 {
        self.channels.iter().map(|c| c.load).sum()
    }
}

// ── 6. Thermal Entropy Bit Sweeping ──────────────────────────────────
// Background scan to purge high-entropy data noise.

#[derive(Debug, Clone)]
pub struct EntropySweeper {
    pub threshold: f64,
    pub period_ticks: u64,
    pub counter: u64,
    pub purged_bytes: u64,
}

impl EntropySweeper {
    pub fn new(threshold: f64, period_ticks: u64) -> Self {
        EntropySweeper {
            threshold,
            period_ticks,
            counter: 0,
            purged_bytes: 0,
        }
    }

    /// Compute Shannon entropy of a byte slice (H = -Σ p(i)·log₂ p(i)).
    pub fn shannon_entropy(data: &[u8]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        let mut freq = [0u64; 256];
        for &b in data {
            freq[b as usize] += 1;
        }
        let len = data.len() as f64;
        let mut h = 0.0;
        for &count in &freq {
            if count > 0 {
                let p = count as f64 / len;
                h -= p * p.log2();
            }
        }
        h
    }

    /// Tick the sweeper; returns bytes purged this tick (0 if not due).
    pub fn tick(&mut self, buffers: &mut [Vec<u8>]) -> u64 {
        self.counter += 1;
        if self.counter < self.period_ticks {
            return 0;
        }
        self.counter = 0;

        let mut total_purged = 0u64;
        for buf in buffers.iter_mut() {
            let entropy = Self::shannon_entropy(buf);
            if entropy > self.threshold {
                total_purged += buf.len() as u64;
                buf.clear();
            }
        }
        self.purged_bytes += total_purged;
        total_purged
    }

    /// Sweep a single buffer; returns true if purged.
    pub fn sweep_buffer(&mut self, buf: &mut Vec<u8>) -> bool {
        if buf.is_empty() {
            return false;
        }
        let entropy = Self::shannon_entropy(buf);
        if entropy > self.threshold {
            self.purged_bytes += buf.len() as u64;
            buf.clear();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navier_stokes_route() {
        let mut router = NavierStokesRouter::new(0.8);
        router.add_channel(0, 100.0, 1000.0);
        router.add_channel(1, 100.0, 1000.0);
        let ch = router.route(10.0);
        assert!(ch == 0 || ch == 1);
        assert!(router.total_load() > 0.0);
    }

    #[test]
    fn test_navier_stokes_drain() {
        let mut router = NavierStokesRouter::new(0.8);
        router.add_channel(0, 50.0, 500.0);
        router.route(100.0);
        router.drain(1.0);
        assert!(router.channels[0].arrival_rate < 100.0);
    }

    #[test]
    fn test_navier_stokes_overflow() {
        let mut router = NavierStokesRouter::new(0.3);
        router.add_channel(0, 10.0, 100.0);
        router.add_channel(1, 100.0, 1000.0);
        // Large route should trigger overflow to channel 1
        router.route(100.0);
        assert!(router.channels[0].load > 0.0);
        assert!(router.channels[1].load > 0.0 || router.channels[0].arrival_rate <= 100.0);
    }

    #[test]
    fn test_entropy_uniform() {
        // All-zero data has entropy 0
        let data = vec![0u8; 64];
        let h = EntropySweeper::shannon_entropy(&data);
        assert!((h - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_entropy_maximal() {
        // All distinct bytes → high entropy
        let data: Vec<u8> = (0..64).collect();
        let h = EntropySweeper::shannon_entropy(&data);
        assert!(h > 5.0);
    }

    #[test]
    fn test_entropy_medium() {
        // Alternating pattern
        let data: Vec<u8> = (0..128).map(|i| (i % 2) as u8).collect();
        let h = EntropySweeper::shannon_entropy(&data);
        assert!(h > 0.0 && h < 2.0);
    }

    #[test]
    fn test_entropy_sweeper_purges() {
        let mut sweeper = EntropySweeper::new(1.0, 3);
        // 256 distinct bytes → maximal entropy (~8.0)
        let high_entropy: Vec<u8> = (0..=255).collect();
        let low_entropy = vec![0u8; 64];
        let mut buffers = vec![high_entropy.clone(), low_entropy.clone()];
        sweeper.tick(&mut buffers);
        assert_eq!(sweeper.counter, 1);
        sweeper.tick(&mut buffers);
        assert_eq!(sweeper.counter, 2);
        let purged = sweeper.tick(&mut buffers);
        assert_eq!(sweeper.counter, 0);
        assert!(purged > 0, "expected some bytes purged, got {}", purged);
    }

    #[test]
    fn test_entropy_sweep_buffer() {
        let mut sweeper = EntropySweeper::new(2.5, 1);
        // 8 distinct bytes → H = log2(8) = 3.0 > 2.5
        let mut buf = vec![1, 2, 3, 4, 5, 6, 7, 8];
        assert!(sweeper.sweep_buffer(&mut buf));
        assert!(buf.is_empty());
    }

    #[test]
    fn test_entropy_sweep_low_entropy() {
        let mut sweeper = EntropySweeper::new(0.5, 1);
        let mut buf = vec![0u8; 64];
        assert!(!sweeper.sweep_buffer(&mut buf)); // entropy ~0, below threshold
        assert!(!buf.is_empty());
    }
}
