/// Compression and retrieval: BitNet 1.58b ternary quantization and
/// VSA-indexed retrieval-augmented generation (VSA-RAG).

use crate::cellular_automata::VsaVector;
use std::collections::HashMap;

// ── BitNet 1.58b Ternary Quantization ────────────────────────────────
// Maps GGUF attention outputs to {-1, 0, +1}, replacing floating-point
// matrix multiplications with integer add/sub across cache lines.

#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TernaryBit {
    NegOne = -1,
    Zero = 0,
    PosOne = 1,
}

#[derive(Debug, Clone)]
pub struct TernaryQuantizer {
    pub input_dim: usize,
    pub threshold: f64,
}

impl TernaryQuantizer {
    pub fn new(dim: usize, threshold: f64) -> Self {
        TernaryQuantizer { input_dim: dim, threshold }
    }

    /// Quantize an f64 vector into ternary {-1, 0, +1}.
    /// Values > threshold → +1, < -threshold → -1, else → 0.
    pub fn quantize(&self, input: &[f64]) -> Vec<TernaryBit> {
        let n = input.len().min(self.input_dim);
        let mut output = Vec::with_capacity(n);
        for &v in input.iter().take(n) {
            if v > self.threshold {
                output.push(TernaryBit::PosOne);
            } else if v < -self.threshold {
                output.push(TernaryBit::NegOne);
            } else {
                output.push(TernaryBit::Zero);
            }
        }
        output
    }

    /// Quantize a GGUF attention row: assume values are already
    /// normalized floats.
    pub fn quantize_row(&self, row: &[f32]) -> Vec<TernaryBit> {
        let vec: Vec<f64> = row.iter().map(|&v| v as f64).collect();
        self.quantize(&vec)
    }

    /// Ternary dot product using only integer add/sub.
    pub fn dot_product(a: &[TernaryBit], b: &[TernaryBit]) -> i64 {
        let n = a.len().min(b.len());
        let mut result = 0i64;
        for i in 0..n {
            result += (a[i] as i8 as i64) * (b[i] as i8 as i64);
        }
        result
    }

    /// Ternary matrix-vector multiply: output[i] = sum_j(A[i][j] * x[j])
    /// All integer operations.
    pub fn matvec_mul(a: &[Vec<TernaryBit>], x: &[TernaryBit]) -> Vec<i64> {
        let mut result = vec![0i64; a.len()];
        for i in 0..a.len() {
            result[i] = Self::dot_product(&a[i], x);
        }
        result
    }

    /// Partial sort of ternary values: count +1, 0, -1.
    pub fn count_categories(tern: &[TernaryBit]) -> (usize, usize, usize) {
        let pos = tern.iter().filter(|&&t| t == TernaryBit::PosOne).count();
        let zero = tern.iter().filter(|&&t| t == TernaryBit::Zero).count();
        let neg = tern.iter().filter(|&&t| t == TernaryBit::NegOne).count();
        (pos, zero, neg)
    }

    /// Dequantize ternary values back to approximate floats.
    ///
    /// Since ternary quantization loses magnitude information, this maps:
    /// -1 → -scale, 0 → 0.0, +1 → +scale
    ///
    /// `scale` should be the threshold or a learned scaling factor.
    pub fn dequantize(tern: &[TernaryBit], scale: f64) -> Vec<f64> {
        tern.iter().map(|&t| match t {
            TernaryBit::NegOne => -scale,
            TernaryBit::Zero => 0.0,
            TernaryBit::PosOne => scale,
        }).collect()
    }

    /// Dequantize ternary values back to approximate floats (f32 version).
    pub fn dequantize_f32(tern: &[TernaryBit], scale: f32) -> Vec<f32> {
        tern.iter().map(|&t| match t {
            TernaryBit::NegOne => -scale,
            TernaryBit::Zero => 0.0,
            TernaryBit::PosOne => scale,
        }).collect()
    }

    /// Estimate the optimal scale factor for dequantization.
    ///
    /// Given the original float values and their ternary quantization,
    /// computes the scale that minimizes reconstruction error.
    pub fn estimate_scale(original: &[f64], ternary: &[TernaryBit]) -> f64 {
        let n = original.len().min(ternary.len());
        if n == 0 { return 1.0; }

        // For each ternary value, the optimal scale is the mean of |original|
        // for non-zero ternary values
        let mut sum_abs = 0.0;
        let mut count_nonzero = 0;
        for i in 0..n {
            if ternary[i] != TernaryBit::Zero {
                sum_abs += original[i].abs();
                count_nonzero += 1;
            }
        }
        if count_nonzero > 0 { sum_abs / count_nonzero as f64 } else { 1.0 }
    }
}

// ── VSA-Indexed Retrieval-Augmented Generation ──────────────────────
// Replaces text search by comparing current execution vector to local
// HDF5 binary tables via fast bitwise popcount/hamming distance.

#[derive(Debug, Clone)]
pub struct VSARetrieval {
    /// Stored VSA entries: (vector, metadata_hash)
    pub entries: Vec<(VsaVector, u64)>,
    pub max_entries: usize,
}

