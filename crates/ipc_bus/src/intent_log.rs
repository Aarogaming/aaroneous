// Append-Only Intent Log & Deterministic Replay
// Provides crash-safe logging of all mutation intents for debugging and replay.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use memmap2::{MmapMut, MmapOptions};

/// Log entry magic number
pub const LOG_MAGIC: u32 = 0x1A73E7; // "INTENT" inspired
/// Computed from the struct itself rather than hardcoded: this was previously
/// a hand-written `48`, four bytes short of the real 56-byte size once
/// `align(8)` padding is accounted for (the trailing `generation: u64` field
/// needs 8-byte alignment, padding `checksum`'s end at offset 44 up to 48
/// before `generation` occupies bytes 48..56). That mismatch silently
/// truncated every persisted entry's `generation` field to zero - it was
/// never written past offset 48, and never read back either, since both the
/// write path's `slice::from_raw_parts` and the read paths'
/// `ptr::copy_nonoverlapping` only ever touched `LOG_ENTRY_HEADER_SIZE`
/// bytes. See `test_log_entry_header_size_matches_struct_layout` and
/// `test_intent_log_round_trips_nonzero_generation` below.
pub const LOG_ENTRY_HEADER_SIZE: usize = std::mem::size_of::<LogEntryHeader>();
pub const LOG_INITIAL_SIZE: usize = 64 * 1024 * 1024; // 64 MB
pub const LOG_GROWTH_FACTOR: usize = 2;

