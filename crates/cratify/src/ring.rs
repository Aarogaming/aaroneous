//! Zero-Copy Inter-ACC Ring Buffer (Lock-Free SPSC & MPSC).
//!
//! Implements the Disruptor-style shared-memory communication layer
//! specified in `dev/docs/inter_acc_ring_buffer_spec.md`.  Payload data
//! moving through these buffers is `bytemuck::Pod` and accessed via
//! raw byte slices — no copying, no locking, no GC.
//!
//! # Memory Layout
//! The ring buffer metadata (cursors, capacity) lives in a normal
//! struct.  The slot data region is a flat `Vec<u8>` (or mmap'd
//! region in production) partitioned into fixed-size slots, each
//! containing an 8-byte `SlotHeader` followed by the payload.
//!
//! # Safety Model
//! - Capacity is always a power of two; masking replaces modulo.
//! - Acquire/Release ordering on every cursor advance prevents reordering.
//! - 64-byte cache-line padding on all hot atomics eliminates false sharing.
//! - Producers spin-wait with exponential back-off when the buffer is full.

use std::fmt;
use std::sync::atomic::{fence, AtomicUsize, Ordering};
use std::thread;

use bytemuck::{Pod, Zeroable};

// ── Constants ────────────────────────────────────────────────────────

/// Cache line size in bytes (x86-64 / ARM64).
const CACHE_LINE: usize = 64;

/// Header size per slot: 4-byte sequence + 4-byte flags = 8 bytes.
const SLOT_HEADER_SIZE: usize = 8;

// ── Error Types ──────────────────────────────────────────────────────

/// Errors arising from ring buffer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RingError {
    /// The buffer is full; the producer must wait.
    Full,
    /// The buffer is empty; the consumer must wait.
    Empty,
    /// Payload size exceeds the configured slot capacity.
    PayloadTooLarge { payload: usize, slot: usize },
    /// Buffer capacity must be a power of two.
    InvalidCapacity(usize),
}

impl fmt::Display for RingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "ring buffer full"),
            Self::Empty => write!(f, "ring buffer empty"),
            Self::PayloadTooLarge { payload, slot } => {
                write!(f, "payload {} bytes exceeds slot capacity {}", payload, slot)
            }
            Self::InvalidCapacity(n) => {
                write!(f, "capacity {} is not a power of two", n)
            }
        }
    }
}

impl std::error::Error for RingError {}

pub type RingResult<T> = std::result::Result<T, RingError>;

// ── Error Flags (§6) ─────────────────────────────────────────────────

/// Bitflags for packet error tracking per spec §6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ErrorFlags(pub u32);

impl std::ops::BitOr for ErrorFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        ErrorFlags(self.0 | rhs.0)
    }
}

impl ErrorFlags {
    pub const NONE: ErrorFlags = ErrorFlags(0);
    /// Payload size exceeds slot — packet dropped.
    pub const PAYLOAD_EXCEEDED: ErrorFlags = ErrorFlags(0x1);
    /// CRC mismatch (optional integrity check).
    pub const CRC_MISMATCH: ErrorFlags = ErrorFlags(0x2);
    /// Consumer timeout — slot recycled without processing.
    pub const CONSUMER_TIMEOUT: ErrorFlags = ErrorFlags(0x4);

    pub fn contains(self, flag: ErrorFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    pub fn bits(self) -> u32 {
        self.0
    }
}

// ── Cache-Line Padded Atomic (§5) ───────────────────────────────────

/// 64-bit atomic aligned to a cache line to prevent false sharing.
///
/// Layout: `[padding..64][usize atomic][padding..64]` ensuring each
/// instance lives on its own cache line.
#[repr(C)]
pub struct AtomicPaddedUsize {
    _padding_before: [u8; CACHE_LINE],
    val: AtomicUsize,
    _padding_after: [u8; CACHE_LINE],
}

impl AtomicPaddedUsize {
    pub fn new(value: usize) -> Self {
        Self {
            _padding_before: [0u8; CACHE_LINE],
            val: AtomicUsize::new(value),
            _padding_after: [0u8; CACHE_LINE],
        }
    }

    pub fn load(&self, order: Ordering) -> usize {
        self.val.load(order)
    }

    pub fn store(&self, val: usize, order: Ordering) {
        self.val.store(val, order);
    }

    pub fn compare_exchange(
        &self,
        current: usize,
        new: usize,
        success: Ordering,
        failure: Ordering,
    ) -> Result<usize, usize> {
        self.val.compare_exchange(current, new, success, failure)
    }

