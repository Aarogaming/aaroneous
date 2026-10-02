// Black-Box Flight Recorder & Replayer
// Circular zero-copy binary logging with RDTSC timestamping and deterministic replay.

use core_contracts::{FlightEventKind, FlightEventPod, FlightFileHeaderPod};
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

/// Total file size for the pre-allocated circular flight log (16 MB exact)
pub const FLIGHT_LOG_SIZE: usize = 16 * 1024 * 1024;

/// Size of the page-aligned header region (4 KB exact)
pub const FLIGHT_HEADER_SIZE: usize = 4096;

/// Fixed size of each flight event slot (128 bytes exact)
pub const FLIGHT_SLOT_SIZE: usize = 128;

/// Maximum number of event slots within the 16 MB ring buffer (131,040 slots)
/// Calculated as (16,777,216 - 4,096) / 128 = 131,040
pub const FLIGHT_MAX_SLOTS: usize = (FLIGHT_LOG_SIZE - FLIGHT_HEADER_SIZE) / FLIGHT_SLOT_SIZE;

/// Magic signature for flight recorder log files: b"AAROFLGT"
pub const FLIGHT_MAGIC: [u8; 8] = *b"AAROFLGT";

/// Binary format schema version
pub const FLIGHT_VERSION: u32 = 1;

/// Structured error conditions for flight recorder and replay operations
#[derive(Debug, thiserror::Error)]
pub enum FlightRecorderError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Corrupted flight log header magic: expected {expected:?}, got {found:?}")]
    CorruptedMagic { expected: [u8; 8], found: [u8; 8] },

    #[error("Unsupported flight log version: expected {expected}, got {found}")]
    UnsupportedVersion { expected: u32, found: u32 },

    #[error("Invalid log file size: expected at least {expected} bytes, found {found} bytes")]
    InvalidFileSize { expected: usize, found: u64 },

    #[error(
        "Checksum mismatch for sequence {sequence}: expected {expected:#x}, calculated {calculated:#x}"
    )]
    ChecksumMismatch {
        sequence: u64,
        expected: u32,
        calculated: u32,
    },

    #[error(
        "Event slot {sequence} overwritten by circular wrap (oldest available is {oldest_available})"
    )]
    EventOverwritten {
        sequence: u64,
        oldest_available: u64,
    },

    #[error("Requested sequence {sequence} is ahead of write sequence {write_sequence}")]
    FutureSequence { sequence: u64, write_sequence: u64 },

    #[error("Invalid sequence number: {0}")]
    InvalidSequence(u64),

    #[error("POD deserialization error: {0}")]
    PodError(String),

    #[error("workspace build lineage must be at most 24 lowercase Base36 characters")]
    InvalidBuildLineage,
}

/// Reads the high-resolution hardware timestamp counter (`_rdtsc`).
#[inline]
pub fn read_rdtsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: _rdtsc is a standard x86_64 CPU instruction supported on all x86_64 targets with no memory side effects.
        unsafe { core::arch::x86_64::_rdtsc() }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}

/// Reads current wall-clock time in nanoseconds since UNIX epoch.
#[inline]
pub fn read_wall_clock_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// Black-Box Flight Recorder.
/// Maintains a zero-allocation, pre-allocated 16 MB memory-mapped circular buffer for
/// ultra-low-overhead recording of causal state transitions and telemetry events.
pub struct FlightRecorder {
    mmap: memmap2::MmapMut,
    path: PathBuf,
    sequence: u64,
    wrap_count: u64,
}

impl FlightRecorder {
    /// Opens an existing flight log or initializes a new 16 MB circular buffer.
    pub fn open_or_create(path: &Path) -> Result<Self, FlightRecorderError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let current_len = file.metadata()?.len();
        if current_len < FLIGHT_LOG_SIZE as u64 {
            file.set_len(FLIGHT_LOG_SIZE as u64)?;
        }

        // SAFETY: The file was created/opened exclusively with read/write permissions
        // and truncated or extended to FLIGHT_LOG_SIZE.
        let mut mmap = unsafe { memmap2::MmapOptions::new().map_mut(&file)? };

        let header_slice = &mmap[..std::mem::size_of::<FlightFileHeaderPod>()];
        let existing_header = bytemuck::try_from_bytes::<FlightFileHeaderPod>(header_slice).ok();

