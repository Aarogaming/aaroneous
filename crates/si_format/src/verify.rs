//! crates/si_format/src/verify.rs
//! Centralized validation logic for `.si` container format.
//!
//! Ensures magic bytes, tier flags, and alignment invariants are enforced
//! consistently across packers, loaders, and pipelines.

use anyhow::{bail, Result};
use crate::utils::ALIGNMENT_BYTES;

/// Magic bytes for `.si` SINT containers (distinct from legacy v2 JSON containers)
pub const SINT_PACKER_MAGIC: [u8; 4] = *b"SINT";

/// Minimum supported version for this loader/packer suite.
/// Must be >= 3 to enforce tensor-descriptor manifest with explicit byte offsets.
pub const MIN_VERSION: u32 = 3;

/// Validates the magic bytes at the start of a `.si` container.
///
/// # Errors
/// Returns `Err` if the magic bytes are missing or incorrect.
pub fn validate_magic_bytes(mmap: &[u8]) -> Result<()> {
    if mmap.len() < 4 || mmap[0..4] != SINT_PACKER_MAGIC {
        bail!("Missing or invalid SINT magic bytes");
    }
    Ok(())
}

/// Validates the version field (offset 0x04) in a `.si` container.
///
/// # Errors
/// Returns `Err` if the version is below `MIN_VERSION`.
pub fn validate_version(version: u32) -> Result<()> {
    if version < MIN_VERSION {
        bail!(
            "Container version v{} is not supported (requires v{}+)",
            version,
            MIN_VERSION
        );
    }
    Ok(())
}

/// Validates that a tensor descriptor's byte offset and length are properly aligned.
///
/// # Errors
/// Returns `Err` if the offset is not 64-byte aligned or the length is not a multiple of 4.
pub fn validate_tensor_descriptor(offset: u64, length: u64) -> Result<()> {
    if !(offset as usize).is_multiple_of(ALIGNMENT_BYTES) {
        bail!(
            "Tensor byte_offset {} is not 64-byte aligned",
            offset
        );
    }
    if !(length as usize).is_multiple_of(4) {
        bail!(
            "Tensor byte_length {} is not a multiple of 4 (f32)",
            length
        );
    }
    Ok(())
}

/// Validates that a given offset and payload size fit within the mapped region.
///
/// # Errors
/// Returns `Err` if the range exceeds the available memory-mapped bytes.
pub fn validate_range(mmap_len: usize, start: usize, end: usize) -> Result<()> {
    if end > mmap_len {
        bail!(
            "Tensor slice exceeds mapped region (start={}, end={}, mmap_len={})",
            start,
            end,
            mmap_len
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_magic_bytes_ok() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&SINT_PACKER_MAGIC);
        assert!(validate_magic_bytes(&data).is_ok());
    }

    #[test]
    fn test_validate_magic_bytes_bad() {
        let data = vec![0u8; 20];
        assert!(validate_magic_bytes(&data).is_err());
    }

    #[test]
    fn test_validate_version_ok() {
        assert!(validate_version(MIN_VERSION).is_ok());
        assert!(validate_version(4).is_ok());
    }

    #[test]
    fn test_validate_version_bad() {
        assert!(validate_version(MIN_VERSION - 1).is_err());
    }

    #[test]
    fn test_validate_tensor_descriptor_aligned() {
        let offset = ALIGNMENT_BYTES as u64;
        let length = (ALIGNMENT_BYTES * 2) as u64;
        assert!(validate_tensor_descriptor(offset, length).is_ok());
    }

    #[test]
    fn test_validate_tensor_descriptor_unaligned() {
        let offset = 1u64;
        let length = ALIGNMENT_BYTES as u64;
        assert!(validate_tensor_descriptor(offset, length).is_err());
    }

    #[test]
    fn test_validate_range_ok() {
        let mmap_len = 256;
        let start = 0;
        let end = 128;
        assert!(validate_range(mmap_len, start, end).is_ok());
    }

    #[test]
    fn test_validate_range_oob() {
        let mmap_len = 64;
        let start = 0;
        let end = 128;
        assert!(validate_range(mmap_len, start, end).is_err());
    }
}