    pub fn fetch_add(&self, val: usize, order: Ordering) -> usize {
        self.val.fetch_add(val, order)
    }
}

// ── Slot Header ──────────────────────────────────────────────────────

/// Per-slot metadata written by the producer before the payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SlotHeader {
    /// Sequence number — consumer checks this to know if the slot is ready.
    pub sequence: u32,
    /// Error flags (bitfield).
    pub flags: u32,
}

// SAFETY: SlotHeader is a plain 8-byte Pod — no padding, no pointers.
unsafe impl Zeroable for SlotHeader {}
unsafe impl Pod for SlotHeader {}

// ── T-Shirt Sized Pools (§3) ────────────────────────────────────────

/// Predefined slot capacities for the T-shirt memory pool model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotSize {
    /// 256 bytes — control messages, small scalars.
    XS,
    /// 1 KB — structured telemetry, small packets.
    S,
    /// 4 KB — standard payloads (default per spec).
    M,
    /// 16 KB — large data frames, vision chunks.
    L,
    /// 64 KB — bulk transfers, model weights.
    XL,
}

impl SlotSize {
    /// Returns the capacity in bytes for this tier.
    pub fn capacity(self) -> usize {
        match self {
            Self::XS => 256,
            Self::S => 1024,
            Self::M => 4096,
            Self::L => 16_384,
            Self::XL => 65_536,
        }
    }
}

impl fmt::Display for SlotSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::XS => write!(f, "XS (256B)"),
            Self::S => write!(f, "S (1KB)"),
            Self::M => write!(f, "M (4KB)"),
            Self::L => write!(f, "L (16KB)"),
            Self::XL => write!(f, "XL (64KB)"),
        }
    }
}

// ── SPSC Ring Buffer ────────────────────────────────────────────────

/// Lock-free Single-Producer Single-Consumer ring buffer.
///
/// # Thread Safety
/// - Exactly one thread may hold `&SpscProducer` (the writer).
/// - Exactly one thread may hold `&SpscConsumer` (the reader).
/// - The shared `SpscRing` may be read by both (atomic cursors).
pub struct SpscRing {
    /// Producer write cursor — cache-line padded.
    pub write_cursor: AtomicPaddedUsize,
    /// Consumer read cursor — cache-line padded.
    pub read_cursor: AtomicPaddedUsize,
    /// Total number of slots (capacity). Always a power of two.
    pub capacity: usize,
    /// Size of each slot in bytes (header + payload, aligned to 8).
    pub slot_size: usize,
}

impl SpscRing {
    /// Create a new SPSC ring buffer header.
    ///
    /// # Arguments
    /// * `capacity` — Number of slots. Must be a power of two.
    /// * `slot_size` — Size of each slot in bytes (header + payload).
    pub fn new(capacity: usize, slot_size: usize) -> RingResult<Self> {
        if capacity == 0 || !capacity.is_power_of_two() {
            return Err(RingError::InvalidCapacity(capacity));
        }
        if slot_size < SLOT_HEADER_SIZE {
            return Err(RingError::PayloadTooLarge { payload: 0, slot: slot_size });
        }

        Ok(Self {
            write_cursor: AtomicPaddedUsize::new(0),
            read_cursor: AtomicPaddedUsize::new(0),
            capacity,
            slot_size,
        })
    }

    /// Total byte size of the data region (excluding this header).
    pub fn data_bytes(&self) -> usize {
        self.capacity * self.slot_size
    }

    /// Number of slots available for writing.
    pub fn available_slots(&self, write: usize, read: usize) -> usize {
        self.capacity - (write - read)
    }

    /// Byte offset of slot `index` in the data region.
    pub fn slot_offset(&self, index: usize) -> usize {
        (index & (self.capacity - 1)) * self.slot_size
    }
}

/// Write handle for the SPSC ring buffer. Owned by the producer thread.
pub struct SpscProducer<'a> {
    ring: &'a SpscRing,
    data: &'a mut [u8],
}

impl<'a> SpscProducer<'a> {
    /// Wrap the ring and data region into a typed producer.
    pub fn new(ring: &'a SpscRing, data: &'a mut [u8]) -> Self {
        Self { ring, data }
    }