        let (sequence, wrap_count) = match existing_header {
            Some(h)
                if h.magic == FLIGHT_MAGIC
                    && h.version == FLIGHT_VERSION
                    && h.slot_size == FLIGHT_SLOT_SIZE as u32
                    && h.max_events == FLIGHT_MAX_SLOTS as u32 =>
            {
                (h.write_sequence, h.wrap_count)
            }
            _ => {
                let mut build_lineage = [0u8; 24];
                let pkg_version = env!("CARGO_PKG_VERSION");
                if let Some(idx) = pkg_version.find("+vb.") {
                    let payload = &pkg_version[idx + 4..];
                    if payload.len() > build_lineage.len()
                        || !payload
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || byte.is_ascii_lowercase())
                    {
                        return Err(FlightRecorderError::InvalidBuildLineage);
                    }
                    build_lineage[..payload.len()].copy_from_slice(payload.as_bytes());
                }

                let header = FlightFileHeaderPod {
                    magic: FLIGHT_MAGIC,
                    version: FLIGHT_VERSION,
                    max_events: FLIGHT_MAX_SLOTS as u32,
                    slot_size: FLIGHT_SLOT_SIZE as u32,
                    header_size: FLIGHT_HEADER_SIZE as u32,
                    write_sequence: 0,
                    wrap_count: 0,
                    build_lineage,
                };
                let header_bytes = bytemuck::bytes_of(&header);
                mmap[..header_bytes.len()].copy_from_slice(header_bytes);
                mmap[header_bytes.len()..FLIGHT_HEADER_SIZE].fill(0);
                mmap.flush()?;
                (0, 0)
            }
        };

        Ok(Self {
            mmap,
            path: path.to_path_buf(),
            sequence,
            wrap_count,
        })
    }

    /// Records an event pod into the circular buffer.
    /// Monotonically increments sequence, stamps hardware clock, calculates checksum,
    /// and writes into the corresponding circular slot.
    pub fn record_event(&mut self, mut event: FlightEventPod) -> Result<u64, FlightRecorderError> {
        self.sequence = self.sequence.saturating_add(1);
        let seq = self.sequence;
        event.sequence = seq;

        if event.timestamp_rdtsc == 0 {
            event.timestamp_rdtsc = read_rdtsc();
        }
        if event.wall_clock_ns == 0 {
            event.wall_clock_ns = read_wall_clock_ns();
        }

        event.checksum = event.calculate_checksum();

        let slot_idx = ((seq - 1) % (FLIGHT_MAX_SLOTS as u64)) as usize;
        let offset = FLIGHT_HEADER_SIZE + slot_idx * FLIGHT_SLOT_SIZE;
        self.wrap_count = (seq - 1) / (FLIGHT_MAX_SLOTS as u64);

        let event_bytes = bytemuck::bytes_of(&event);
        self.mmap[offset..offset + FLIGHT_SLOT_SIZE].copy_from_slice(event_bytes);

        // Update header write_sequence and wrap_count
        let header_slice = &mut self.mmap[..std::mem::size_of::<FlightFileHeaderPod>()];
        if let Ok(header_mut) = bytemuck::try_from_bytes_mut::<FlightFileHeaderPod>(header_slice) {
            header_mut.write_sequence = seq;
            header_mut.wrap_count = self.wrap_count;
        }

        Ok(seq)
    }

    /// Helper to record a state transition or command event with automatic hashing and payload packing.
    pub fn record_transition(
        &mut self,
        kind: FlightEventKind,
        source_id: u16,
        input_hash: u64,
        pre_state_hash: u64,
        post_state_hash: u64,
        payload: &[u8],
    ) -> Result<u64, FlightRecorderError> {
        let mut event = FlightEventPod {
            timestamp_rdtsc: 0,
            wall_clock_ns: 0,
            sequence: 0,
            input_hash,
            pre_state_hash,
            post_state_hash,
            event_kind: kind as u16,
            source_id,
            payload_len: 0,
            flags: 0,
            checksum: 0,
            data_payload: [0u8; 64],
        };
        event.set_payload(payload);
        self.record_event(event)
    }

    /// Current monotonic write sequence.
    pub fn write_sequence(&self) -> u64 {
        self.sequence
    }

    /// Number of times the circular ring buffer has wrapped.
    pub fn wrap_count(&self) -> u64 {
        self.wrap_count
    }

    /// Flushes changes to disk.
    pub fn flush(&self) -> Result<(), FlightRecorderError> {
        self.mmap.flush()?;
        Ok(())
    }

    /// Target file path of the flight recorder log.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Total event capacity before wrapping.
    pub fn capacity(&self) -> usize {
        FLIGHT_MAX_SLOTS
    }
}

