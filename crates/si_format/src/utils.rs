//! crates/si_format/src/utils.rs
//! Shared alignment utilities for `.si` container format.
//!
//! Provides deterministic padding and alignment routines to ensure
//! zero-copy memory mapping safety across platforms (AVX-512, SIMD).

/// Enforced SIMD / cache-line alignment for all tensor payloads in `.si` containers.
pub const ALIGNMENT_BYTES: usize = 64;

/// Pads a byte slice to the next `ALIGNMENT_BYTES` boundary.
/// Returns a new vector containing the original data followed by zero-padding.
///
/// # Examples
/// ```
/// use si_format::utils::{align_to_64, ALIGNMENT_BYTES};
/// let data = vec![1u8, 2, 3];
/// let padded = align_to_64(&data);
/// assert!(padded.len() % ALIGNMENT_BYTES == 0);
/// ```
pub fn align_to_64(bytes: &[u8]) -> Vec<u8> {
    let len = bytes.len();
    let rem = len % ALIGNMENT_BYTES;
    if rem == 0 {
        bytes.to_vec()
    } else {
        let pad_len = ALIGNMENT_BYTES - rem;
        let mut padded = Vec::with_capacity(len + pad_len);
        padded.extend_from_slice(bytes);
        padded.resize(len + pad_len, 0u8);
        padded
    }
}

/// Computes the number of zero-padding bytes needed to reach the next
/// `ALIGNMENT_BYTES` boundary from `current_offset`.
///
/// # Examples
/// ```
/// use si_format::utils::compute_padding;
/// assert_eq!(compute_padding(0), 0);
/// assert_eq!(compute_padding(63), 1);
/// assert_eq!(compute_padding(64), 0);
/// ```
pub fn compute_padding(offset: u64) -> usize {
    let rem = (offset as usize) % ALIGNMENT_BYTES;
    if rem == 0 { 0 } else { ALIGNMENT_BYTES - rem }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_to_64_empty() {
        let padded = align_to_64(&[]);
        assert_eq!(padded.len(), 0);
    }

    #[test]
    fn test_align_to_64_already_aligned() {
        let data = vec![0u8; ALIGNMENT_BYTES];
        let padded = align_to_64(&data);
        assert_eq!(padded.len(), ALIGNMENT_BYTES);
    }

    #[test]
    fn test_align_to_64_small() {
        let data = vec![1u8, 2, 3];
        let padded = align_to_64(&data);
        assert_eq!(padded.len(), ALIGNMENT_BYTES);
        assert_eq!(&padded[0..3], &[1, 2, 3]);
        assert_eq!(&padded[3..ALIGNMENT_BYTES], &vec![0u8; ALIGNMENT_BYTES - 3]);
    }

    #[test]
    fn test_compute_padding_edge_cases() {
        for offset in 0u64..=256 {
            let pad = compute_padding(offset);
            assert!((offset as usize + pad).is_multiple_of(ALIGNMENT_BYTES),
                "offset={} pad={} not aligned", offset, pad);
        }
    }
}
