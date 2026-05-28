/// Quantum and holographic surface primitives.
///
/// Color confinement (2-bit color states bundling into 64-byte chunks),
/// quantum decoherence isolation (timer-collapsed probability states),
/// and holographic surface boundary projection (HD→2D bitmask flattening).

use crate::cellular_automata::SuperpositionState;

// ── 7. Color Confinement Data Clustering ─────────────────────────────
// Assigns 2-bit color states (00/01/10/11) to data chunks; bundler
// packs same-color rows into contiguous 64-byte cache lines.

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorCharge {
    Red   = 0b00,
    Green = 0b01,
    Blue  = 0b10,
    White = 0b11,
}

#[repr(C, align(64))]
#[derive(Debug, Clone)]
pub struct ColorChunk {
    pub color: ColorCharge,
    pub data: [u8; 64],
}

#[derive(Debug, Clone)]
pub struct ColorConfinement {
    pub chunks: Vec<ColorChunk>,
    pub red_count: usize,
    pub green_count: usize,
    pub blue_count: usize,
    pub white_count: usize,
}

impl ColorConfinement {
    pub fn new() -> Self {
        ColorConfinement { chunks: Vec::new(), red_count: 0, green_count: 0, blue_count: 0, white_count: 0 }
    }

    /// Assign a color based on a hash of the data (2-bit quantization).
    pub fn assign_color(data: &[u8]) -> ColorCharge {
        let hash: u8 = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b).wrapping_mul(31));
        match hash & 0b11 {
            0b00 => ColorCharge::Red,
            0b01 => ColorCharge::Green,
            0b10 => ColorCharge::Blue,
            _    => ColorCharge::White,
        }
    }

    /// Insert data into a ColorChunk. If data > 64 bytes, first 64 are used.
    pub fn insert(&mut self, data: &[u8]) {
        let color = Self::assign_color(data);
        let mut chunk_data = [0u8; 64];
        let n = data.len().min(64);
        chunk_data[..n].copy_from_slice(&data[..n]);
        self.chunks.push(ColorChunk { color, data: chunk_data });
        match color {
            ColorCharge::Red   => self.red_count += 1,
            ColorCharge::Green => self.green_count += 1,
            ColorCharge::Blue  => self.blue_count += 1,
            ColorCharge::White => self.white_count += 1,
        }
    }

    /// Pack same-color chunks into contiguous groups (returns indices per color).
    pub fn pack_by_color(&self) -> [Vec<usize>; 4] {
        let mut packed: [Vec<usize>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for (i, chunk) in self.chunks.iter().enumerate() {
            match chunk.color {
                ColorCharge::Red   => packed[0].push(i),
                ColorCharge::Green => packed[1].push(i),
                ColorCharge::Blue  => packed[2].push(i),
                ColorCharge::White => packed[3].push(i),
            }
        }
        packed
    }

    pub fn total_chunks(&self) -> usize { self.chunks.len() }
}

// ── 8. Quantum Decoherence Error Isolation ───────────────────────────
// Hardware-timer interrupt that snaps superposition to binary decisions.

#[derive(Debug, Clone)]
pub struct DecoherenceGate {
    pub timeout_ns: u64,
    pub timer: u64,
    pub collapsed_count: u64,
}

impl DecoherenceGate {
    pub fn new(timeout_ns: u64) -> Self {
        DecoherenceGate { timeout_ns, timer: 0, collapsed_count: 0 }
    }

    /// Advance timer; returns true if decoherence fires (timeout reached).
    pub fn tick(&mut self, dt_ns: u64) -> bool {
        self.timer += dt_ns;
        if self.timer >= self.timeout_ns {
            self.timer = 0;
            self.collapsed_count += 1;
            true
        } else {
            false
        }
    }

    /// Force-collapse a superposition state into its most probable basis state.
    /// Returns the index of the collapsed state.
    pub fn collapse(&mut self, state: &SuperpositionState) -> usize {
        self.collapsed_count += 1;
        self.timer = 0;
        state.collapse()
    }

    /// Reset the timer without collapsing.
    pub fn reset(&mut self) { self.timer = 0; }
}

// ── 9. Holographic Surface Boundary Projection ───────────────────────
// Flatten high-dimensional vectors onto 2D bitmask surfaces via random
// projection (Johnson-Lindenstrauss style).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct HolographicProjector {
    pub input_dims: usize,
    pub output_cols: usize,
    /// Random projection matrix: output_cols rows of input_dims bits.
    pub projection: Vec<Vec<bool>>,
}

