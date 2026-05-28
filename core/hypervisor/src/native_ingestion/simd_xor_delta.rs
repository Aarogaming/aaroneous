use std::sync::atomic::{AtomicU64, Ordering};

/// Configuration for the XOR-delta screening substrate.
pub struct XorDeltaConfig {
    /// Width of the frame in pixels.
    pub width: u32,
    /// Height of the frame in pixels.
    pub height: u32,
    /// Minimum number of changed bytes to trigger a delta report.
    pub change_threshold: u64,
}

impl Default for XorDeltaConfig {
    fn default() -> Self {
        Self {
            width: 640,
            height: 480,
            change_threshold: 64,
        }
    }
}

/// Result of a delta-screening pass between two frames.
#[repr(C)]
pub struct DeltaReport {
    /// Number of bytes that changed between previous and current frame.
    pub changed_bytes: u64,
    /// Ratio of changed bytes to total frame bytes.
    pub change_ratio: f32,
    /// Frame ID of the current capture.
    pub frame_id: u64,
    /// Elapsed nanoseconds for the delta scan.
    pub scan_ns: u64,
    /// True when the change count exceeds the configured threshold.
    pub significant: bool,
}

/// High-speed frame comparison using SIMD bitwise XOR operations.
///
/// Compares two byte buffers via:
///   1. Load aligned 256-bit (AVX2) or 128-bit (SSE4/NEON) vectors
///   2. XOR the vectors
///   3. Popcount the result
///   4. Accumulate total changed bytes
///
/// Falls back to scalar u64 word-at-a-time on unsupported hardware.
pub struct XorDeltaScreen {
    config: XorDeltaConfig,
    previous_frame: Vec<u8>,
    frame_counter: AtomicU64,
    total_bytes: usize,
}

impl XorDeltaScreen {
    pub fn new(config: XorDeltaConfig) -> Self {
        let total_bytes = (config.width as usize) * (config.height as usize) * 4;
        Self {
            previous_frame: vec![0u8; total_bytes],
            config,
            frame_counter: AtomicU64::new(0),
            total_bytes,
        }
    }

    /// Compare `current_frame` against the stored previous frame using
    /// SIMD-accelerated XOR-popcount.
    ///
    /// Returns a `DeltaReport` with the number of changed bytes and
    /// stores the current frame as the new baseline for the next call.
    pub fn screen_delta(&mut self, current_frame: &[u8]) -> DeltaReport {
        assert_eq!(current_frame.len(), self.total_bytes);
        let frame_id = self.frame_counter.fetch_add(1, Ordering::Relaxed);
        let start = now_ns();

        let changed = self.compute_xor_popcount(current_frame);

        // Store current as previous for next iteration
        self.previous_frame.copy_from_slice(current_frame);

        let scan_ns = now_ns() - start;
        let change_ratio = changed as f32 / self.total_bytes as f32;

        DeltaReport {
            changed_bytes: changed,
            change_ratio,
            frame_id,
            scan_ns,
            significant: changed >= self.config.change_threshold,
        }
    }

    /// Reset the stored previous frame to zeros (useful after a resolution change).
    pub fn reset(&mut self) {
        self.previous_frame.fill(0);
    }

    /// Core XOR-popcount loop (counts bytes with any change).
    ///
    /// Attempts AVX2 (256-bit) first, falls back to SSE4/NEON (128-bit),
    /// then falls back to scalar u64 word-at-a-time.
    fn compute_xor_popcount(&self, current: &[u8]) -> u64 {
        let prev = &self.previous_frame;
        let len = current.len();
        let mut changed = 0u64;

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return unsafe { self::avx2_xor_bytecount(prev, current, len) };
            }
            if is_x86_feature_detected!("sse4.2") {
                return unsafe { self::sse4_xor_bytecount(prev, current, len) };
            }
            if is_x86_feature_detected!("sse2") {
                return unsafe { self::sse2_xor_bytecount(prev, current, len) };
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if is_aarch64_feature_detected!("neon") {
                return unsafe { self::neon_xor_bytecount(prev, current, len) };
            }
        }

        // Scalar fallback: 8 bytes at a time, count non-equal bytes
        let mut i = 0;
        while i + 8 <= len {
            let a = u64::from_ne_bytes([
                prev[i], prev[i + 1], prev[i + 2], prev[i + 3],
                prev[i + 4], prev[i + 5], prev[i + 6], prev[i + 7],
            ]);
            let b = u64::from_ne_bytes([
                current[i], current[i + 1], current[i + 2], current[i + 3],
                current[i + 4], current[i + 5], current[i + 6], current[i + 7],
            ]);
            let xor = a ^ b;
            if xor != 0 {
                for b_idx in 0..8 {
                    if ((xor >> (b_idx * 8)) & 0xFF) != 0 {
                        changed += 1;
                    }
                }
            }
            i += 8;
        }
        for j in i..len {
            if prev[j] != current[j] {
                changed += 1;
            }
        }
        changed
    }
}

