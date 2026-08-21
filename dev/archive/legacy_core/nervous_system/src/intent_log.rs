// Append-Only Intent Log & Deterministic Replay
// Provides crash-safe logging of all mutation intents for debugging and replay.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use memmap2::{MmapMut, MmapOptions};

/// Log entry magic number
pub const LOG_MAGIC: u32 = 0x1A73E7; // "INTENT" inspired
pub const LOG_ENTRY_HEADER_SIZE: usize = 48;
pub const LOG_INITIAL_SIZE: usize = 64 * 1024 * 1024; // 64 MB
pub const LOG_GROWTH_FACTOR: usize = 2;

/// Log entry header - fixed size for fast seeking
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy)]
pub struct LogEntryHeader {
    pub magic: u32,
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub source_id: u64,
    pub packet_type: u8,
    pub priority: u8,
    pub schema_version: u16,
    pub payload_length: u32,
    pub checksum: u32,
    pub generation: u64,
}

impl LogEntryHeader {
    pub fn compute_checksum(&self) -> u32 {
        let mut acc = 0u32;
        acc = acc.wrapping_add(self.magic);
        acc = acc.wrapping_add(self.sequence as u32);
        acc = acc.wrapping_add((self.sequence >> 32) as u32);
        acc = acc.wrapping_add(self.timestamp_ns as u32);
        acc = acc.wrapping_add((self.timestamp_ns >> 32) as u32);
        acc = acc.wrapping_add(self.source_id as u32);
        acc = acc.wrapping_add((self.source_id >> 32) as u32);
        acc = acc.wrapping_add(self.packet_type as u32);
        acc = acc.wrapping_add(self.priority as u32);
        acc = acc.wrapping_add(self.schema_version as u32);
        acc = acc.wrapping_add(self.payload_length);
        acc = acc.wrapping_add(self.generation as u32);
        acc = acc.wrapping_add((self.generation >> 32) as u32);
        acc
    }

    pub fn verify(&self) -> bool {
        self.magic == LOG_MAGIC && self.checksum == self.compute_checksum()
    }
}

/// Append-only log file for mutation intents
pub struct IntentLog {
    file: std::fs::File,
    mmap: MmapMut,
    write_offset: usize,
    entry_count: u64,
    path: PathBuf,
}

impl IntentLog {
    pub fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create log directory")?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .context("Failed to open intent log")?;

        let file_len = file.metadata()?.len();
        if file_len == 0 {
            file.set_len(LOG_INITIAL_SIZE as u64)
                .context("Failed to initialize log file")?;
        }

        let mmap = unsafe {
            MmapOptions::new()
                .map_mut(&file)
                .context("Failed to mmap log")?
        };

        let entry_count = u64::from_le_bytes(mmap[0..8].try_into().unwrap_or([0; 8]));

        // Calculate write offset by scanning entries
        let write_offset = if entry_count > 0 {
            Self::calculate_write_offset(&mmap, entry_count)
        } else {
            8
        };

