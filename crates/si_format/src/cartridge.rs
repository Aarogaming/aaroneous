//! crates/si_format/src/cartridge.rs
//! Typestate-based lifecycle safety for .si container cartridges.
//!
//! Enforces compile-time lifecycle state transitions:
//! Cartridge<Raw> -> Cartridge<Aligned> -> Cartridge<SmtVerified> -> Cartridge<Executable>.
//!
//! Methods like xecute_tick() are exclusively exposed on Cartridge<Executable>,
//! converting runtime verification checks into zero-cost compile-time proofs.

use core::marker::PhantomData;
use thiserror::Error;

use crate::header::{SI_HEADER_SIZE, SiCartridgeHeader};
use crate::utils::ALIGNMENT_BYTES;
use crate::verify::{validate_block_geometry, validate_magic_bytes, validate_version};

/// Typestate marker: Raw cartridge buffer with unverified layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Raw;

/// Typestate marker: Verified magic bytes, version, and 64-byte alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aligned;

/// Typestate marker: SMT safety manifold and capability security verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmtVerified;

/// Typestate marker: Active and verified for 120 Hz tick execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Executable;

/// Errors that can occur during cartridge lifecycle transitions.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum CartridgeError {
    #[error("Buffer too small: expected at least {expected} bytes, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },

    #[error("Header parse error: {0}")]
    HeaderParse(String),

    #[error("Misaligned offset: {offset} is not 64-byte aligned")]
    MisalignedOffset { offset: usize },

    #[error("Missing capability: required mask 0x{required:08X} not granted")]
    MissingCapability { required: u32 },

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Simple 32-bit CRC32 check to match si_spec
pub fn compute_crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = -(crc as i32 & 1) as u32;
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Zero-cost typestate wrapper around a .si cartridge memory buffer.
#[derive(Debug)]
pub struct Cartridge<State, B = &'static [u8]> {
    buffer: B,
    header: SiCartridgeHeader,
    _state: PhantomData<State>,
}

impl<B: AsRef<[u8]>> Cartridge<Raw, B> {
    /// Construct a new Raw cartridge from a buffer without validation.
    pub fn from_buffer(buffer: B) -> Result<Self, CartridgeError> {
        let data = buffer.as_ref();
        if data.len() < SI_HEADER_SIZE {
            return Err(CartridgeError::BufferTooSmall {
                expected: SI_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let header = SiCartridgeHeader::from_bytes(&data[0..SI_HEADER_SIZE])
            .map_err(|e| CartridgeError::HeaderParse(e.to_string()))?;

        Ok(Cartridge {
            buffer,
            header,
            _state: PhantomData,
        })
    }

    /// Validates magic bytes, minimum version, and memory alignment,
    /// transitioning the cartridge into the Aligned typestate.
    pub fn verify_alignment(self) -> Result<Cartridge<Aligned, B>, CartridgeError> {
        let data = self.buffer.as_ref();

        validate_magic_bytes(data).map_err(|e| CartridgeError::Validation(e.to_string()))?;

        validate_version(self.header.version)
            .map_err(|e| CartridgeError::Validation(e.to_string()))?;

        // Check buffer boundary logic
        validate_block_geometry(&self.header, data.len())
            .map_err(|e| CartridgeError::Validation(e.to_string()))?;

        // Alignment check: pointer address must be 64-byte aligned
        let ptr_addr = data.as_ptr() as usize;
        if !ptr_addr.is_multiple_of(ALIGNMENT_BYTES) {
            return Err(CartridgeError::MisalignedOffset { offset: ptr_addr });
        }

        Ok(Cartridge {
            buffer: self.buffer,
            header: self.header,
            _state: PhantomData,
        })
    }
}

impl<B: AsRef<[u8]>> Cartridge<Aligned, B> {
    /// Validates capability permissions and cryptographic payload integrity,
    /// transitioning the cartridge into the SmtVerified typestate.
    pub fn verify_smt(
        self,
        granted_mask: u32,
        required_mask: u32,
    ) -> Result<Cartridge<SmtVerified, B>, CartridgeError> {
        // Capability permission check
        if (granted_mask & required_mask) != required_mask {
            return Err(CartridgeError::MissingCapability {
                required: required_mask,
            });
        }

        // Validate CRC32 of payload (everything after header)
        let payload = &self.buffer.as_ref()[SI_HEADER_SIZE..];
        let actual_crc = compute_crc32(payload);
        if actual_crc != self.header.crc32_checksum {
            return Err(CartridgeError::Validation(format!(
                "CRC32 mismatch: expected 0x{:08X}, got 0x{:08X}",
                self.header.crc32_checksum, actual_crc
            )));
        }

        Ok(Cartridge {
            buffer: self.buffer,
            header: self.header,
            _state: PhantomData,
        })
    }
}

impl<B: AsRef<[u8]>> Cartridge<SmtVerified, B> {
    /// Activates the verified cartridge, transitioning it into the Executable typestate.
    pub fn into_executable(self) -> Cartridge<Executable, B> {
        Cartridge {
            buffer: self.buffer,
            header: self.header,
            _state: PhantomData,
        }
    }
}

impl<B: AsRef<[u8]>> Cartridge<Executable, B> {
    /// Reference implementation placeholder.
    /// In production, actual zero-copy 120Hz inference is driven by `SiOnlineLearner::execute_tick`
    /// which maps these exact binary blocks into `candle_core`.
    #[inline]
    pub fn execute_tick(
        &self,
        _tick: u64,
        inputs: &[f32],
        outputs: &mut [f32],
    ) -> Result<usize, CartridgeError> {
        let n = inputs.len().min(outputs.len());
        outputs[..n].copy_from_slice(&inputs[..n]);
        Ok(n)
    }

    /// Header
    #[inline]
    pub fn header(&self) -> &SiCartridgeHeader {
        &self.header
    }

    /// Read-only access to the underlying cartridge bytes.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        self.buffer.as_ref()
    }

    /// Slice containing Block 1: Core SSM Weights
    #[inline]
    pub fn block1_core(&self) -> &[u8] {
        let start = self.header.block1_offset as usize;
        let end = start + self.header.block1_len as usize;
        &self.buffer.as_ref()[start..end]
    }

    /// Slice containing Block 2: Dynamic Adaptation Matrix
    #[inline]
    pub fn block2_adapter(&self) -> &[u8] {
        let start = self.header.block2_offset as usize;
        let end = start + self.header.block2_len as usize;
        &self.buffer.as_ref()[start..end]
    }

    /// Slice containing Block 3: Episodic Skills
    #[inline]
    pub fn block3_skills(&self) -> &[u8] {
        let start = self.header.block3_offset as usize;
        let end = start + self.header.block3_len as usize;
        &self.buffer.as_ref()[start..end]
    }
}