/// Black-Box Flight Replayer.
/// Reads and validates causal event logs point-in-time with zero-copy decoding.
pub struct FlightReplayer {
    mmap: memmap2::Mmap,
    path: PathBuf,
}

impl FlightReplayer {
    /// Opens an existing flight log for replay and analysis.
    pub fn open(path: &Path) -> Result<Self, FlightRecorderError> {
        let file = OpenOptions::new().read(true).open(path)?;
        let file_len = file.metadata()?.len();
        if file_len < (FLIGHT_HEADER_SIZE + FLIGHT_SLOT_SIZE) as u64 {
            return Err(FlightRecorderError::InvalidFileSize {
                expected: FLIGHT_HEADER_SIZE + FLIGHT_SLOT_SIZE,
                found: file_len,
            });
        }

        // SAFETY: Read-only memory mapping over valid open file descriptor.
        let mmap = unsafe { memmap2::MmapOptions::new().map(&file)? };

        let header_slice = &mmap[..std::mem::size_of::<FlightFileHeaderPod>()];
        let header = bytemuck::try_from_bytes::<FlightFileHeaderPod>(header_slice)
            .map_err(|e| FlightRecorderError::PodError(format!("{:?}", e)))?;

        if header.magic != FLIGHT_MAGIC {
            return Err(FlightRecorderError::CorruptedMagic {
                expected: FLIGHT_MAGIC,
                found: header.magic,
            });
        }

        if header.version != FLIGHT_VERSION {
            return Err(FlightRecorderError::UnsupportedVersion {
                expected: FLIGHT_VERSION,
                found: header.version,
            });
        }

        Ok(Self {
            mmap,
            path: path.to_path_buf(),
        })
    }

    /// Accesses the verified flight file header.
    pub fn header(&self) -> Result<&FlightFileHeaderPod, FlightRecorderError> {
        let header_slice = &self.mmap[..std::mem::size_of::<FlightFileHeaderPod>()];
        bytemuck::try_from_bytes::<FlightFileHeaderPod>(header_slice)
            .map_err(|e| FlightRecorderError::PodError(format!("{:?}", e)))
    }

    /// Range of causal event sequence numbers currently available in the circular buffer `(oldest, latest)`.
    /// Returns `(0, 0)` if no events have been recorded.
    pub fn available_event_range(&self) -> Result<(u64, u64), FlightRecorderError> {
        let header = self.header()?;
        let write_seq = header.write_sequence;
        if write_seq == 0 {
            return Ok((0, 0));
        }

        let max_events = header.max_events as u64;
        let oldest = if write_seq > max_events {
            write_seq - max_events + 1
        } else {
            1
        };

        Ok((oldest, write_seq))
    }

    /// Reads and verifies an event by sequence number.
    /// Verifies checksum and ensures slot was not overwritten by a circular wrap.
    pub fn read_event(&self, sequence: u64) -> Result<FlightEventPod, FlightRecorderError> {
        let (oldest, write_seq) = self.available_event_range()?;

        if sequence == 0 {
            return Err(FlightRecorderError::InvalidSequence(0));
        }
        if sequence > write_seq {
            return Err(FlightRecorderError::FutureSequence {
                sequence,
                write_sequence: write_seq,
            });
        }
        if sequence < oldest {
            return Err(FlightRecorderError::EventOverwritten {
                sequence,
                oldest_available: oldest,
            });
        }

        let header = self.header()?;
        let slot_idx = ((sequence - 1) % (header.max_events as u64)) as usize;
        let offset = header.header_size as usize + slot_idx * (header.slot_size as usize);
        let slot_bytes = &self.mmap[offset..offset + (header.slot_size as usize)];

        let event: &FlightEventPod = bytemuck::try_from_bytes(slot_bytes)
            .map_err(|e| FlightRecorderError::PodError(format!("{:?}", e)))?;

        if event.sequence != sequence {
            return Err(FlightRecorderError::EventOverwritten {
                sequence,
                oldest_available: oldest,
            });
        }

        if !event.verify_checksum() {
            return Err(FlightRecorderError::ChecksumMismatch {
                sequence,
                expected: event.checksum,
                calculated: event.calculate_checksum(),
            });
        }

        Ok(*event)
    }

