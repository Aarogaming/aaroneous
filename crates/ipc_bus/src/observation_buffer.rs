//! crates/ipc_bus/src/observation_buffer.rs
//! Lock-Free Single-Writer Multiple-Reader (SWMR) Observation Ring Buffer.
//! Captures high-frequency (120Hz) machine execution telemetry (state snapshot, opcodes, features)
//! for passive observation training, dual-rail shadow verification, and continuous RL.

use core_contracts::ObservationFramePod;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Magic bytes for the observation ring buffer: b"SIOBSBUF"
pub const OBSERVATION_MAGIC: [u8; 8] = *b"SIOBSBUF";

/// Binary schema version
pub const OBSERVATION_VERSION: u32 = 1;

/// Fixed page-aligned header region (4096 bytes)
pub const OBSERVATION_HEADER_SIZE: usize = 4096;

/// Frame payload size in bytes (matches ObservationFramePod exactly)
pub const OBSERVATION_FRAME_SIZE: usize = std::mem::size_of::<ObservationFramePod>();

/// Size of each slot in the ring buffer:
/// 8 bytes (seq start) + 1088 bytes (frame) + 8 bytes (seq end) + 48 bytes (pad) = 1152 bytes (64-byte aligned)
pub const OBSERVATION_SLOT_SIZE: usize = 1152;

/// Default capacity: 16,384 slots (~18.8 MB), holding over 2 minutes of continuous 120Hz history
pub const DEFAULT_OBSERVATION_CAPACITY: usize = 16_384;

const _: () = assert!(OBSERVATION_FRAME_SIZE == 1088);
const _: () = assert!(OBSERVATION_SLOT_SIZE.is_multiple_of(64));
const _: () = assert!(8 + OBSERVATION_FRAME_SIZE + 8 <= OBSERVATION_SLOT_SIZE);

/// Zero-copy 64-byte file header stored in the page-aligned header region
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObservationBufferHeaderPod {
    pub magic: [u8; 8],
    pub version: u32,
    pub slot_size: u32,
    pub capacity: u32,
    pub _pad0: u32,
    pub write_sequence: u64,
    pub wrap_count: u64,
    pub _reserved: [u8; 24],
}

impl Default for ObservationBufferHeaderPod {
    fn default() -> Self {
        Self {
            magic: OBSERVATION_MAGIC,
            version: OBSERVATION_VERSION,
            slot_size: OBSERVATION_SLOT_SIZE as u32,
            capacity: DEFAULT_OBSERVATION_CAPACITY as u32,
            _pad0: 0,
            write_sequence: 0,
            wrap_count: 0,
            _reserved: [0u8; 24],
        }
    }
}

/// Structured error conditions for observation buffer operations
#[derive(Debug, thiserror::Error)]
pub enum ObservationBufferError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Corrupted observation log header magic: expected {expected:?}, got {found:?}")]
    CorruptedMagic { expected: [u8; 8], found: [u8; 8] },

    #[error("Unsupported observation log version: expected {expected}, got {found}")]
    UnsupportedVersion { expected: u32, found: u32 },

    #[error("Invalid log file size: expected at least {expected} bytes, found {found} bytes")]
    InvalidFileSize { expected: usize, found: u64 },

    #[error("Buffer capacity must be greater than zero")]
    ZeroCapacity,

    #[error("POD deserialization error: {0}")]
    PodError(String),
}

enum Storage {
    Mmap {
        mmap: memmap2::MmapMut,
        path: PathBuf,
    },
    Memory {
        buffer: Vec<u8>,
    },
}

impl Storage {
    fn as_ptr(&self) -> *const u8 {
        match self {
            Self::Mmap { mmap, .. } => mmap.as_ptr(),
            Self::Memory { buffer } => buffer.as_ptr(),
        }
    }

    fn as_mut_ptr(&mut self) -> *mut u8 {
        match self {
            Self::Mmap { mmap, .. } => mmap.as_mut_ptr(),
            Self::Memory { buffer } => buffer.as_mut_ptr(),
        }
    }

