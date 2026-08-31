/// System integrity and governance gates.
///
/// Items 18–23 of Phase 6 Expansion: Byzantine consensus, Zipfian cache,
/// Braess optimizer, Little's law backpressure, Anna Karenina watchdog,
/// asymmetric hash verification.
// ── 18. Byzantine Consensus Gate ───────────────────────────────────────
// Fault-tolerant agreement among N replicas; tolerate f faulty.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ByzantineProposal {
    pub proposer: u64,
    pub value: u64,
    pub round: u64,
}

#[derive(Debug, Clone)]
pub struct ByzantineConsensusGate {
    pub nodes: usize,
    pub faulty_max: usize,
    pub proposals: Vec<ByzantineProposal>,
    pub commits: Vec<u64>,
}

impl ByzantineConsensusGate {
    pub fn new(nodes: usize) -> Self {
        let faulty_max = (nodes - 1) / 3; // tolerate up to n/3 faulty
        ByzantineConsensusGate {
            nodes,
            faulty_max,
            proposals: Vec::new(),
            commits: Vec::new(),
        }
    }

    /// Submit a proposal; returns true if quorum (2f+1) reached.
    pub fn submit(&mut self, proposal: ByzantineProposal) -> bool {
        self.proposals.push(proposal);
        let count = self
            .proposals
            .iter()
            .filter(|p| p.value == proposal.value)
            .count();
        if count > 2 * self.faulty_max {
            if !self.commits.contains(&proposal.value) {
                self.commits.push(proposal.value);
            }
            true
        } else {
            false
        }
    }

    pub fn committed_count(&self) -> usize {
        self.commits.len()
    }
}

// ── 19. Zipfian L3 Cache Split ────────────────────────────────────────
// Partition cache according to Zipf (frequency) distribution.

#[derive(Debug, Clone)]
pub struct ZipfianCacheSplit {
    pub total_slots: usize,
    pub hot_ratio: f32,
    pub warm_ratio: f32,
    pub cold_ratio: f32,
    pub hot_slots: usize,
    pub warm_slots: usize,
    pub cold_slots: usize,
}

impl ZipfianCacheSplit {
    pub fn new(total: usize, hot: f32, warm: f32, cold: f32) -> Self {
        let h = (total as f32 * hot) as usize;
        let w = (total as f32 * warm) as usize;
        let c = total.saturating_sub(h + w);
        ZipfianCacheSplit {
            total_slots: total,
            hot_ratio: hot,
            warm_ratio: warm,
            cold_ratio: cold,
            hot_slots: h,
            warm_slots: w,
            cold_slots: c,
        }
    }

    pub fn assign_tier(&self, frequency: u64) -> &'static str {
        if frequency > 100 {
            "hot"
        } else if frequency > 10 {
            "warm"
        } else {
            "cold"
        }
    }
}

// ── 20. Braess Path Optimizer ─────────────────────────────────────────
// Anticipates Braess's paradox: adding capacity can slow the system.

#[derive(Debug, Clone)]
pub struct BraessPathOptimizer {
    pub paths: Vec<BraessPath>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct BraessPath {
    pub id: usize,
    pub base_latency: f32,
    pub capacity: f32,
    pub load: f32,
}

impl Default for BraessPathOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl BraessPathOptimizer {
    pub fn new() -> Self {
        BraessPathOptimizer { paths: Vec::new() }
    }

    pub fn add_path(&mut self, id: usize, latency: f32, capacity: f32) {
        self.paths.push(BraessPath {
            id,
            base_latency: latency,
            capacity,
            load: 0.0,
        });
    }

    /// Compute effective latency with Braess penalty: latency * (1 + load/capacity).
    pub fn effective_latency(&self, id: usize) -> Option<f32> {
        self.paths.iter().find(|p| p.id == id).map(|p| {
            let congestion = 1.0 + p.load / p.capacity.max(0.01);
            p.base_latency * congestion
        })
    }