    /// Write a raw payload (any Pod type) into the next available slot.
    ///
    /// # Arguments
    /// * `payload` — The data to write. Must fit in `slot_size - SLOT_HEADER_SIZE`.
    pub fn write_raw<T: Pod>(&mut self, payload: &T) -> RingResult<usize> {
        let payload_size = std::mem::size_of::<T>();
        if payload_size + SLOT_HEADER_SIZE > self.ring.slot_size {
            return Err(RingError::PayloadTooLarge {
                payload: payload_size,
                slot: self.ring.slot_size,
            });
        }
        self.write_bytes(bytemuck::bytes_of(payload))
    }

    /// Write raw bytes into the next available slot.
    ///
    /// Spins with exponential back-off if the buffer is full, waiting
    /// for the consumer to release slots.
    pub fn write_bytes(&mut self, payload: &[u8]) -> RingResult<usize> {
        if payload.len() + SLOT_HEADER_SIZE > self.ring.slot_size {
            return Err(RingError::PayloadTooLarge {
                payload: payload.len(),
                slot: self.ring.slot_size,
            });
        }

        // Backpressure: wait until a slot is available.
        self.wait_available(1);

        let slot_index = self.ring.write_cursor.fetch_add(1, Ordering::AcqRel);
        let offset = self.ring.slot_offset(slot_index);
        let slot = &mut self.data[offset..offset + self.ring.slot_size];

        // Zero-fill the slot first (prevents stale data in padding).
        slot.fill(0);

        // Write payload after the header region.
        slot[SLOT_HEADER_SIZE..SLOT_HEADER_SIZE + payload.len()]
            .copy_from_slice(payload);

        // Release the slot — set sequence = slot_index + 1.
        let seq = (slot_index + 1) as u32;
        let header = SlotHeader { sequence: seq, flags: 0 };
        slot[..SLOT_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));

        // Release fence ensures payload writes are visible before the
        // sequence number update is observed by the consumer.
        fence(Ordering::Release);

        Ok(slot_index)
    }

    /// Write a slot with custom error flags.
    pub fn write_with_flags(&mut self, payload: &[u8], flags: ErrorFlags) -> RingResult<usize> {
        if payload.len() + SLOT_HEADER_SIZE > self.ring.slot_size {
            return Err(RingError::PayloadTooLarge {
                payload: payload.len(),
                slot: self.ring.slot_size,
            });
        }

        let slot_index = self.ring.write_cursor.fetch_add(1, Ordering::AcqRel);
        let offset = self.ring.slot_offset(slot_index);
        let slot = &mut self.data[offset..offset + self.ring.slot_size];

        slot.fill(0);
        slot[SLOT_HEADER_SIZE..SLOT_HEADER_SIZE + payload.len()]
            .copy_from_slice(payload);

        let seq = (slot_index + 1) as u32;
        let header = SlotHeader { sequence: seq, flags: flags.bits() };
        slot[..SLOT_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        fence(Ordering::Release);

        Ok(slot_index)
    }

    /// Number of slots available for writing right now.
    pub fn available(&self) -> usize {
        let w = self.ring.write_cursor.load(Ordering::Relaxed);
        let r = self.ring.read_cursor.load(Ordering::Acquire);
        self.ring.available_slots(w, r)
    }

    /// Spin-wait until at least `n` slots are available.
    pub fn wait_available(&self, n: usize) {
        let mut backoff = 1u64;
        loop {
            if self.available() >= n {
                return;
            }
            if backoff <= 32 {
                for _ in 0..backoff {
                    std::hint::spin_loop();
                }
                backoff = backoff.saturating_mul(2);
            } else {
                thread::yield_now();
                backoff = 1;
            }
        }
    }
}

/// Read handle for the SPSC ring buffer. Owned by the consumer thread.
pub struct SpscConsumer<'a> {
    ring: &'a SpscRing,
    data: &'a [u8],
}

impl<'a> SpscConsumer<'a> {
    /// Wrap the ring and data region into a typed consumer.
    pub fn new(ring: &'a SpscRing, data: &'a [u8]) -> Self {
        Self { ring, data }
    }

    /// Try to read the next available slot header. Returns `None` if empty.
    pub fn try_peek_header(&self) -> Option<SlotHeader> {
        let r = self.ring.read_cursor.load(Ordering::Relaxed);
        let w = self.ring.write_cursor.load(Ordering::Acquire);
        if r >= w {
            return None;
        }

        let offset = self.ring.slot_offset(r);
        let slot = &self.data[offset..offset + self.ring.slot_size];
        let header: &SlotHeader = bytemuck::from_bytes(&slot[..SLOT_HEADER_SIZE]);

        // Check sequence number — producer writes (slot_index + 1).
        if header.sequence != (r + 1) as u32 {
            return None; // Slot not yet released.
        }

        Some(*header)
    }

