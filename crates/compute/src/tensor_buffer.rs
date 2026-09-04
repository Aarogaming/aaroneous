//! crates/compute/src/tensor_buffer.rs
//! Universal Tensor & Numerical Buffer Abstraction.
//!
//! Provides zero-copy strided views and heap-allocated tensor buffers supporting:
//! - 1D through 4D tensor indexing (Rank 1 to 4)
//! - Row-major and custom-strided indexing
//! - Elementwise addition, scaling, and normalized L2 metrics
//! - Slicing and reshape operations without memory reallocations where contiguous

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Immutable strided view over contiguous numerical slices
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UniversalTensorView<'a, T> {
    data: &'a [T],
    shape: [usize; 4],
    strides: [usize; 4],
    rank: usize,
}

impl<'a, T> UniversalTensorView<'a, T> {
    /// Creates a 1D tensor view
    pub fn from_1d(data: &'a [T]) -> Self {
        Self {
            data,
            shape: [data.len(), 1, 1, 1],
            strides: [1, 1, 1, 1],
            rank: 1,
        }
    }

    /// Creates a 2D matrix view (rows, cols)
    pub fn from_2d(data: &'a [T], rows: usize, cols: usize) -> Result<Self> {
        if data.len() < rows * cols {
            return Err(anyhow!("Data slice length {} insufficient for shape [{}, {}]", data.len(), rows, cols));
        }
        Ok(Self {
            data,
            shape: [rows, cols, 1, 1],
            strides: [cols, 1, 1, 1],
            rank: 2,
        })
    }

    /// Creates a 3D tensor view (depth/channels, rows, cols)
    pub fn from_3d(data: &'a [T], c: usize, h: usize, w: usize) -> Result<Self> {
        if data.len() < c * h * w {
            return Err(anyhow!("Data slice length {} insufficient for shape [{}, {}, {}]", data.len(), c, h, w));
        }
        Ok(Self {
            data,
            shape: [c, h, w, 1],
            strides: [h * w, w, 1, 1],
            rank: 3,
        })
    }

    /// Creates a 4D tensor view (batch, channels, height, width)
    pub fn from_4d(data: &'a [T], b: usize, c: usize, h: usize, w: usize) -> Result<Self> {
        if data.len() < b * c * h * w {
            return Err(anyhow!("Data slice length {} insufficient for shape [{}, {}, {}, {}]", data.len(), b, c, h, w));
        }
        Ok(Self {
            data,
            shape: [b, c, h, w],
            strides: [c * h * w, h * w, w, 1],
            rank: 4,
        })
    }

    pub fn rank(&self) -> usize {
        self.rank
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape[..self.rank]
    }

    pub fn len(&self) -> usize {
        self.shape[..self.rank].iter().product()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_slice(&self) -> &'a [T] {
        self.data
    }

    /// Retrieves element by 1D linear index
    pub fn get_1d(&self, idx: usize) -> Option<&'a T> {
        if idx < self.len() {
            self.data.get(idx)
        } else {
            None
        }
    }

    /// Retrieves element by 2D coordinates (row, col)
    pub fn get_2d(&self, r: usize, c: usize) -> Option<&'a T> {
        if self.rank >= 2 && r < self.shape[0] && c < self.shape[1] {
            let offset = r * self.strides[0] + c * self.strides[1];
            self.data.get(offset)
        } else {
            None
        }
    }

    /// Retrieves element by 3D coordinates (channel, row, col)
    pub fn get_3d(&self, ch: usize, r: usize, c: usize) -> Option<&'a T> {
        if self.rank >= 3 && ch < self.shape[0] && r < self.shape[1] && c < self.shape[2] {
            let offset = ch * self.strides[0] + r * self.strides[1] + c * self.strides[2];
            self.data.get(offset)
        } else {
            None
        }
    }
}

/// Owned multidimensional contiguous tensor buffer
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TensorBuffer<T> {
    data: Vec<T>,
    shape: [usize; 4],
    strides: [usize; 4],
    rank: usize,
}

impl<T: Default + Clone> TensorBuffer<T> {
    pub fn zeros_1d(len: usize) -> Self {
        Self {
            data: vec![T::default(); len],
            shape: [len, 1, 1, 1],
            strides: [1, 1, 1, 1],
            rank: 1,
        }
    }

    pub fn zeros_2d(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![T::default(); rows * cols],
            shape: [rows, cols, 1, 1],
            strides: [cols, 1, 1, 1],
            rank: 2,
        }
    }

    pub fn zeros_3d(c: usize, h: usize, w: usize) -> Self {
        Self {
            data: vec![T::default(); c * h * w],
            shape: [c, h, w, 1],
            strides: [h * w, w, 1, 1],
            rank: 3,
        }
    }
}

impl<T: Clone> TensorBuffer<T> {
    pub fn from_vec_1d(data: Vec<T>) -> Self {
        let len = data.len();
        Self {
            data,
            shape: [len, 1, 1, 1],
            strides: [1, 1, 1, 1],
            rank: 1,
        }
    }

