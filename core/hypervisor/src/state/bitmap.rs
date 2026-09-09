// src/state/bitmap.rs
//! Fixed-size bitmap for task occupancy tracking.

use core::sync::atomic::{AtomicUsize, Ordering};

/// Bitmap with `MAX_TASKS` bits. Uses an atomic usize as a simple bitset.
/// For `MAX_TASKS <= usize::BITS as usize` this fits in one word.
pub struct Bitmap {
    bits: AtomicUsize,
}

impl Bitmap {
    /// Number of bits supported (must match `MAX_TASKS`).
    pub const CAPACITY: usize = crate::constants::MAX_TASKS;

    /// Create a new empty bitmap.
    #[inline]
    pub const fn new() -> Self {
        Self { bits: AtomicUsize::new(0) }
    }

    /// Set the bit at `idx`.
    #[inline]
    pub fn set(&self, idx: usize) {
        debug_assert!(idx < Self::CAPACITY);
        let mask = 1usize << idx;
        self.bits.fetch_or(mask, Ordering::SeqCst);
    }

    /// Clear the bit at `idx`.
    #[inline]
    pub fn clear(&self, idx: usize) {
        debug_assert!(idx < Self::CAPACITY);
        let mask = !(1usize << idx);
        self.bits.fetch_and(mask, Ordering::SeqCst);
    }

    /// Test if the bit at `idx` is set.
    #[inline]
    pub fn is_set(&self, idx: usize) -> bool {
        debug_assert!(idx < Self::CAPACITY);
        let mask = 1usize << idx;
        (self.bits.load(Ordering::SeqCst) & mask) != 0
    }

    /// Find the first free (zero) bit, returning `Some(idx)` or `None` if full.
    #[inline]
    pub fn find_free(&self) -> Option<usize> {
        let bits = self.bits.load(Ordering::SeqCst);
        for idx in 0..Self::CAPACITY {
            let mask = 1usize << idx;
            if (bits & mask) == 0 {
                return Some(idx);
            }
        }
        None
    }
}