    fn flush(&self) -> Result<(), std::io::Error> {
        match self {
            Self::Mmap { mmap, .. } => mmap.flush(),
            Self::Memory { .. } => Ok(()),
        }
    }
}

/// Lock-free Single-Writer Multiple-Reader (SWMR) circular observation buffer.
pub struct ObservationBuffer {
    storage: Storage,
    capacity: usize,
    sequence: u64,
}

// SAFETY: Storage is accessed via atomic seqlock sequencing and synchronized slot boundaries.
unsafe impl Send for ObservationBuffer {}
unsafe impl Sync for ObservationBuffer {}

impl ObservationBuffer {
    /// Opens an existing observation buffer or creates a new one at `path`.
    pub fn open_or_create(
        path: impl AsRef<Path>,
        capacity: usize,
    ) -> Result<Self, ObservationBufferError> {
        if capacity == 0 {
            return Err(ObservationBufferError::ZeroCapacity);
        }

        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let total_size = OBSERVATION_HEADER_SIZE + (capacity * OBSERVATION_SLOT_SIZE);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let current_len = file.metadata()?.len();
        if current_len < total_size as u64 {
            file.set_len(total_size as u64)?;
        }

        // SAFETY: File was sized to total_size and opened read/write.
        let mut mmap = unsafe { memmap2::MmapOptions::new().map_mut(&file)? };

        let header_slice = &mmap[..std::mem::size_of::<ObservationBufferHeaderPod>()];
        let existing_header =
            bytemuck::try_from_bytes::<ObservationBufferHeaderPod>(header_slice).ok();

        let sequence = match existing_header {
            Some(h)
                if h.magic == OBSERVATION_MAGIC
                    && h.version == OBSERVATION_VERSION
                    && h.slot_size == OBSERVATION_SLOT_SIZE as u32
                    && h.capacity == capacity as u32 =>
            {
                h.write_sequence
            }
            _ => {
                let header = ObservationBufferHeaderPod {
                    magic: OBSERVATION_MAGIC,
                    version: OBSERVATION_VERSION,
                    slot_size: OBSERVATION_SLOT_SIZE as u32,
                    capacity: capacity as u32,
                    _pad0: 0,
                    write_sequence: 0,
                    wrap_count: 0,
                    _reserved: [0u8; 24],
                };
                let header_bytes = bytemuck::bytes_of(&header);
                mmap[..header_bytes.len()].copy_from_slice(header_bytes);
                mmap[header_bytes.len()..OBSERVATION_HEADER_SIZE].fill(0);
                mmap.flush()?;
                0
            }
        };

        Ok(Self {
            storage: Storage::Mmap {
                mmap,
                path: path.to_path_buf(),
            },
            capacity,
            sequence,
        })
    }

    /// Allocates an in-memory observation buffer without disk persistence (for testing/sandboxing).
    pub fn new_in_memory(capacity: usize) -> Result<Self, ObservationBufferError> {
        if capacity == 0 {
            return Err(ObservationBufferError::ZeroCapacity);
        }

        let total_size = OBSERVATION_HEADER_SIZE + (capacity * OBSERVATION_SLOT_SIZE);
        let mut buffer = vec![0u8; total_size];

        let header = ObservationBufferHeaderPod {
            magic: OBSERVATION_MAGIC,
            version: OBSERVATION_VERSION,
            slot_size: OBSERVATION_SLOT_SIZE as u32,
            capacity: capacity as u32,
            _pad0: 0,
            write_sequence: 0,
            wrap_count: 0,
            _reserved: [0u8; 24],
        };
        let header_bytes = bytemuck::bytes_of(&header);
        buffer[..header_bytes.len()].copy_from_slice(header_bytes);

        Ok(Self {
            storage: Storage::Memory { buffer },
            capacity,
            sequence: 0,
        })
    }

    /// Total slot capacity in this ring buffer.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Returns the filesystem path if the buffer is file-backed.
    pub fn path(&self) -> Option<&Path> {
        match &self.storage {
            Storage::Mmap { path, .. } => Some(path.as_path()),
            Storage::Memory { .. } => None,
        }
    }