    pub fn from_vec_2d(data: Vec<T>, rows: usize, cols: usize) -> Result<Self> {
        if data.len() != rows * cols {
            return Err(anyhow!("Data len {} != rows {} * cols {}", data.len(), rows, cols));
        }
        Ok(Self {
            data,
            shape: [rows, cols, 1, 1],
            strides: [cols, 1, 1, 1],
            rank: 2,
        })
    }

    pub fn view(&self) -> UniversalTensorView<'_, T> {
        UniversalTensorView {
            data: &self.data,
            shape: self.shape,
            strides: self.strides,
            rank: self.rank,
        }
    }

    pub fn rank(&self) -> usize {
        self.rank
    }

    pub fn shape(&self) -> &[usize] {
        &self.shape[..self.rank]
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.data
    }

    pub fn get_2d(&self, r: usize, c: usize) -> Option<&T> {
        if self.rank >= 2 && r < self.shape[0] && c < self.shape[1] {
            let offset = r * self.strides[0] + c * self.strides[1];
            self.data.get(offset)
        } else {
            None
        }
    }

    pub fn set_2d(&mut self, r: usize, c: usize, val: T) -> bool {
        if self.rank >= 2 && r < self.shape[0] && c < self.shape[1] {
            let offset = r * self.strides[0] + c * self.strides[1];
            if offset < self.data.len() {
                self.data[offset] = val;
                return true;
            }
        }
        false
    }
}

impl TensorBuffer<f32> {
    /// Elementwise vector scalar multiplication
    pub fn scale(&mut self, scalar: f32) {
        for v in &mut self.data {
            *v *= scalar;
        }
    }

    /// Elementwise in-place addition
    pub fn add_assign(&mut self, other: &Self) -> Result<()> {
        if self.shape() != other.shape() {
            return Err(anyhow!("Shape mismatch in add_assign: {:?} vs {:?}", self.shape(), other.shape()));
        }
        for (a, &b) in self.data.iter_mut().zip(&other.data) {
            *a += b;
        }
        Ok(())
    }

    /// L2 Frobenius norm
    pub fn l2_norm(&self) -> f32 {
        self.data.iter().map(|&x| x * x).sum::<f32>().sqrt()
    }

    /// Normalize tensor in-place to unit L2 norm
    pub fn normalize_l2(&mut self) {
        let norm = self.l2_norm();
        if norm > 1e-12 {
            self.scale(1.0 / norm);
        }
    }

    /// Computes inner dot product between two tensors of identical shape
    pub fn dot_product(&self, other: &Self) -> Result<f32> {
        if self.shape() != other.shape() {
            return Err(anyhow!("Shape mismatch in dot_product: {:?} vs {:?}", self.shape(), other.shape()));
        }
        let dot: f32 = self.data.iter().zip(&other.data).map(|(&a, &b)| a * b).sum();
        Ok(dot)
    }

    /// Computes cosine similarity between two tensors of identical shape
    pub fn cosine_similarity(&self, other: &Self) -> Result<f32> {
        let dot = self.dot_product(other)?;
        let norm_a = self.l2_norm();
        let norm_b = other.l2_norm();

        if norm_a <= 1e-12 || norm_b <= 1e-12 {
            return Ok(0.0);
        }

        Ok((dot / (norm_a * norm_b)).clamp(-1.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_view_2d_and_3d() {
        let data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let view = UniversalTensorView::from_2d(&data, 2, 3).unwrap();
        assert_eq!(view.rank(), 2);
        assert_eq!(view.shape(), &[2, 3]);
        assert_eq!(view.get_2d(0, 1), Some(&2.0));
        assert_eq!(view.get_2d(1, 2), Some(&6.0));
        assert_eq!(view.get_2d(2, 0), None);
    }

    #[test]
    fn test_tensor_buffer_math_and_normalization() {
        let mut buf = TensorBuffer::zeros_2d(2, 2);
        buf.set_2d(0, 0, 3.0);
        buf.set_2d(1, 1, 4.0);
        assert_eq!(buf.l2_norm(), 5.0);

        buf.normalize_l2();
        assert!((buf.l2_norm() - 1.0).abs() < 1e-6);
        assert_eq!(buf.get_2d(0, 0), Some(&0.6));
        assert_eq!(buf.get_2d(1, 1), Some(&0.8));
    }

    #[test]
    fn test_tensor_buffer_dot_and_cosine() {
        let a = TensorBuffer::from_vec_1d(vec![1.0f32, 0.0, 0.0]);
        let b = TensorBuffer::from_vec_1d(vec![1.0f32, 0.0, 0.0]);
        let c = TensorBuffer::from_vec_1d(vec![0.0f32, 1.0, 0.0]);

        assert_eq!(a.dot_product(&b).unwrap(), 1.0);
        assert_eq!(a.dot_product(&c).unwrap(), 0.0);

        let sim_ab = a.cosine_similarity(&b).unwrap();
        let sim_ac = a.cosine_similarity(&c).unwrap();

        assert!((sim_ab - 1.0).abs() < 1e-6);
        assert!(sim_ac.abs() < 1e-6);
    }
}