    /// Reads the trailing `n` events up to the latest write sequence.
    pub fn read_tail(&self, n: usize) -> Result<Vec<FlightEventPod>, FlightRecorderError> {
        let (oldest, write_seq) = self.available_event_range()?;
        if write_seq == 0 || n == 0 {
            return Ok(Vec::new());
        }

        let start_seq = write_seq
            .saturating_sub((n as u64).saturating_sub(1))
            .max(oldest);
        let count = (write_seq - start_seq + 1) as usize;
        let mut events = Vec::with_capacity(count);

        for seq in start_seq..=write_seq {
            events.push(self.read_event(seq)?);
        }

        Ok(events)
    }

    /// Creates an iterator that replays all currently available events in chronological order.
    pub fn iter(&self) -> Result<FlightReplayIterator<'_>, FlightRecorderError> {
        let (oldest, write_seq) = self.available_event_range()?;
        Ok(FlightReplayIterator {
            replayer: self,
            current_seq: oldest,
            end_seq: write_seq,
        })
    }

    /// Target file path of the replayer.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Chronological iterator over available events in a flight recorder log.
pub struct FlightReplayIterator<'a> {
    replayer: &'a FlightReplayer,
    current_seq: u64,
    end_seq: u64,
}

impl<'a> Iterator for FlightReplayIterator<'a> {
    type Item = Result<FlightEventPod, FlightRecorderError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_seq == 0 || self.current_seq > self.end_seq {
            return None;
        }

        let seq = self.current_seq;
        self.current_seq += 1;
        Some(self.replayer.read_event(seq))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flight_recorder_create_and_append() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("test.flight");

        // 1. Initialize flight recorder
        let mut recorder = FlightRecorder::open_or_create(&path).expect("open_or_create");
        assert_eq!(recorder.write_sequence(), 0);
        assert_eq!(recorder.wrap_count(), 0);

        // 2. Append 5 events
        for i in 1..=5 {
            let seq = recorder
                .record_transition(
                    FlightEventKind::StateTransition,
                    0x01,
                    0xAAAA0000 + i,
                    0xBBBB0000 + i,
                    0xCCCC0000 + i,
                    format!("action_{}", i).as_bytes(),
                )
                .expect("record_transition");
            assert_eq!(seq, i);
        }

        assert_eq!(recorder.write_sequence(), 5);
        assert_eq!(recorder.wrap_count(), 0);
        recorder.flush().expect("flush");

        // 3. Open replayer and verify
        let replayer = FlightReplayer::open(&path).expect("replayer open");
        let header = replayer.header().expect("header");
        assert_eq!(header.magic, FLIGHT_MAGIC);
        assert_eq!(header.version, FLIGHT_VERSION);
        assert_eq!(header.write_sequence, 5);
        assert_eq!(header.wrap_count, 0);
        if let Some((_, lineage)) = env!("CARGO_PKG_VERSION").split_once("+vb.") {
            assert_eq!(&header.build_lineage[..lineage.len()], lineage.as_bytes());
            assert!(
                header.build_lineage[lineage.len()..]
                    .iter()
                    .all(|byte| *byte == 0)
            );
        }

        let (oldest, latest) = replayer.available_event_range().expect("range");
        assert_eq!(oldest, 1);
        assert_eq!(latest, 5);

        for seq in 1..=5 {
            let event = replayer.read_event(seq).expect("read_event");
            assert_eq!(event.sequence, seq);
            assert_eq!(event.event_kind, FlightEventKind::StateTransition as u16);
            assert_eq!(event.source_id, 0x01);
            assert_eq!(event.input_hash, 0xAAAA0000 + seq);
            assert_eq!(event.payload(), format!("action_{}", seq).as_bytes());
            assert!(event.verify_checksum());
        }