        Ok(Self {
            file,
            mmap,
            write_offset,
            entry_count,
            path: path.to_path_buf(),
        })
    }

    fn calculate_write_offset(mmap: &[u8], entry_count: u64) -> usize {
        let mut offset: usize = 8;
        for _ in 0..entry_count {
            if offset + LOG_ENTRY_HEADER_SIZE > mmap.len() {
                break;
            }
            let mut header = LogEntryHeader {
                magic: 0,
                sequence: 0,
                timestamp_ns: 0,
                source_id: 0,
                packet_type: 0,
                priority: 0,
                schema_version: 0,
                payload_length: 0,
                checksum: 0,
                generation: 0,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    mmap.as_ptr().add(offset),
                    &mut header as *mut LogEntryHeader as *mut u8,
                    LOG_ENTRY_HEADER_SIZE,
                );
            }
            offset += LOG_ENTRY_HEADER_SIZE + header.payload_length as usize;
        }
        offset
    }

    pub fn append(&mut self, header: &LogEntryHeader, payload: &[u8]) -> Result<u64> {
        let needed = LOG_ENTRY_HEADER_SIZE + payload.len();
        self.ensure_capacity(needed)?;

        let offset = self.write_offset;

        // Write header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                header as *const LogEntryHeader as *const u8,
                LOG_ENTRY_HEADER_SIZE,
            )
        };
        self.mmap[offset..offset + LOG_ENTRY_HEADER_SIZE].copy_from_slice(header_bytes);

        // Write payload
        let payload_start = offset + LOG_ENTRY_HEADER_SIZE;
        self.mmap[payload_start..payload_start + payload.len()].copy_from_slice(payload);

        // Update entry count at start of file
        let count_bytes = (self.entry_count + 1).to_le_bytes();
        self.mmap[0..8].copy_from_slice(&count_bytes);

        self.mmap.flush()?;

        let seq = self.entry_count;
        self.entry_count += 1;
        self.write_offset += needed;

        Ok(seq)
    }

    pub fn ensure_capacity(&mut self, needed: usize) -> Result<()> {
        if self.write_offset + needed > self.mmap.len() {
            let new_size = self.mmap.len() * LOG_GROWTH_FACTOR;
            self.file
                .set_len(new_size as u64)
                .context("Failed to grow log file")?;
            self.mmap = unsafe {
                MmapOptions::new()
                    .map_mut(&self.file)
                    .context("Failed to remap log")?
            };
        }
        Ok(())
    }

    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Sequential/random access log reader
pub struct LogReader {
    mmap: memmap2::Mmap,
    entry_count: u64,
}

impl LogReader {
    pub fn open(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path).context("Failed to open log for reading")?;
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .context("Failed to mmap log")?
        };

        let entry_count = u64::from_le_bytes(mmap[0..8].try_into().unwrap_or([0; 8]));

        Ok(Self { mmap, entry_count })
    }

    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    pub fn get_entry(&self, sequence: u64) -> Result<Option<(LogEntryHeader, Vec<u8>)>> {
        if sequence >= self.entry_count {
            return Ok(None);
        }

        let mut offset: usize = 8;
        for _ in 0..sequence {
            if offset + LOG_ENTRY_HEADER_SIZE > self.mmap.len() {
                return Ok(None);
            }
            let mut header = LogEntryHeader {
                magic: 0,
                sequence: 0,
                timestamp_ns: 0,
                source_id: 0,
                packet_type: 0,
                priority: 0,
                schema_version: 0,
                payload_length: 0,
                checksum: 0,
                generation: 0,
            };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    self.mmap.as_ptr().add(offset),
                    &mut header as *mut LogEntryHeader as *mut u8,
                    LOG_ENTRY_HEADER_SIZE,
                );
            }
            offset += LOG_ENTRY_HEADER_SIZE + header.payload_length as usize;
        }

        if offset + LOG_ENTRY_HEADER_SIZE > self.mmap.len() {
            return Ok(None);
        }

        let mut header = LogEntryHeader {
            magic: 0,
            sequence: 0,
            timestamp_ns: 0,
            source_id: 0,
            packet_type: 0,
            priority: 0,
            schema_version: 0,
            payload_length: 0,
            checksum: 0,
            generation: 0,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.mmap.as_ptr().add(offset),
                &mut header as *mut LogEntryHeader as *mut u8,
                LOG_ENTRY_HEADER_SIZE,
            );
        }

        let payload_start = offset + LOG_ENTRY_HEADER_SIZE;
        let payload_end = payload_start + header.payload_length as usize;
        let payload = self.mmap[payload_start..payload_end].to_vec();

        Ok(Some((header, payload)))
    }

    pub fn iter(&self) -> LogEntryIter<'_> {
        LogEntryIter {
            mmap: &self.mmap,
            offset: 8,
            remaining: self.entry_count,
        }
    }
}

pub struct LogEntryIter<'a> {
    mmap: &'a [u8],
    offset: usize,
    remaining: u64,
}