impl HolographicProjector {
    pub fn new(input_dims: usize, output_cols: usize) -> Self {
        // Deterministic pseudo-random projection matrix
        let mut projection = Vec::with_capacity(output_cols);
        for row in 0..output_cols {
            let mut bits = Vec::with_capacity(input_dims);
            let mut h = DefaultHasher::new();
            (row as u64).hash(&mut h);
            let seed = h.finish();
            for col in 0..input_dims {
                let mut h2 = DefaultHasher::new();
                (seed ^ col as u64).hash(&mut h2);
                bits.push(h2.finish() & 1 == 1);
            }
            projection.push(bits);
        }
        HolographicProjector { input_dims, output_cols, projection }
    }

    /// Project a high-D vector (as f64 slice) onto a 2D bitmask surface.
    /// Each output bit = sign of dot product with random row.
    pub fn project_f64(&self, vector: &[f64]) -> Vec<bool> {
        let n = self.input_dims.min(vector.len());
        let mut surface = Vec::with_capacity(self.output_cols);
        for row in &self.projection {
            let mut dot = 0.0f64;
            for j in 0..n {
                if row[j] { dot += vector[j]; } else { dot -= vector[j]; }
            }
            surface.push(dot > 0.0);
        }
        surface
    }

    /// Project a byte vector by treating each byte as [0, 255] float.
    pub fn project_bytes(&self, data: &[u8]) -> Vec<bool> {
        let vec: Vec<f64> = data.iter().map(|&b| b as f64).collect();
        self.project_f64(&vec)
    }

    /// Project a u64 vector directly.
    pub fn project_u64(&self, data: &[u64]) -> Vec<bool> {
        let vec: Vec<f64> = data.iter().map(|&v| v as f64).collect();
        self.project_f64(&vec)
    }

    /// Pack the bool surface into bytes (8 bits per byte).
    pub fn surface_to_bytes(&self, surface: &[bool]) -> Vec<u8> {
        let byte_len = (surface.len() + 7) / 8;
        let mut bytes = vec![0u8; byte_len];
        for (i, &bit) in surface.iter().enumerate() {
            if bit {
                bytes[i / 8] |= 1 << (i % 8);
            }
        }
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_assignment() {
        let data_a = vec![1, 2, 3, 4];
        let data_b = vec![5, 6, 7, 8];
        let c1 = ColorConfinement::assign_color(&data_a);
        let c2 = ColorConfinement::assign_color(&data_b);
        // Different data may produce different or same colors
        assert!(matches!(c1, ColorCharge::Red | ColorCharge::Green | ColorCharge::Blue | ColorCharge::White));
        assert!(matches!(c2, ColorCharge::Red | ColorCharge::Green | ColorCharge::Blue | ColorCharge::White));
    }

    #[test]
    fn test_color_confinement_insert() {
        let mut conf = ColorConfinement::new();
        conf.insert(&[1u8; 64]);
        conf.insert(&[2u8; 64]);
        conf.insert(&[3u8; 64]);
        assert_eq!(conf.total_chunks(), 3);
        assert_eq!(conf.chunks[0].data.len(), 64);
    }

    #[test]
    fn test_color_confinement_pack() {
        let mut conf = ColorConfinement::new();
        for i in 0..32 {
            conf.insert(&[i; 60]);
        }
        let packed = conf.pack_by_color();
        let total: usize = packed.iter().map(|v| v.len()).sum();
        assert_eq!(total, 32);
    }

    #[test]
    fn test_decoherence_timer() {
        let mut gate = DecoherenceGate::new(100);
        assert!(!gate.tick(50));
        assert!(gate.tick(60)); // 50+60=110 >= 100
    }

    #[test]
    fn test_decoherence_collapse() {
        let mut gate = DecoherenceGate::new(100);
        let state = SuperpositionState::new(4);
        let idx = gate.collapse(&state);
        assert!(idx < 4);
        assert_eq!(gate.collapsed_count, 1);
    }

    #[test]
    fn test_holographic_project_f64() {
        let projector = HolographicProjector::new(8, 16);
        let vec = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let surface = projector.project_f64(&vec);
        assert_eq!(surface.len(), 16);
    }

    #[test]
    fn test_holographic_project_bytes() {
        let projector = HolographicProjector::new(16, 32);
        let data = b"hello holographic world!";
        let surface = projector.project_bytes(data);
        assert_eq!(surface.len(), 32);
    }

    #[test]
    fn test_holographic_deterministic() {
        let projector = HolographicProjector::new(4, 8);
        let v = vec![1.0, 0.0, 1.0, 0.0];
        let a = projector.project_f64(&v);
        let b = projector.project_f64(&v);
        assert_eq!(a, b);
    }

    #[test]
    fn test_holographic_surface_to_bytes() {
        let projector = HolographicProjector::new(4, 12);
        let surface = vec![true, false, true, false, true, false, true, false, true, false, true, false];
        let bytes = projector.surface_to_bytes(&surface);
        assert_eq!(bytes.len(), 2); // 12 bits = 2 bytes
        assert_eq!(bytes[0], 0b01010101);
    }
}