        // 4. Test read_tail
        let tail = replayer.read_tail(3).expect("read_tail");
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].sequence, 3);
        assert_eq!(tail[1].sequence, 4);
        assert_eq!(tail[2].sequence, 5);

        // 5. Test iterator
        let all_events: Result<Vec<_>, _> = replayer.iter().expect("iter").collect();
        let all_events = all_events.expect("iter collect");
        assert_eq!(all_events.len(), 5);
        assert_eq!(all_events[0].sequence, 1);
        assert_eq!(all_events[4].sequence, 5);
    }

    #[test]
    fn test_flight_recorder_wrapping_at_16mb() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("wrap_test.flight");

        let mut recorder = FlightRecorder::open_or_create(&path).expect("open_or_create");

        // Write FLIGHT_MAX_SLOTS + 5 events to trigger circular wrapping
        let total_events = FLIGHT_MAX_SLOTS as u64 + 5;
        let sample_event = FlightEventPod {
            event_kind: FlightEventKind::TelemetryTick as u16,
            source_id: 0x02,
            ..Default::default()
        };

        for _ in 1..=total_events {
            recorder.record_event(sample_event).expect("record_event");
        }

        assert_eq!(recorder.write_sequence(), total_events);
        assert_eq!(recorder.wrap_count(), 1);
        recorder.flush().expect("flush");

        let replayer = FlightReplayer::open(&path).expect("replayer open");
        let (oldest, latest) = replayer.available_event_range().expect("range");
        assert_eq!(latest, total_events);
        assert_eq!(oldest, 6); // Slots 1..=5 were overwritten by slots 131,041..=131,045

        // Sequence 1 should fail with EventOverwritten
        match replayer.read_event(1) {
            Err(FlightRecorderError::EventOverwritten {
                sequence: 1,
                oldest_available: 6,
            }) => {}
            other => panic!("Expected EventOverwritten for seq 1, got {:?}", other),
        }

        // Sequence 5 should also fail with EventOverwritten
        match replayer.read_event(5) {
            Err(FlightRecorderError::EventOverwritten {
                sequence: 5,
                oldest_available: 6,
            }) => {}
            other => panic!("Expected EventOverwritten for seq 5, got {:?}", other),
        }

        // Sequence 6 should succeed
        let ev6 = replayer.read_event(6).expect("read event 6");
        assert_eq!(ev6.sequence, 6);
        assert!(ev6.verify_checksum());

        // Sequence latest should succeed
        let ev_latest = replayer.read_event(latest).expect("read event latest");
        assert_eq!(ev_latest.sequence, latest);
        assert!(ev_latest.verify_checksum());
    }

    #[test]
    fn test_flight_replayer_checksum_verification() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("checksum_test.flight");

        let mut recorder = FlightRecorder::open_or_create(&path).expect("open_or_create");
        recorder
            .record_transition(
                FlightEventKind::AnomalyTrigger,
                0x01,
                111,
                222,
                333,
                b"anomaly_detected",
            )
            .expect("record");
        recorder.flush().expect("flush");
        drop(recorder);

        // Tamper with the event in the file
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open for tamper");

        use std::io::{Seek, SeekFrom, Write};
        // Seek to the data payload of slot 0: 4096 (header) + 64 (offset into FlightEventPod)
        file.seek(SeekFrom::Start(4096 + 64)).expect("seek");
        file.write_all(b"CORRUPTED").expect("write corruption");
        drop(file);

        let replayer = FlightReplayer::open(&path).expect("replayer open");
        match replayer.read_event(1) {
            Err(FlightRecorderError::ChecksumMismatch { sequence: 1, .. }) => {}
            other => panic!("Expected ChecksumMismatch, got {:?}", other),
        }
    }

    #[test]
    fn test_flight_recorder_persistence_reopen() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("reopen_test.flight");

        {
            let mut recorder = FlightRecorder::open_or_create(&path).expect("open 1");
            for i in 1..=10 {
                recorder
                    .record_transition(
                        FlightEventKind::MutationCommitted,
                        0x03,
                        i,
                        i * 2,
                        i * 3,
                        b"commit",
                    )
                    .expect("record");
            }
            assert_eq!(recorder.write_sequence(), 10);
        }

        // Re-open and verify sequence resumes
        {
            let mut recorder = FlightRecorder::open_or_create(&path).expect("open 2");
            assert_eq!(recorder.write_sequence(), 10);
            for i in 11..=15 {
                let seq = recorder
                    .record_transition(
                        FlightEventKind::MutationCommitted,
                        0x03,
                        i,
                        i * 2,
                        i * 3,
                        b"commit",
                    )
                    .expect("record");
                assert_eq!(seq, i);
            }
            assert_eq!(recorder.write_sequence(), 15);
        }

        let replayer = FlightReplayer::open(&path).expect("replayer open");
        let (oldest, latest) = replayer.available_event_range().expect("range");
        assert_eq!(oldest, 1);
        assert_eq!(latest, 15);
        assert_eq!(replayer.read_tail(15).expect("tail").len(), 15);
    }
}