    /// Read raw bytes from the next slot. Returns `(slot_index, payload, header)`.
    pub fn try_read_bytes(&self) -> Option<(usize, &[u8], SlotHeader)> {
        let r = self.ring.read_cursor.load(Ordering::Relaxed);
        let w = self.ring.write_cursor.load(Ordering::Acquire);
        if r >= w {
            return None;
        }

        let offset = self.ring.slot_offset(r);
        let slot = &self.data[offset..offset + self.ring.slot_size];
        let header: &SlotHeader = bytemuck::from_bytes(&slot[..SLOT_HEADER_SIZE]);

        if header.sequence != (r + 1) as u32 {
            return None;
        }

        let payload = &slot[SLOT_HEADER_SIZE..SLOT_HEADER_SIZE + self.ring.slot_size - SLOT_HEADER_SIZE];
        Some((r, payload, *header))
    }

    /// Read a typed payload from the next slot.
    pub fn try_read<T: Pod>(&self) -> Option<(usize, T, SlotHeader)> {
        let (index, payload_bytes, header) = self.try_read_bytes()?;
        let payload_size = std::mem::size_of::<T>();
        if payload_size > payload_bytes.len() {
            return None;
        }
        let payload: &T = bytemuck::from_bytes(&payload_bytes[..payload_size]);
        Some((index, *payload, header))
    }

    /// Consume the current slot and advance the consumer cursor.
    pub fn consume(&self) {
        fence(Ordering::Release);
        self.ring.read_cursor.fetch_add(1, Ordering::AcqRel);
    }

    /// Read and consume in one call (convenience).
    pub fn read<T: Pod>(&self) -> Option<(T, SlotHeader)> {
        let (_index, payload, header) = self.try_read::<T>()?;
        self.consume();
        Some((payload, header))
    }

    /// Number of slots available for reading right now.
    pub fn available(&self) -> usize {
        let w = self.ring.write_cursor.load(Ordering::Acquire);
        let r = self.ring.read_cursor.load(Ordering::Relaxed);
        w.saturating_sub(r)
    }

    /// Spin-wait until at least `n` slots are available for reading.
    pub fn wait_available(&self, n: usize) {
        let mut backoff = 1u64;
        loop {
            if self.available() >= n {
                return;
            }
            if backoff <= 32 {
                for _ in 0..backoff {
                    std::hint::spin_loop();
                }
                backoff = backoff.saturating_mul(2);
            } else {
                thread::yield_now();
                backoff = 1;
            }
        }
    }
}

// ── MPSC Ring Buffer ────────────────────────────────────────────────

/// Multi-Producer Single-Consumer ring buffer.
///
/// Multiple threads may concurrently call `MpscProducer::write_*` —
/// the CAS on `write_cursor` ensures exactly one producer claims each slot.
pub struct MpscRing {
    /// Producer write cursor — cache-line padded.
    pub write_cursor: AtomicPaddedUsize,
    /// Consumer read cursor — cache-line padded.
    pub read_cursor: AtomicPaddedUsize,
    /// Total number of slots. Always a power of two.
    pub capacity: usize,
    /// Size of each slot in bytes (header + payload).
    pub slot_size: usize,
}

impl MpscRing {
    pub fn new(capacity: usize, slot_size: usize) -> RingResult<Self> {
        if capacity == 0 || !capacity.is_power_of_two() {
            return Err(RingError::InvalidCapacity(capacity));
        }
        if slot_size < SLOT_HEADER_SIZE {
            return Err(RingError::PayloadTooLarge { payload: 0, slot: slot_size });
        }

        Ok(Self {
            write_cursor: AtomicPaddedUsize::new(0),
            read_cursor: AtomicPaddedUsize::new(0),
            capacity,
            slot_size,
        })
    }

    pub fn data_bytes(&self) -> usize {
        self.capacity * self.slot_size
    }

    pub fn available_slots(&self, write: usize, read: usize) -> usize {
        self.capacity - (write - read)
    }

    pub fn slot_offset(&self, index: usize) -> usize {
        (index & (self.capacity - 1)) * self.slot_size
    }
}