impl<'a> Iterator for LogEntryIter<'a> {
    type Item = (LogEntryHeader, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        if self.offset + LOG_ENTRY_HEADER_SIZE > self.mmap.len() {
            return None;
        }

        let mut header = LogEntryHeader {
            magic: 0,
            sequence: 0,
            timestamp_ns: 0,
            source_id: 0,
            packet_type: 0,
            priority: 0,
            schema_version: 0,
            payload_length: 0,
            checksum: 0,
            generation: 0,
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.mmap.as_ptr().add(self.offset),
                &mut header as *mut LogEntryHeader as *mut u8,
                LOG_ENTRY_HEADER_SIZE,
            );
        }

        let payload_start = self.offset + LOG_ENTRY_HEADER_SIZE;
        let payload_end = payload_start + header.payload_length as usize;

        if payload_end > self.mmap.len() {
            return None;
        }

        let payload = &self.mmap[payload_start..payload_end];

        self.offset = payload_end;
        self.remaining -= 1;

        Some((header, payload))
    }
}

/// Generation snapshot for replay checkpointing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerationSnapshot {
    pub generation: u64,
    pub log_sequence: u64,
    pub timestamp_ns: u64,
    pub state_bytes: Vec<u8>,
    pub checksum: u32,
}

impl GenerationSnapshot {
    pub fn compute_checksum(&self) -> u32 {
        let mut acc = 0u32;
        acc = acc.wrapping_add(self.generation as u32);
        acc = acc.wrapping_add((self.generation >> 32) as u32);
        acc = acc.wrapping_add(self.log_sequence as u32);
        acc = acc.wrapping_add((self.log_sequence >> 32) as u32);
        acc = acc.wrapping_add(self.timestamp_ns as u32);
        acc = acc.wrapping_add((self.timestamp_ns >> 32) as u32);
        for chunk in self.state_bytes.chunks(4) {
            let mut val = 0u32;
            for (i, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (i * 8);
            }
            acc = acc.wrapping_add(val);
        }
        acc
    }

    pub fn verify(&self) -> bool {
        self.checksum == self.compute_checksum()
    }
}

/// Snapshot store for generation checkpoints
pub struct SnapshotStore {
    snapshots: std::collections::BTreeMap<u64, GenerationSnapshot>,
    snapshot_interval: u64,
    path: PathBuf,
}

impl SnapshotStore {
    pub fn new(path: &Path, snapshot_interval: u64) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create snapshot directory")?;
        }

        let mut store = Self {
            snapshots: std::collections::BTreeMap::new(),
            snapshot_interval,
            path: path.to_path_buf(),
        };

        if path.exists() {
            store.load_from_disk()?;
        }

        Ok(store)
    }

    pub fn should_snapshot(&self, current_generation: u64) -> bool {
        current_generation > 0
            && current_generation.is_multiple_of(self.snapshot_interval)
            && !self.snapshots.contains_key(&current_generation)
    }

    pub fn save_snapshot(&mut self, mut snapshot: GenerationSnapshot) -> Result<()> {
        snapshot.checksum = snapshot.compute_checksum();
        self.snapshots.insert(snapshot.generation, snapshot);
        self.persist_to_disk()
    }

    pub fn find_nearest_snapshot(&self, target_generation: u64) -> Option<&GenerationSnapshot> {
        self.snapshots
            .range(..=target_generation)
            .next_back()
            .map(|(_, s)| s)
    }

    fn persist_to_disk(&self) -> Result<()> {
        let bytes = serde_json::to_vec(&self.snapshots).context("Failed to serialize snapshots")?;
        std::fs::write(&self.path, &bytes).context("Failed to write snapshots")?;
        Ok(())
    }

    fn load_from_disk(&mut self) -> Result<()> {
        let bytes = std::fs::read(&self.path).context("Failed to read snapshots")?;
        self.snapshots =
            serde_json::from_slice(&bytes).context("Failed to deserialize snapshots")?;
        Ok(())
    }
}

/// Replay report summarizing log replay results
#[derive(Debug, Default)]
pub struct ReplayReport {
    pub total_entries: u64,
    pub successful: u64,
    pub failures: u64,
    pub continue_on_error: bool,
    pub last_error: Option<ReplayError>,
}