/// On-disk file format version, bumped from the implicit "version 1" (no
/// version field existed at all; entries started immediately at byte
/// offset 8) to 2 alongside the `LOG_ENTRY_HEADER_SIZE` fix above. Without
/// this, opening a log file written by the old code with the new code
/// would silently misinterpret the first 8 bytes of what used to be
/// payload data as part of the (now 8-bytes-wider) header, desyncing
/// every subsequent entry's computed offset - a real data-corruption bug
/// a review caught before this shipped. See
/// `existing_file_has_incompatible_version`/`archive_incompatible_log`.
pub const LOG_FORMAT_VERSION: u32 = 2;
/// File header layout: `entry_count: u64` at bytes `0..8`, `format_version:
/// u32` at bytes `8..12`. Entries begin immediately after, at this offset.
pub const LOG_FILE_HEADER_SIZE: usize = 12;

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

        if Self::existing_file_has_incompatible_version(path)? {
            Self::archive_incompatible_log(path)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .context("Failed to open intent log")?;

        let file_len = file.metadata()?.len();
        let is_fresh = file_len == 0;
        if is_fresh {
            file.set_len(LOG_INITIAL_SIZE as u64)
                .context("Failed to initialize log file")?;
        }

        // `map_mut`'s unsafety is inherent to mmap - the OS can't stop
        // another process from concurrently truncating or writing the backing
        // file underneath us. This file was just opened/sized by this call
        // (or already exists as a log this process previously created and
        // just had its format_version confirmed compatible above), and
        // `IntentLog` is the sole owner of the mapping once constructed.
        // SAFETY: sole owner of a file this call just opened/sized.
        let mut mmap = unsafe {
            MmapOptions::new()
                .map_mut(&file)
                .context("Failed to mmap log")?
        };

        let (entry_count, write_offset) = if is_fresh {
            mmap[0..8].copy_from_slice(&0u64.to_le_bytes());
            mmap[8..12].copy_from_slice(&LOG_FORMAT_VERSION.to_le_bytes());
            (0, LOG_FILE_HEADER_SIZE)
        } else {
            let entry_count = u64::from_le_bytes(mmap[0..8].try_into().unwrap_or([0; 8]));
            let write_offset = if entry_count > 0 {
                Self::calculate_write_offset(&mmap, entry_count)
            } else {
                LOG_FILE_HEADER_SIZE
            };
            (entry_count, write_offset)
        };

        Ok(Self {
            file,
            mmap,
            write_offset,
            entry_count,
            path: path.to_path_buf(),
        })
    }

    /// `Ok(true)` iff `path` exists, is at least `LOG_FILE_HEADER_SIZE`
    /// bytes long, and its `format_version` field doesn't equal
    /// `LOG_FORMAT_VERSION` - including a pre-versioning legacy file, which
    /// has no such field at all (those bytes are actually the first 4
    /// bytes of its first entry's `magic`/`sequence`, essentially never
    /// equal to the current `LOG_FORMAT_VERSION` by construction). A
    /// missing or too-short file is not incompatible, just not there yet
    /// (or not yet initialized) - `new` handles that case itself.
    fn existing_file_has_incompatible_version(path: &Path) -> Result<bool> {
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e).context("Failed to open log file for version check"),
        };
        if file.metadata()?.len() < LOG_FILE_HEADER_SIZE as u64 {
            return Ok(false);
        }
        let mut header = [0u8; LOG_FILE_HEADER_SIZE];
        std::io::Read::read_exact(&mut file, &mut header)
            .context("Failed to read log file header for version check")?;
        let format_version = u32::from_le_bytes(header[8..12].try_into().unwrap());
        Ok(format_version != LOG_FORMAT_VERSION)
    }

    /// Renames an incompatible-format log file aside (never deletes it) so
    /// a fresh, current-format log can be created at `path`. Mirrors
    /// `rotate_segment`'s archive-then-recreate pattern, just triggered on
    /// open instead of on demand.
    fn archive_incompatible_log(path: &Path) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "intent_log".to_string());
        let mut archive_path = path.to_path_buf();
        archive_path.set_file_name(format!("{file_name}.legacy-format.{timestamp}"));
        std::fs::rename(path, &archive_path).with_context(|| {
            format!(
                "Failed to archive incompatible-format log from {} to {}",
                path.display(),
                archive_path.display()
            )
        })
    }

    fn calculate_write_offset(mmap: &[u8], entry_count: u64) -> usize {
        let mut offset: usize = LOG_FILE_HEADER_SIZE;
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
            // The loop guard above (`offset + LOG_ENTRY_HEADER_SIZE >
            // mmap.len()`) proves `[offset, offset + LOG_ENTRY_HEADER_SIZE)`
            // is in bounds for the source read. `header` is a live, properly
            // aligned local `LogEntryHeader` whose fields are all plain
            // integers - any bit pattern is a valid value, so overwriting its
            // full `size_of` (== `LOG_ENTRY_HEADER_SIZE`) via a byte copy has
            // no padding/niche hazard.
            // SAFETY: bounds-checked by the loop guard just above.
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
        // `header` is a valid `&LogEntryHeader` for the duration of this
        // call; `LOG_ENTRY_HEADER_SIZE == size_of::<LogEntryHeader>()`, so
        // this views exactly `header`'s own representation, byte for byte,
        // with no over-read.
        // SAFETY: exact-size view of `header`'s own bytes.
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
            // Same inherent mmap caveat as `IntentLog::new` above -
            // `self.file` was just grown via `set_len` and remains solely
            // owned by this `IntentLog`.
            // SAFETY: sole owner of a file this call just grew.
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

    pub fn write_offset(&self) -> usize {
        self.write_offset
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Rotates the active log file to an archived segment path and re-initializes a fresh log file
    pub fn rotate_segment(&mut self, archive_path: &Path) -> Result<()> {
        self.mmap.flush()?;
        // Same inherent mmap caveat as `IntentLog::new` above - `self.file`
        // is unchanged here (still this `IntentLog`'s own file, just
        // flushed); this is a throwaway remap immediately replaced below
        // once the file is renamed and a fresh one opened.
        // SAFETY: sole owner of this already-flushed file.
        drop(std::mem::replace(&mut self.mmap, unsafe {
            MmapOptions::new()
                .map_mut(&self.file)
                .context("Failed to temporary remap")?
        }));

        // Move current file to archive destination
        if let Some(parent) = archive_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(&self.path, archive_path)?;

        // Re-create a fresh log file at the canonical path
        let fresh_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;

        fresh_file.set_len(LOG_INITIAL_SIZE as u64)?;
        // SAFETY: same inherent mmap caveat as `IntentLog::new` above -
        // `fresh_file` was just created and sized by this call, not yet
        // shared with any other owner.
        let fresh_mmap = unsafe { MmapOptions::new().map_mut(&fresh_file)? };

        self.file = fresh_file;
        self.mmap = fresh_mmap;
        self.write_offset = LOG_FILE_HEADER_SIZE;
        self.entry_count = 0;

        // Initialize header with 0 entries and the current format version
        self.mmap[0..8].copy_from_slice(&0u64.to_le_bytes());
        self.mmap[8..12].copy_from_slice(&LOG_FORMAT_VERSION.to_le_bytes());
        self.mmap.flush()?;

        Ok(())
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
        // Same inherent mmap caveat as `IntentLog::new` - a concurrent
        // external write to the file while mapped could race, but this is a
        // read-only map of a log file this process expects to be
        // append-only and not concurrently truncated by anything else.
        // SAFETY: read-only map of an append-only log file.
        let mmap = unsafe {
            MmapOptions::new()
                .map(&file)
                .context("Failed to mmap log")?
        };

        if mmap.len() < LOG_FILE_HEADER_SIZE {
            anyhow::bail!(
                "Log file at {} is too small to contain a header",
                path.display()
            );
        }
        let format_version = u32::from_le_bytes(mmap[8..12].try_into().unwrap());
        if format_version != LOG_FORMAT_VERSION {
            anyhow::bail!(
                "Log file at {} has format version {format_version}, expected \
                 {LOG_FORMAT_VERSION} (a pre-versioning legacy log, or a newer format this \
                 build doesn't understand) - refusing to misread it rather than guessing",
                path.display()
            );
        }

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

        let mut offset: usize = LOG_FILE_HEADER_SIZE;
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
            // The loop guard above (`offset + LOG_ENTRY_HEADER_SIZE >
            // self.mmap.len()`) proves the source range is in bounds;
            // `header` is a live, aligned local of all-integer fields, so a
            // full-size byte copy into it is sound. See the identical
            // rationale on `IntentLog::calculate_write_offset` above.
            // SAFETY: bounds-checked by the loop guard just above.
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
        // SAFETY: the bounds check immediately above proves the source range
        // is in bounds; see the identical rationale on
        // `IntentLog::calculate_write_offset` above.
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
            offset: LOG_FILE_HEADER_SIZE,
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
        // SAFETY: the bounds check immediately above proves the source range
        // is in bounds; see the identical rationale on
        // `IntentLog::calculate_write_offset` above.
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
    use crate::machine_packet::packet_types;

    fn temp_path(name: &str) -> PathBuf {
        let temp_dir = tempfile::tempdir().unwrap();
        temp_dir
            .path()
            .to_path_buf()
            .join(format!("aaroneous_test_{}", name))
    }

    #[test]
    fn test_log_entry_header_checksum() {
        let header = create_log_entry(0, 42, packet_types::INTENT, 2, 0, 4);
        assert!(header.verify());
    }

    /// Regression guard for the `LOG_ENTRY_HEADER_SIZE` bug: it was
    /// previously hand-written as `48`, four bytes short of the real
    /// `align(8)`-padded 56-byte layout, which silently truncated every
    /// persisted entry's `generation` field to zero on both write and read.
    #[test]
    fn test_log_entry_header_size_matches_struct_layout() {
        assert_eq!(
            LOG_ENTRY_HEADER_SIZE,
            std::mem::size_of::<LogEntryHeader>(),
            "LOG_ENTRY_HEADER_SIZE must track the struct's real size, including \
             align(8) padding before the trailing `generation: u64` field"
        );
    }

    /// End-to-end proof that `generation` actually round-trips through the
    /// mmap'd log now, not just that the size constant matches the struct.
    #[test]
    fn test_intent_log_round_trips_nonzero_generation() {
        let path = temp_path("log_generation_roundtrip");
        let _ = std::fs::remove_file(&path);

        let mut log = IntentLog::new(&path).unwrap();
        let header = create_log_entry(0, 7, packet_types::INTENT, 1, 999, 5);
        assert_eq!(header.generation, 999);
        log.append(&header, b"hello").unwrap();

        let reader = LogReader::open(&path).unwrap();
        let (read_header, payload) = reader.get_entry(0).unwrap().unwrap();
        assert_eq!(read_header.generation, 999);
        assert!(read_header.verify());
        assert_eq!(payload, b"hello");

        let iter_header = reader.iter().next().unwrap().0;
        assert_eq!(iter_header.generation, 999);

        let _ = std::fs::remove_file(&path);
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

    /// Regression guard for the review finding on the `LOG_ENTRY_HEADER_SIZE`
    /// fix above: opening a file written by the pre-versioning code (no
    /// `format_version` field, entries starting at byte 8 with 48-byte
    /// headers) must never be parsed with the new 56-byte-header,
    /// 12-byte-file-header layout - that would silently misinterpret old
    /// payload bytes as header fields and desync every entry after the
    /// first. It must instead be archived aside untouched and replaced with
    /// a fresh, current-format log.
    #[test]
    fn test_intent_log_archives_pre_versioning_legacy_file_instead_of_misreading_it() {
        let path = temp_path("log_legacy_format");
        let _ = std::fs::remove_file(&path);

        // Simulate a log file written by the pre-format-version code: an
        // 8-byte entry_count header only, with entries starting immediately
        // at offset 8 (no format_version field ever existed). The exact
        // entry bytes don't matter - they must never be parsed at all.
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut legacy_bytes = vec![0u8; 64];
        legacy_bytes[0..8].copy_from_slice(&1u64.to_le_bytes()); // entry_count = 1
        legacy_bytes[8..12].copy_from_slice(&LOG_MAGIC.to_le_bytes());
        std::fs::write(&path, &legacy_bytes).unwrap();

        let log = IntentLog::new(&path).unwrap();
        assert_eq!(
            log.entry_count(),
            0,
            "opening a pre-versioning file must start a fresh log, not misparse the old one"
        );

        let parent = path.parent().unwrap();
        let archived: Vec<_> = std::fs::read_dir(parent)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("legacy-format"))
            .collect();
        assert_eq!(
            archived.len(),
            1,
            "the incompatible file must be archived aside, not deleted or overwritten in place"
        );
        let archived_bytes = std::fs::read(archived[0].path()).unwrap();
        assert_eq!(
            archived_bytes, legacy_bytes,
            "the archived copy must be byte-for-byte the original legacy file"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_log_reader_rejects_incompatible_format_version() {
        let path = temp_path("log_bad_version");
        let _ = std::fs::remove_file(&path);

        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut bytes = vec![0u8; 64];
        bytes[0..8].copy_from_slice(&0u64.to_le_bytes());
        bytes[8..12].copy_from_slice(&(LOG_FORMAT_VERSION + 1).to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();

        assert!(
            LogReader::open(&path).is_err(),
            "LogReader must refuse a log whose format_version it doesn't recognize, not guess"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_intent_log_segment_rotation() {
        let active_path = temp_path("rotate_active");
        let archive_path = temp_path("rotate_archive");
        let _ = std::fs::remove_file(&active_path);
        let _ = std::fs::remove_file(&archive_path);

        let mut log = IntentLog::new(&active_path).unwrap();
        let header = create_log_entry(0, 100, packet_types::INTENT, 1, 1, 0);
        log.append(&header, b"test_payload").unwrap();
        assert_eq!(log.entry_count(), 1);

        log.rotate_segment(&archive_path).unwrap();
        assert_eq!(log.entry_count(), 0);
        assert_eq!(log.write_offset(), LOG_FILE_HEADER_SIZE);

        // Verify archive has original entry
        let reader = LogReader::open(&archive_path).unwrap();
        assert_eq!(reader.entry_count(), 1);

        // Verify new active log accepts new entries
        let header2 = create_log_entry(0, 200, packet_types::NOTIFICATION, 1, 2, 1);
        log.append(&header2, b"new_payload").unwrap();
        assert_eq!(log.entry_count(), 1);

        let _ = std::fs::remove_file(&active_path);
        let _ = std::fs::remove_file(&archive_path);
    }
}