/// Write handle for the MPSC ring buffer. Cloned per producer thread.
pub struct MpscProducer<'a> {
    ring: &'a MpscRing,
    data: &'a mut [u8],
}

impl<'a> MpscProducer<'a> {
    pub fn new(ring: &'a MpscRing, data: &'a mut [u8]) -> Self {
        Self { ring, data }
    }

    /// Write raw bytes via CAS-claimed slot. Returns the slot index.
    pub fn write_bytes(&mut self, payload: &[u8]) -> RingResult<usize> {
        if payload.len() + SLOT_HEADER_SIZE > self.ring.slot_size {
            return Err(RingError::PayloadTooLarge {
                payload: payload.len(),
                slot: self.ring.slot_size,
            });
        }

        // CAS-loop to claim a slot.
        let mut backoff = 1u64;
        let slot_index = loop {
            let w = self.ring.write_cursor.load(Ordering::Relaxed);
            let r = self.ring.read_cursor.load(Ordering::Acquire);
            if self.ring.available_slots(w, r) == 0 {
                // Buffer full — spin.
                if backoff <= 32 {
                    for _ in 0..backoff {
                        std::hint::spin_loop();
                    }
                    backoff = backoff.saturating_mul(2);
                } else {
                    thread::yield_now();
                    backoff = 1;
                }
                continue;
            }
            match self.ring.write_cursor.compare_exchange(
                w,
                w + 1,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(index) => break index,
                Err(_) => {
                    backoff = 1;
                    continue;
                }
            }
        };

        let offset = self.ring.slot_offset(slot_index);
        let slot = &mut self.data[offset..offset + self.ring.slot_size];

        slot.fill(0);
        slot[SLOT_HEADER_SIZE..SLOT_HEADER_SIZE + payload.len()]
            .copy_from_slice(payload);

        let seq = (slot_index + 1) as u32;
        let header = SlotHeader { sequence: seq, flags: 0 };
        slot[..SLOT_HEADER_SIZE].copy_from_slice(bytemuck::bytes_of(&header));
        fence(Ordering::Release);

        Ok(slot_index)
    }

    /// Write a Pod payload via CAS-claimed slot.
    pub fn write_raw<T: Pod>(&mut self, payload: &T) -> RingResult<usize> {
        self.write_bytes(bytemuck::bytes_of(payload))
    }

    pub fn available(&self) -> usize {
        let w = self.ring.write_cursor.load(Ordering::Relaxed);
        let r = self.ring.read_cursor.load(Ordering::Acquire);
        self.ring.available_slots(w, r)
    }
}

/// Consumer handle for the MPSC ring buffer.
pub struct MpscConsumer<'a> {
    ring: &'a MpscRing,
    data: &'a [u8],
}

impl<'a> MpscConsumer<'a> {
    pub fn new(ring: &'a MpscRing, data: &'a [u8]) -> Self {
        Self { ring, data }
    }

    /// Try to read the next slot's header.
    pub fn try_peek_header(&self) -> Option<SlotHeader> {
        let r = self.ring.read_cursor.load(Ordering::Relaxed);
        let w = self.ring.write_cursor.load(Ordering::Acquire);
        if r >= w {
            return None;
        }

        let offset = self.ring.slot_offset(r);
        let slot = &self.data[offset..offset + self.ring.slot_size];
        let header: &SlotHeader = bytemuck::from_bytes(&slot[..SLOT_HEADER_SIZE]);
        if header.sequence != (r + 1) as u32 {
            return None;
        }
        Some(*header)
    }

    /// Read raw bytes from the next slot.
    pub fn try_read_bytes(&self) -> Option<(usize, &[u8], SlotHeader)> {
        let r = self.ring.read_cursor.load(Ordering::Relaxed);
        let w = self.ring.write_cursor.load(Ordering::Acquire);
        if r >= w {
            return None;
        }

        let offset = self.ring.slot_offset(r);
        let slot = &self.data[offset..offset + self.ring.slot_size];
        let header: &SlotHeader = bytemuck::from_bytes(&slot[..SLOT_HEADER_SIZE]);
        if header.sequence != (r + 1) as u32 {
            return None;
        }

        let payload = &slot[SLOT_HEADER_SIZE..SLOT_HEADER_SIZE + self.ring.slot_size - SLOT_HEADER_SIZE];
        Some((r, payload, *header))
    }