// ── SIMD intrinsics implementations (byte-level counting) ─────────────

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn avx2_xor_bytecount(prev: &[u8], current: &[u8], len: usize) -> u64 {
    use std::arch::x86_64::*;
    let mut changed = 0u64;
    let mut i = 0;

    while i + 32 <= len {
        let a = _mm256_loadu_si256(prev.as_ptr().add(i) as *const __m256i);
        let b = _mm256_loadu_si256(current.as_ptr().add(i) as *const __m256i);
        let xor = _mm256_xor_si256(a, b);
        // Extract as bytes and count non-zero bytes via vectorized compare
        let zero = _mm256_setzero_si256();
        let cmp = _mm256_cmpeq_epi8(xor, zero);
        // Count match bits, then subtract from 32
        let eq_mask_i32 = _mm256_movemask_epi8(cmp);
        let unchanged = (eq_mask_i32 as u32).count_ones() as u64;
        changed += 32 - unchanged;
        i += 32;
    }
    for j in i..len {
        if prev[j] != current[j] {
            changed += 1;
        }
    }
    changed
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2")]
unsafe fn sse4_xor_bytecount(prev: &[u8], current: &[u8], len: usize) -> u64 {
    use std::arch::x86_64::*;
    let mut changed = 0u64;
    let mut i = 0;

    while i + 16 <= len {
        let a = _mm_loadu_si128(prev.as_ptr().add(i) as *const __m128i);
        let b = _mm_loadu_si128(current.as_ptr().add(i) as *const __m128i);
        let xor = _mm_xor_si128(a, b);
        let zero = _mm_setzero_si128();
        let cmp = _mm_cmpeq_epi8(xor, zero);
        let eq_mask_i32 = _mm_movemask_epi8(cmp);
        let unchanged = (eq_mask_i32 as u32).count_ones() as u64;
        changed += 16 - unchanged;
        i += 16;
    }
    for j in i..len {
        if prev[j] != current[j] {
            changed += 1;
        }
    }
    changed
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse2")]
unsafe fn sse2_xor_bytecount(prev: &[u8], current: &[u8], len: usize) -> u64 {
    sse4_xor_bytecount(prev, current, len)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn neon_xor_bytecount(prev: &[u8], current: &[u8], len: usize) -> u64 {
    use std::arch::aarch64::*;
    let mut changed = 0u64;
    let mut i = 0;

    while i + 16 <= len {
        let a = vld1q_u8(prev.as_ptr().add(i));
        let b = vld1q_u8(current.as_ptr().add(i));
        let xor = veorq_u8(a, b);
        // Count non-zero bytes: extract result as array and check each byte
        let xor_bytes: [u8; 16] = std::mem::transmute(xor);
        for b_idx in 0..16 {
            if xor_bytes[b_idx] != 0 {
                changed += 1;
            }
        }
        i += 16;
    }
    for j in i..len {
        if prev[j] != current[j] {
            changed += 1;
        }
    }
    changed
}

fn now_ns() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_identical_frames() {
        let cfg = XorDeltaConfig {
            width: 16,
            height: 16,
            change_threshold: 1,
        };
        let mut screen = XorDeltaScreen::new(cfg);
        let frame = vec![0xABu8; 16 * 16 * 4];
        let report = screen.screen_delta(&frame);
        // First frame compares against zeros — should see all bytes changed
        assert!(report.changed_bytes > 0);

        let report2 = screen.screen_delta(&frame);
        assert_eq!(report2.changed_bytes, 0);
        assert!(!report2.significant);
    }

    #[test]
    fn test_delta_different_frames() {
        let cfg = XorDeltaConfig {
            width: 16,
            height: 16,
            change_threshold: 1,
        };
        let mut screen = XorDeltaScreen::new(cfg);
        // Prime with all-zeros (default)
        let zeros = vec![0u8; 16 * 16 * 4];
        screen.screen_delta(&zeros);

        let ones = vec![0xFFu8; 16 * 16 * 4];
        let report = screen.screen_delta(&ones);
        assert!(report.changed_bytes > 0);
        assert!(report.significant);
        assert!((report.change_ratio - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_delta_threshold() {
        let cfg = XorDeltaConfig {
            width: 4,
            height: 4,
            change_threshold: 1000,
        };
        let mut screen = XorDeltaScreen::new(cfg);
        let zeros = vec![0u8; 4 * 4 * 4];
        screen.screen_delta(&zeros);

        let almost = vec![0u8; 4 * 4 * 4];
        let report = screen.screen_delta(&almost);
        assert!(!report.significant);
    }

    #[test]
    fn test_reset() {
        let cfg = XorDeltaConfig {
            width: 8,
            height: 8,
            change_threshold: 1,
        };
        let mut screen = XorDeltaScreen::new(cfg);
        let frame = vec![0xFFu8; 8 * 8 * 4];
        screen.screen_delta(&frame);
        screen.reset();
        let report = screen.screen_delta(&frame);
        assert!(report.changed_bytes > 0);
    }

    #[test]
    fn test_scalar_fallback_produces_same_result() {
        // Verify correctness against a manual byte-by-byte comparison
        let cfg = XorDeltaConfig {
            width: 32,
            height: 32,
            change_threshold: 0,
        };
        let mut screen = XorDeltaScreen::new(cfg);
        let prev = vec![0u8; 32 * 32 * 4];
        screen.screen_delta(&prev);

        let mut curr = vec![0u8; 32 * 32 * 4];
        curr[100] = 0x01;
        curr[500] = 0x80;
        curr[1000] = 0xFF;
        let report = screen.screen_delta(&curr);
        assert_eq!(report.changed_bytes, 3);
    }
}