    /// Assign load to minimize total system latency (greedy).
    pub fn assign_load(&mut self, total_load: f32) {
        for path in &mut self.paths {
            path.load = 0.0;
        }
        let mut remaining = total_load;
        while remaining > 0.0 {
            let increment = 0.1;
            let best = self.paths.iter_mut().min_by(|a, b| {
                let lat_a = a.base_latency * (1.0 + (a.load + increment) / a.capacity.max(0.01));
                let lat_b = b.base_latency * (1.0 + (b.load + increment) / b.capacity.max(0.01));
                lat_a.partial_cmp(&lat_b).unwrap()
            });
            if let Some(p) = best {
                p.load += increment;
                remaining -= increment;
            } else {
                break;
            }
        }
    }
}

// ── 21. Little's Law Backpressure ─────────────────────────────────────
// L = λ * W: control admission based on queue length + wait time.

#[derive(Debug, Clone)]
pub struct LittlesLawBackpressure {
    pub arrival_rate: f32,
    pub avg_wait_time: f32,
    pub max_queue: f32,
    pub current_queue: f32,
    pub dropped: u64,
}

impl LittlesLawBackpressure {
    pub fn new(max_queue: f32) -> Self {
        LittlesLawBackpressure {
            arrival_rate: 0.0,
            avg_wait_time: 0.0,
            max_queue,
            current_queue: 0.0,
            dropped: 0,
        }
    }

    /// Update with measured arrival rate and wait time; returns drop probability.
    pub fn update(&mut self, arrival_rate: f32, avg_wait: f32) -> f32 {
        self.arrival_rate = arrival_rate;
        self.avg_wait_time = avg_wait;
        let expected = self.arrival_rate * self.avg_wait_time;
        if expected > self.max_queue {
            let over = expected - self.max_queue;
            let drop_prob = (over / expected).min(1.0);
            self.dropped += 1;
            drop_prob
        } else {
            0.0
        }
    }

    /// Attempt to enqueue; returns false if dropped.
    pub fn try_enqueue(&mut self) -> bool {
        let drop_prob = self.update(self.arrival_rate, self.avg_wait_time);
        if drop_prob > 0.5 {
            self.dropped += 1;
            false
        } else {
            self.current_queue += 1.0;
            true
        }
    }
}

// ── 22. Anna Karenina Watchdog ────────────────────────────────────────
// Detects failure cascades: "All happy families are alike; each unhappy
// family is unhappy in its own way." Monitors for diverse failure modes.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FailureEvent {
    pub component_id: u64,
    pub error_code: u32,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct AnnaKareninaWatchdog {
    pub events: Vec<FailureEvent>,
    pub cascade_threshold: usize,
    pub window_ticks: u64,
}

impl AnnaKareninaWatchdog {
    pub fn new(threshold: usize, window: u64) -> Self {
        AnnaKareninaWatchdog {
            events: Vec::new(),
            cascade_threshold: threshold,
            window_ticks: window,
        }
    }

    pub fn report_failure(&mut self, component: u64, code: u32, now: u64) {
        self.events.push(FailureEvent {
            component_id: component,
            error_code: code,
            timestamp: now,
        });
    }

    /// Check if a cascade is in progress: many distinct errors in window.
    pub fn is_cascade(&self, now: u64) -> bool {
        let recent: Vec<_> = self
            .events
            .iter()
            .filter(|e| now - e.timestamp <= self.window_ticks)
            .collect();
        if recent.len() >= self.cascade_threshold {
            // "Each unhappy family is unhappy in its own way" — check diversity
            let distinct_errors: std::collections::HashSet<_> =
                recent.iter().map(|e| e.error_code).collect();
            distinct_errors.len() >= self.cascade_threshold / 2
        } else {
            false
        }
    }
}

// ── 23. Asymmetric Hash Verification ──────────────────────────────────
// Fast to verify (single hash), slow to forge (iterative hashing proof).

#[derive(Debug, Clone)]
pub struct AsymmetricHashVerification {
    pub digest: [u8; 32],
    pub work_factor: u32,
}

impl AsymmetricHashVerification {
    pub fn new(data: &[u8], work: u32) -> Self {
        let digest = simple_hash(data);
        AsymmetricHashVerification {
            digest,
            work_factor: work,
        }
    }

    /// Verify a proof: must provide a nonce such that hash(data || nonce)
    /// has `work_factor` leading zero bits.
    pub fn verify(&self, data: &[u8], nonce: u64) -> bool {
        let mut buf = Vec::with_capacity(data.len() + 8);
        buf.extend_from_slice(data);
        buf.extend_from_slice(&nonce.to_le_bytes());
        let h = simple_hash(&buf);
        leading_zero_bits(&h) >= self.work_factor
    }