    /// Read a typed payload from the next slot.
    pub fn try_read<T: Pod>(&self) -> Option<(usize, T, SlotHeader)> {
        let (index, payload_bytes, header) = self.try_read_bytes()?;
        let payload_size = std::mem::size_of::<T>();
        if payload_size > payload_bytes.len() {
            return None;
        }
        let payload: &T = bytemuck::from_bytes(&payload_bytes[..payload_size]);
        Some((index, *payload, header))
    }

    /// Consume the current slot and advance cursor.
    pub fn consume(&self) {
        fence(Ordering::Release);
        self.ring.read_cursor.fetch_add(1, Ordering::AcqRel);
    }

    /// Read and consume in one call.
    pub fn read<T: Pod>(&self) -> Option<(T, SlotHeader)> {
        let (_index, payload, header) = self.try_read::<T>()?;
        self.consume();
        Some((payload, header))
    }

    pub fn available(&self) -> usize {
        let w = self.ring.write_cursor.load(Ordering::Acquire);
        let r = self.ring.read_cursor.load(Ordering::Relaxed);
        w.saturating_sub(r)
    }
}

// ── Shared Memory Layout Builder ────────────────────────────────────

/// Compute the total shared memory size for a ring buffer configuration.
pub fn shared_memory_size(capacity: usize, slot_size: usize) -> usize {
    capacity * slot_size
}