#[derive(Debug)]
pub struct ReplayError {
    pub sequence: u64,
    pub error: String,
    pub header: LogEntryHeader,
}

/// Helper to create log entry headers from intent data
pub fn create_log_entry(
    sequence: u64,
    source_id: u64,
    packet_type: u8,
    priority: u8,
    generation: u64,
    payload_length: u32,
) -> LogEntryHeader {
    let mut header = LogEntryHeader {
        magic: LOG_MAGIC,
        sequence,
        timestamp_ns: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64,
        source_id,
        packet_type,
        priority,
        schema_version: crate::swmr_synapse::SCHEMA_VERSION as u16,
        payload_length,
        checksum: 0,
        generation,
    };
    header.checksum = header.compute_checksum();
    header
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nucleotide_packet::packet_types;

    fn temp_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("aaroneous_test_{}", name));
        path
    }

    #[test]
    fn test_log_entry_header_checksum() {
        let header = create_log_entry(0, 42, packet_types::INTENT, 2, 0, 4);
        assert!(header.verify());
    }

    #[test]
    fn test_intent_log_append_read() {
        let path = temp_path("log_append");
        let _ = std::fs::remove_file(&path);

        let mut log = IntentLog::new(&path).unwrap();

        let h1 = create_log_entry(0, 1, packet_types::INTENT, 1, 0, 8);
        let h2 = create_log_entry(1, 2, packet_types::STATE_READ, 2, 1, 8);

        let seq1 = log.append(&h1, b"payload1").unwrap();
        let seq2 = log.append(&h2, b"payload2").unwrap();

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(log.entry_count(), 2);

        // Read back
        let reader = LogReader::open(&path).unwrap();
        assert_eq!(reader.entry_count(), 2);

        let (h1_read, p1_read) = reader.get_entry(0).unwrap().unwrap();
        assert_eq!(h1_read.source_id, 1);
        assert_eq!(p1_read, b"payload1");

        let (h2_read, p2_read) = reader.get_entry(1).unwrap().unwrap();
        assert_eq!(h2_read.source_id, 2);
        assert_eq!(p2_read, b"payload2");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_log_reader_iter() {
        let path = temp_path("log_iter");
        let _ = std::fs::remove_file(&path);

        let mut log = IntentLog::new(&path).unwrap();

        for i in 0..5 {
            let h = create_log_entry(i, i + 1, packet_types::INTENT, 1, i, 7);
            log.append(&h, b"payload").unwrap();
        }

        let reader = LogReader::open(&path).unwrap();
        let entries: Vec<_> = reader.iter().collect();

        assert_eq!(entries.len(), 5);
        for (i, (header, payload)) in entries.iter().enumerate() {
            assert_eq!(header.sequence, i as u64);
            assert_eq!(header.source_id, i as u64 + 1);
            assert_eq!(*payload, b"payload");
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_snapshot_store() {
        let path = temp_path("snapshots");
        let _ = std::fs::remove_file(&path);

        let mut store = SnapshotStore::new(&path, 10).unwrap();

        assert!(store.should_snapshot(10));
        assert!(!store.should_snapshot(5));

        let snapshot = GenerationSnapshot {
            generation: 10,
            log_sequence: 5,
            timestamp_ns: 12345,
            state_bytes: vec![1, 2, 3, 4],
            checksum: 0,
        };

        store.save_snapshot(snapshot).unwrap();
        assert!(store.find_nearest_snapshot(15).is_some());
        assert!(store.find_nearest_snapshot(5).is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_generation_snapshot_checksum() {
        let mut snapshot = GenerationSnapshot {
            generation: 42,
            log_sequence: 100,
            timestamp_ns: 999,
            state_bytes: vec![1, 2, 3, 4, 5],
            checksum: 0,
        };
        snapshot.checksum = snapshot.compute_checksum();
        assert!(snapshot.verify());

        snapshot.generation = 99;
        assert!(!snapshot.verify());
    }
}