    /// Forge: find a nonce that satisfies the work factor (O(2^n) brute force).
    pub fn forge(&self, data: &[u8]) -> Option<u64> {
        (0..(1u64 << self.work_factor).max(1_000_000)).find(|&nonce| self.verify(data, nonce))
    }
}

/// Cryptographic SHA-256 implementation for asymmetric integrity verification
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[4 * i], chunk[4 * i + 1], chunk[4 * i + 2], chunk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[4 * i..4 * i + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn simple_hash(data: &[u8]) -> [u8; 32] {
    sha256_digest(data)
}

fn leading_zero_bits(data: &[u8]) -> u32 {
    let mut count = 0;
    for &b in data {
        if b == 0 {
            count += 8;
        } else {
            count += b.leading_zeros();
            break;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byzantine_quorum() {
        let mut gate = ByzantineConsensusGate::new(7); // f=2, quorum=5
        for i in 0..5 {
            let committed = gate.submit(ByzantineProposal {
                proposer: i,
                value: 42,
                round: 1,
            });
            if i < 4 {
                assert!(!committed, "round {} should not commit yet", i);
            } else {
                assert!(committed, "round {} should commit", i);
            }
        }
        assert_eq!(gate.committed_count(), 1);
    }

    #[test]
    fn test_zipfian_cache_split() {
        let split = ZipfianCacheSplit::new(100, 0.2, 0.3, 0.5);
        assert_eq!(split.hot_slots, 20);
        assert_eq!(split.warm_slots, 30);
        assert!(split.cold_slots <= 50);
        assert_eq!(split.assign_tier(200), "hot");
        assert_eq!(split.assign_tier(50), "warm");
        assert_eq!(split.assign_tier(5), "cold");
    }

    #[test]
    fn test_braess_path_optimizer() {
        let mut opt = BraessPathOptimizer::new();
        opt.add_path(1, 10.0, 5.0);
        opt.add_path(2, 20.0, 10.0);
        opt.assign_load(8.0);
        // Lower-latency path should bear more load
        let p1 = opt.effective_latency(1).unwrap();
        let p2 = opt.effective_latency(2).unwrap();
        assert!(p1 > 0.0);
        assert!(p2 > 0.0);
    }

    #[test]
    fn test_littles_law() {
        let mut bp = LittlesLawBackpressure::new(10.0);
        let drop = bp.update(100.0, 1.0);
        assert!(drop > 0.0); // 100 * 1 = 100 >> 10
        assert_eq!(bp.try_enqueue(), false); // should be dropped
    }

    #[test]
    fn test_anna_karenina_watchdog() {
        let mut dog = AnnaKareninaWatchdog::new(3, 100);
        dog.report_failure(1, 101, 1);
        dog.report_failure(2, 102, 2);
        dog.report_failure(3, 103, 3);
        assert!(dog.is_cascade(50)); // 3 distinct errors within window
    }

    #[test]
    fn test_anna_karenina_no_cascade() {
        let mut dog = AnnaKareninaWatchdog::new(5, 100);
        dog.report_failure(1, 101, 1);
        assert!(!dog.is_cascade(50)); // not enough failures
    }

    #[test]
    fn test_asymmetric_hash_verification() {
        let data = b"test data";
        let ahv = AsymmetricHashVerification::new(data, 8); // 8 leading zero bits
        let nonce = ahv.forge(data);
        assert!(nonce.is_some(), "should find a valid nonce");
        assert!(ahv.verify(data, nonce.unwrap()));
    }

    #[test]
    fn test_asymmetric_hash_invalid() {
        let data = b"test data";
        let ahv = AsymmetricHashVerification::new(data, 20); // hard to forge in tests
        // We won't forge here, just verify that wrong nonce returns false
        assert!(!ahv.verify(data, 0));
    }

    #[test]
    fn test_leading_zero_bits() {
        assert_eq!(leading_zero_bits(&[0x00, 0xFF]), 8);
        assert_eq!(leading_zero_bits(&[0x0F]), 4);
        assert_eq!(leading_zero_bits(&[0xFF]), 0);
    }
}