/// Validate that a buffer configuration is sound.
pub fn validate_config(capacity: usize, slot_size: usize) -> RingResult<()> {
    if capacity == 0 || !capacity.is_power_of_two() {
        return Err(RingError::InvalidCapacity(capacity));
    }
    if slot_size < SLOT_HEADER_SIZE {
        return Err(RingError::PayloadTooLarge { payload: 0, slot: slot_size });
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_size_tiers() {
        assert_eq!(SlotSize::XS.capacity(), 256);
        assert_eq!(SlotSize::S.capacity(), 1024);
        assert_eq!(SlotSize::M.capacity(), 4096);
        assert_eq!(SlotSize::L.capacity(), 16_384);
        assert_eq!(SlotSize::XL.capacity(), 65_536);
    }

    #[test]
    fn spsc_new_valid() {
        let ring = SpscRing::new(8, 64).unwrap();
        assert_eq!(ring.capacity, 8);
        assert_eq!(ring.slot_size, 64);
        assert_eq!(ring.data_bytes(), 512);
    }

    #[test]
    fn spsc_new_invalid_capacity() {
        assert!(matches!(SpscRing::new(3, 64), Err(RingError::InvalidCapacity(3))));
        assert!(matches!(SpscRing::new(0, 64), Err(RingError::InvalidCapacity(0))));
    }

    #[test]
    fn spsc_new_slot_too_small() {
        assert!(SpscRing::new(8, 4).is_err());
    }

    #[test]
    fn mpsc_new_valid() {
        let ring = MpscRing::new(16, 128).unwrap();
        assert_eq!(ring.capacity, 16);
        assert_eq!(ring.slot_size, 128);
    }

    #[test]
    fn slot_offset_masking() {
        let ring = SpscRing::new(4, 64).unwrap();
        assert_eq!(ring.slot_offset(0), 0);
        assert_eq!(ring.slot_offset(1), 64);
        assert_eq!(ring.slot_offset(2), 128);
        assert_eq!(ring.slot_offset(3), 192);
        // Wraps: index 4 & 3 = 0.
        assert_eq!(ring.slot_offset(4), 0);
        assert_eq!(ring.slot_offset(5), 64);
    }

    #[test]
    fn available_slots_calculation() {
        let ring = SpscRing::new(8, 64).unwrap();
        assert_eq!(ring.available_slots(0, 0), 8);
        assert_eq!(ring.available_slots(3, 1), 6);
        assert_eq!(ring.available_slots(8, 0), 0);
    }

    #[test]
    fn error_flags_bitor() {
        let flags = ErrorFlags::PAYLOAD_EXCEEDED | ErrorFlags::CRC_MISMATCH;
        assert!(flags.contains(ErrorFlags::PAYLOAD_EXCEEDED));
        assert!(flags.contains(ErrorFlags::CRC_MISMATCH));
        assert!(!flags.contains(ErrorFlags::CONSUMER_TIMEOUT));
    }

    #[test]
    fn error_flags_bits() {
        assert_eq!(ErrorFlags::NONE.bits(), 0);
        assert_eq!(ErrorFlags::PAYLOAD_EXCEEDED.bits(), 0x1);
        assert_eq!(ErrorFlags::CRC_MISMATCH.bits(), 0x2);
        assert_eq!(ErrorFlags::CONSUMER_TIMEOUT.bits(), 0x4);
    }

    #[test]
    fn validate_config_ok() {
        assert!(validate_config(8, 64).is_ok());
        assert!(validate_config(1024, 4096).is_ok());
    }

    #[test]
    fn validate_config_errors() {
        assert!(validate_config(3, 64).is_err());
        assert!(validate_config(0, 64).is_err());
        assert!(validate_config(8, 4).is_err());
    }

    #[test]
    fn shared_memory_size_calculation() {
        assert_eq!(shared_memory_size(8, 64), 512);
        assert_eq!(shared_memory_size(1024, 4096), 4_194_304);
    }

    #[test]
    fn atomic_padded_usize_basic_ops() {
        let a = AtomicPaddedUsize::new(42);
        assert_eq!(a.load(Ordering::Relaxed), 42);
        a.store(100, Ordering::Relaxed);
        assert_eq!(a.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn atomic_padded_usize_cas() {
        let a = AtomicPaddedUsize::new(0);
        assert!(a.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Relaxed).is_ok());
        assert_eq!(a.load(Ordering::Relaxed), 1);
        assert!(a.compare_exchange(0, 2, Ordering::AcqRel, Ordering::Relaxed).is_err());
    }

    #[test]
    fn atomic_padded_usize_fetch_add() {
        let a = AtomicPaddedUsize::new(10);
        let prev = a.fetch_add(5, Ordering::AcqRel);
        assert_eq!(prev, 10);
        assert_eq!(a.load(Ordering::Relaxed), 15);
    }

    #[test]
    fn slot_header_roundtrip() {
        let header = SlotHeader { sequence: 42, flags: 0x3 };
        let bytes = bytemuck::bytes_of(&header);
        let recovered: &SlotHeader = bytemuck::from_bytes(bytes);
        assert_eq!(recovered.sequence, 42);
        assert_eq!(recovered.flags, 0x3);
    }

    #[test]
    fn spsc_basic_write_read_u64() {
        let ring = SpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            producer.write_raw(&0xDEAD_BEEF_u64).unwrap();
            producer.write_raw(&0xCAFE_BABE_u64).unwrap();
        }

        {
            let consumer = SpscConsumer::new(&ring, &data);
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, 0xDEAD_BEEF);
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, 0xCAFE_BABE);
            assert!(consumer.read::<u64>().is_none());
        }
    }

    #[test]
    fn spsc_write_read_bytes() {
        let ring = SpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];
        let msg = b"hello world";

        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            producer.write_bytes(msg).unwrap();
        }

        {
            let consumer = SpscConsumer::new(&ring, &data);
            let (idx, payload, _hdr) = consumer.try_read_bytes().unwrap();
            assert_eq!(idx, 0);
            assert_eq!(&payload[..msg.len()], msg);
        }
    }

    #[test]
    fn spsc_ring_drain() {
        let ring = SpscRing::new(2, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            producer.write_raw(&1_u32).unwrap();
            producer.write_raw(&2_u32).unwrap();
        }

        {
            let consumer = SpscConsumer::new(&ring, &data);
            assert_eq!(consumer.read::<u32>().unwrap().0, 1);
            assert_eq!(consumer.read::<u32>().unwrap().0, 2);
            assert!(consumer.read::<u32>().is_none());
        }
    }

    #[test]
    fn spsc_many_slots() {
        let n = 64;
        let ring = SpscRing::new(n, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            for i in 0..n as u64 {
                producer.write_raw(&i).unwrap();
            }
        }

        {
            let consumer = SpscConsumer::new(&ring, &data);
            for i in 0..n as u64 {
                let (val, _hdr) = consumer.read::<u64>().unwrap();
                assert_eq!(val, i);
            }
            assert!(consumer.read::<u64>().is_none());
        }
    }

    #[test]
    fn spsc_available_counts() {
        let ring = SpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        // Write one slot, then drop producer to release the mutable borrow.
        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            assert_eq!(producer.available(), 4);
            producer.write_raw(&1_u64).unwrap();
            assert_eq!(producer.available(), 3);
        }

        // Now create consumer with shared access.
        {
            let consumer = SpscConsumer::new(&ring, &data);
            assert_eq!(consumer.available(), 1);
            consumer.read::<u64>().unwrap();
            assert_eq!(consumer.available(), 0);
        }
    }

    #[test]
    fn spsc_write_with_flags() {
        let ring = SpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            producer.write_with_flags(b"test", ErrorFlags::CRC_MISMATCH).unwrap();
        }

        {
            let consumer = SpscConsumer::new(&ring, &data);
            let (idx, _payload, hdr) = consumer.try_read_bytes().unwrap();
            assert_eq!(idx, 0);
            assert!(ErrorFlags(hdr.flags).contains(ErrorFlags::CRC_MISMATCH));
        }
    }

    #[test]
    fn spsc_payload_too_large() {
        let ring = SpscRing::new(4, 16).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];
        let mut producer = SpscProducer::new(&ring, &mut data);

        // Payload of 16 bytes + 8 header = 24 > 16 slot size.
        let result = producer.write_raw(&[0u8; 16]);
        assert!(result.is_err());
    }

    #[test]
    fn spsc_wrap_around() {
        let ring = SpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        // Interleave writes and reads to exercise wrap-around.
        for i in 0..8u64 {
            {
                let mut producer = SpscProducer::new(&ring, &mut data);
                producer.write_raw(&i).unwrap();
            }
            {
                let consumer = SpscConsumer::new(&ring, &data);
                let (val, _hdr) = consumer.read::<u64>().unwrap();
                assert_eq!(val, i);
            }
        }
    }

    #[test]
    fn mpsc_basic_write_read() {
        let ring = MpscRing::new(8, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = MpscProducer::new(&ring, &mut data);
            producer.write_raw(&42_u64).unwrap();
            producer.write_raw(&99_u64).unwrap();
        }

        {
            let consumer = MpscConsumer::new(&ring, &data);
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, 42);
            let (val, _hdr) = consumer.read::<u64>().unwrap();
            assert_eq!(val, 99);
            assert!(consumer.read::<u64>().is_none());
        }
    }

    #[test]
    fn mpsc_available_counts() {
        let ring = MpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = MpscProducer::new(&ring, &mut data);
            assert_eq!(producer.available(), 4);
            producer.write_raw(&1_u64).unwrap();
        }

        {
            let consumer = MpscConsumer::new(&ring, &data);
            assert_eq!(consumer.available(), 1);
            consumer.read::<u64>().unwrap();
            assert_eq!(consumer.available(), 0);
        }
    }

    #[test]
    fn mpsc_write_bytes() {
        let ring = MpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];
        let msg = b"mpsc payload";

        {
            let mut producer = MpscProducer::new(&ring, &mut data);
            producer.write_bytes(msg).unwrap();
        }

        {
            let consumer = MpscConsumer::new(&ring, &data);
            let (_idx, payload, _hdr) = consumer.try_read_bytes().unwrap();
            assert_eq!(&payload[..msg.len()], msg);
        }
    }

    #[test]
    fn mpsc_wrap_around() {
        let ring = MpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        // Interleave writes and reads to exercise wrap-around.
        for i in 0..8u64 {
            {
                let mut producer = MpscProducer::new(&ring, &mut data);
                producer.write_raw(&i).unwrap();
            }
            {
                let consumer = MpscConsumer::new(&ring, &data);
                let (val, _hdr) = consumer.read::<u64>().unwrap();
                assert_eq!(val, i);
            }
        }
    }

    #[test]
    fn mpsc_consumer_peek_header() {
        let ring = MpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = MpscProducer::new(&ring, &mut data);
            producer.write_raw(&1_u32).unwrap();
        }

        {
            let consumer = MpscConsumer::new(&ring, &data);
            let hdr = consumer.try_peek_header().unwrap();
            assert_eq!(hdr.sequence, 1);
            assert_eq!(hdr.flags, 0);
            // Peek doesn't consume — available is still 1.
            assert_eq!(consumer.available(), 1);
        }
    }

    #[test]
    fn spsc_consumer_peek_header() {
        let ring = SpscRing::new(4, 64).unwrap();
        let mut data = vec![0u8; ring.data_bytes()];

        {
            let mut producer = SpscProducer::new(&ring, &mut data);
            producer.write_raw(&7_u32).unwrap();
        }

        {
            let consumer = SpscConsumer::new(&ring, &data);
            let hdr = consumer.try_peek_header().unwrap();
            assert_eq!(hdr.sequence, 1);
            // Still available.
            assert_eq!(consumer.available(), 1);
        }
    }
}