impl VSARetrieval {
    pub fn new(max: usize) -> Self { VSARetrieval { entries: Vec::new(), max_entries: max } }

    /// Store a VSA vector with its metadata.
    pub fn store(&mut self, vector: VsaVector, metadata: u64) {
        if self.entries.len() >= self.max_entries {
            self.entries.remove(0); // FIFO eviction
        }
        self.entries.push((vector, metadata));
    }

    /// Popcount similarity between two byte slices (1.0 = identical).
    pub fn popcount_sim(a: &[u8], b: &[u8]) -> f32 {
        let n = a.len().min(b.len());
        if n == 0 { return 0.0; }
        let mut same = 0usize;
        for i in 0..n {
            let diff = (a[i] ^ b[i]).count_ones() as usize;
            same += 8 - diff;
        }
        same as f32 / (n as f32 * 8.0)
    }

    /// Retrieve top-k most similar entries by popcount.
    pub fn retrieve(&self, query: &VsaVector, k: usize) -> Vec<(u64, f32)> {
        let mut scored: Vec<(usize, f32)> = self.entries.iter().enumerate()
            .map(|(i, (v, _))| (i, Self::popcount_sim(query.as_bytes(), v.as_bytes())))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.truncate(k);
        scored.into_iter()
            .filter_map(|(i, sim)| self.entries.get(i).map(|(_, meta)| (*meta, sim)))
            .collect()
    }

    /// Store from raw bytes (hashes internally).
    pub fn store_bytes(&mut self, data: &[u8], metadata: u64) {
        let hash: u64 = data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64));
        let vec = VsaVector::from_bytes(data);
        self.store(vec, hash ^ metadata);
    }

    pub fn entry_count(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_quantize() {
        let quant = TernaryQuantizer::new(4, 0.5);
        let input = vec![1.0, -0.1, 0.0, -2.0];
        let tern = quant.quantize(&input);
        assert_eq!(tern[0], TernaryBit::PosOne);
        assert_eq!(tern[1], TernaryBit::Zero);
        assert_eq!(tern[2], TernaryBit::Zero);
        assert_eq!(tern[3], TernaryBit::NegOne);
    }

    #[test]
    fn test_ternary_dot_product() {
        let a = vec![TernaryBit::PosOne, TernaryBit::NegOne, TernaryBit::Zero];
        let b = vec![TernaryBit::PosOne, TernaryBit::NegOne, TernaryBit::PosOne];
        // 1*1 + (-1)*(-1) + 0*1 = 1 + 1 + 0 = 2
        assert_eq!(TernaryQuantizer::dot_product(&a, &b), 2);
    }

    #[test]
    fn test_ternary_matvec_mul() {
        let a = vec![
            vec![TernaryBit::PosOne, TernaryBit::Zero],
            vec![TernaryBit::NegOne, TernaryBit::PosOne],
        ];
        let x = vec![TernaryBit::PosOne, TernaryBit::NegOne];
        let result = TernaryQuantizer::matvec_mul(&a, &x);
        // row0: 1*1 + 0*(-1) = 1
        // row1: (-1)*1 + 1*(-1) = -2
        assert_eq!(result[0], 1);
        assert_eq!(result[1], -2);
    }

    #[test]
    fn test_ternary_count() {
        let t = vec![TernaryBit::PosOne, TernaryBit::Zero, TernaryBit::NegOne, TernaryBit::PosOne];
        let (pos, zero, neg) = TernaryQuantizer::count_categories(&t);
        assert_eq!(pos, 2);
        assert_eq!(zero, 1);
        assert_eq!(neg, 1);
    }

    #[test]
    fn test_vsa_retrieval_store() {
        let mut rag = VSARetrieval::new(100);
        rag.store(VsaVector::from_bytes(&[1, 2, 3]), 0xAA);
        assert_eq!(rag.entry_count(), 1);
    }

    #[test]
    fn test_vsa_retrieval_retrieve() {
        let mut rag = VSARetrieval::new(100);
        rag.store(VsaVector::from_bytes(&[0xAA; 32]), 100);
        rag.store(VsaVector::from_bytes(&[0xBB; 32]), 200);
        let query = VsaVector::from_bytes(&[0xAA; 32]);
        let results = rag.retrieve(&query, 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 100);
    }

    #[test]
    fn test_vsa_retrieval_popcount_sim() {
        let a = vec![0xFF; 4];
        let b = vec![0xFF; 4];
        assert!((VSARetrieval::popcount_sim(&a, &b) - 1.0).abs() < 1e-6);
        let c = vec![0x00; 4];
        assert!((VSARetrieval::popcount_sim(&a, &c) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_vsa_retrieval_eviction() {
        let mut rag = VSARetrieval::new(3);
        for i in 0..5 {
            rag.store(VsaVector::from_bytes(&[i; 8]), i as u64);
        }
        assert_eq!(rag.entry_count(), 3);
    }
}