    /// Returns the current monotonic write sequence number.
    #[inline]
    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }

    /// Records an observation frame into the next circular slot.
    ///
    /// Monotonically increments sequence, assigns it to `frame.sequence`, writes into the slot
    /// protected by a seqlock pattern, and updates the header sequence atomically.
    /// Strictly non-allocating.
    pub fn record(
        &mut self,
        mut frame: ObservationFramePod,
    ) -> Result<u64, ObservationBufferError> {
        self.sequence = self.sequence.saturating_add(1);
        let seq = self.sequence;
        frame.sequence = seq;

        let slot_idx = ((seq - 1) as usize) % self.capacity;
        let slot_offset = OBSERVATION_HEADER_SIZE + (slot_idx * OBSERVATION_SLOT_SIZE);

        let base_ptr = self.storage.as_mut_ptr();

        // Seqlock protocol:
        // 1. Mark start with odd sequence (in-progress marker = seq * 2 - 1)
        let seq_start_val = seq.wrapping_mul(2).wrapping_sub(1);
        let seq_end_val = seq.wrapping_mul(2);

        // SAFETY: Pointer is strictly within bounds of allocated storage;
        // writes use standard byte copy followed by atomic release sequence updates.
        unsafe {
            let start_ptr = base_ptr.add(slot_offset) as *mut AtomicU64;
            let payload_ptr = base_ptr.add(slot_offset + 8);
            let end_ptr = base_ptr.add(slot_offset + 8 + OBSERVATION_FRAME_SIZE) as *mut AtomicU64;

            // Signal write started
            (*start_ptr).store(seq_start_val, Ordering::Release);

            // Copy frame payload directly (zero heap allocation)
            let frame_bytes = bytemuck::bytes_of(&frame);
            std::ptr::copy_nonoverlapping(
                frame_bytes.as_ptr(),
                payload_ptr,
                OBSERVATION_FRAME_SIZE,
            );

            // Signal write completed with matching even sequence
            (*end_ptr).store(seq_end_val, Ordering::Release);
            (*start_ptr).store(seq_end_val, Ordering::Release);

            // Update header write_sequence atomically
            let header_seq_ptr = base_ptr.add(24) as *mut AtomicU64;
            (*header_seq_ptr).store(seq, Ordering::Release);
        }

        Ok(seq)
    }

    /// Reads the most recently committed observation frame, if any.
    pub fn read_latest(&self) -> Option<ObservationFramePod> {
        if self.sequence == 0 {
            return None;
        }
        self.read_by_sequence(self.sequence)
    }

    /// Reads the observation frame corresponding to `seq` if it has not been overwritten.
    pub fn read_by_sequence(&self, seq: u64) -> Option<ObservationFramePod> {
        if seq == 0 || seq > self.sequence {
            return None;
        }

        // Check if slot has already been overwritten by circular wrap
        if self.sequence > self.capacity as u64 && seq <= self.sequence - (self.capacity as u64) {
            return None;
        }

        let slot_idx = ((seq - 1) as usize) % self.capacity;
        let slot_offset = OBSERVATION_HEADER_SIZE + (slot_idx * OBSERVATION_SLOT_SIZE);
        let base_ptr = self.storage.as_ptr();

        let expected_val = seq.wrapping_mul(2);

        // Seqlock read loop with bounded retry count
        for _ in 0..10 {
            unsafe {
                let start_ptr = base_ptr.add(slot_offset) as *const AtomicU64;
                let payload_ptr = base_ptr.add(slot_offset + 8);
                let end_ptr =
                    base_ptr.add(slot_offset + 8 + OBSERVATION_FRAME_SIZE) as *const AtomicU64;

                let s1 = (*start_ptr).load(Ordering::Acquire);
                if !s1.is_multiple_of(2) || s1 != expected_val {
                    std::hint::spin_loop();
                    continue;
                }

                let mut out_frame = ObservationFramePod::default();
                let out_bytes = bytemuck::bytes_of_mut(&mut out_frame);
                std::ptr::copy_nonoverlapping(
                    payload_ptr,
                    out_bytes.as_mut_ptr(),
                    OBSERVATION_FRAME_SIZE,
                );

                let s2 = (*end_ptr).load(Ordering::Acquire);
                if s1 == s2 {
                    return Some(out_frame);
                }
            }
            std::hint::spin_loop();
        }

        None
    }

    /// Reads a batch of consecutive frames into `out_slice` starting at `start_seq`.
    /// Returns the number of frames successfully populated.
    pub fn read_batch(&self, start_seq: u64, out_slice: &mut [ObservationFramePod]) -> usize {
        let mut count = 0;
        for (i, slot) in out_slice.iter_mut().enumerate() {
            let target_seq = start_seq.saturating_add(i as u64);
            match self.read_by_sequence(target_seq) {
                Some(frame) => {
                    *slot = frame;
                    count += 1;
                }
                None => break,
            }
        }
        count
    }

    /// Flushes unwritten memory pages to disk if file-backed.
    pub fn flush(&self) -> Result<(), ObservationBufferError> {
        self.storage.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_observation_buffer_roundtrip() {
        let mut buf = ObservationBuffer::new_in_memory(16).unwrap();
        assert_eq!(buf.capacity(), 16);
        assert_eq!(buf.current_sequence(), 0);
        assert!(buf.read_latest().is_none());

        let frame = ObservationFramePod {
            sequence: 0,
            timestamp_ns: 1000,
            tick_duration_us: 150,
            actual_opcode: 0x0100,
            predicted_opcode: 0x0100,
            actor_tier: 3,
            flags: 1,
            concurrence: 1,
            _pad0: 0,
            bus_integrity: 99.5,
            bus_understanding: 98.0,
            flow_score: 0.95,
            thermal_factor: 1.0,
            reward: 1.0,
            confidence: 0.99,
            free_energy: 0.01,
            _reserved: [0; 2],
            state_features: [0.25; 256],
        };

        let seq1 = buf.record(frame).unwrap();
        assert_eq!(seq1, 1);
        assert_eq!(buf.current_sequence(), 1);

        let read1 = buf.read_latest().unwrap();
        assert_eq!(read1.sequence, 1);
        assert_eq!(read1.actual_opcode, 0x0100);
        assert_eq!(read1.state_features[0], 0.25);

        for i in 2..=20 {
            let mut f = frame;
            f.actual_opcode = (0x0100 + i) as u16;
            buf.record(f).unwrap();
        }

        assert_eq!(buf.current_sequence(), 20);
        let latest = buf.read_latest().unwrap();
        assert_eq!(latest.sequence, 20);
        assert_eq!(latest.actual_opcode, (0x0100 + 20) as u16);

        assert!(buf.read_by_sequence(1).is_none());
        assert!(buf.read_by_sequence(4).is_none());

        let f5 = buf.read_by_sequence(5).unwrap();
        assert_eq!(f5.sequence, 5);

        let mut batch = [ObservationFramePod::default(); 5];
        let n = buf.read_batch(16, &mut batch);
        assert_eq!(n, 5);
        assert_eq!(batch[0].sequence, 16);
        assert_eq!(batch[4].sequence, 20);
    }

    #[test]
    fn test_file_backed_observation_buffer() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("obs_test.shm");

        {
            let mut buf = ObservationBuffer::open_or_create(&file_path, 8).unwrap();
            let mut f = ObservationFramePod::default();
            f.actual_opcode = 0x0400;
            buf.record(f).unwrap();
            buf.flush().unwrap();
        }

        {
            let buf = ObservationBuffer::open_or_create(&file_path, 8).unwrap();
            assert_eq!(buf.current_sequence(), 1);
            let latest = buf.read_latest().unwrap();
            assert_eq!(latest.sequence, 1);
            assert_eq!(latest.actual_opcode, 0x0400);
        }
    }
}
