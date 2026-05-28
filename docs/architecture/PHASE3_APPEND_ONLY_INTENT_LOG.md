# Phase 3: Append-Only Intent Log & Deterministic Replay

## Problem Statement
Non-deterministic bugs in distributed Raft consensus + NATS mesh are impossible to reproduce. Agent panics inside Wasmtime sandboxes cannot be debugged with logs alone.

## Architecture

### 1. Append-Only Intent Log

```rust
use std::fs::OpenOptions;
use std::io::{Write, Seek, SeekFrom};
use memmap2::{MmapMut, MmapOptions};

/// Log entry header - fixed size for fast seeking
#[repr(C, align(8))]
pub struct LogEntryHeader {
    pub magic: u32,          // 0x1NT3NT (INTENT)
    pub sequence: u64,       // Global monotonic sequence
    pub timestamp_ns: u64,   // Nanosecond precision
    pub source_id: u64,      // Agent/specialist ID hash
    pub packet_type: u8,     // Intent/StateRead/Notification/Response
    pub priority: u8,
    pub schema_version: u16,
    pub payload_length: u32,
    pub checksum: u32,
    pub generation: u64,     // Synapse generation at time of intent
}

pub const LOG_ENTRY_HEADER_SIZE: usize = 48;
pub const LOG_MAGIC: u32 = 0x1NT3NT;

/// Append-only log file
pub struct IntentLog {
    file: std::fs::File,
    mmap: MmapMut,
    write_offset: u64,
    entry_count: u64,
    path: PathBuf,
}

impl IntentLog {
    pub fn new(path: &Path) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        
        let file_len = file.metadata()?.len();
        if file_len == 0 {
            // Initialize with 64 MB
            file.set_len(64 * 1024 * 1024)?;
        }
        
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        
        // Read existing entry count from header
        let entry_count = if file_len > 0 {
            // First 8 bytes store entry count
            u64::from_le_bytes(mmap[0..8].try_into().unwrap())
        } else {
            0
        };
        
        Ok(Self {
            file,
            mmap,
            write_offset: 8 + entry_count * (LOG_ENTRY_HEADER_SIZE as u64),
            entry_count,
            path: path.to_path_buf(),
        })
    }
    
    pub fn append(&mut self, header: &LogEntryHeader, payload: &[u8]) -> Result<u64> {
        // Write header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                header as *const LogEntryHeader as *const u8,
                LOG_ENTRY_HEADER_SIZE,
            )
        };
        
        let offset = self.write_offset as usize;
        self.mmap[offset..offset + LOG_ENTRY_HEADER_SIZE].copy_from_slice(header_bytes);
        
        // Write payload
        let payload_start = offset + LOG_ENTRY_HEADER_SIZE;
        self.mmap[payload_start..payload_start + payload.len()].copy_from_slice(payload);
        
        // Update entry count
        let count_bytes = (self.entry_count + 1).to_le_bytes();
        self.mmap[0..8].copy_from_slice(&count_bytes);
        
        self.mmap.flush()?;
        
        let seq = self.entry_count;
        self.entry_count += 1;
        self.write_offset += LOG_ENTRY_HEADER_SIZE as u64 + payload.len() as u64;
        
        Ok(seq)
    }
    
    /// Grow the log file if approaching capacity
    pub fn ensure_capacity(&mut self, needed: usize) -> Result<()> {
        if self.write_offset as usize + needed > self.mmap.len() {
            let new_size = self.mmap.len() * 2;
            self.file.set_len(new_size as u64)?;
            self.mmap = unsafe { MmapOptions::new().map_mut(&self.file)? };
        }
        Ok(())
    }
}
```

### 2. Log Replay Engine

```rust
pub struct LogReplayer {
    log: IntentLog,
    wasm_runtime: WasmtimeEngine,
}

impl LogReplayer {
    /// Replay the entire log from the beginning
    pub async fn replay_full(&mut self) -> Result<ReplayReport> {
        let mut report = ReplayReport::default();
        let mut reader = LogReader::open(&self.log.path)?;
        
        while let Some((header, payload)) = reader.next_entry()? {
            report.total_entries += 1;
            
            // Recreate the exact system state at this point
            self.restore_generation(header.generation).await?;
            
            // Feed the exact packet into the WASM runtime
            match self.process_replay_entry(&header, &payload).await {
                Ok(_) => report.successful += 1,
                Err(e) => {
                    report.failures += 1;
                    report.last_error = Some(ReplayError {
                        sequence: header.sequence,
                        error: e.to_string(),
                        header: header.clone(),
                    });
                    
                    // Stop at first failure for debugging
                    if !report.continue_on_error {
                        break;
                    }
                }
            }
        }
        
        Ok(report)
    }
    
    /// Replay from a specific sequence number
    pub async fn replay_from(&mut self, from_sequence: u64) -> Result<ReplayReport> {
        let mut reader = LogReader::open(&self.log.path)?;
        reader.seek_to_sequence(from_sequence)?;
        // ... same as replay_full but starting from offset
        todo!()
    }
    
    /// Replay a single entry step-by-step with breakpoints
    pub async fn replay_step(&mut self, sequence: u64) -> Result<StepResult> {
        let reader = LogReader::open(&self.log.path)?;
        let (header, payload) = reader.get_entry(sequence)?;
        
        // Set breakpoint before processing
        self.set_breakpoint(sequence);
        
        // Process single entry
        self.process_replay_entry(&header, &payload).await
    }
    
    async fn process_replay_entry(
        &mut self,
        header: &LogEntryHeader,
        payload: &[u8],
    ) -> Result<()> {
        // Reconstruct NucleotidePacket from log entry
        let packet = NucleotidePacket::new(
            header.sequence,
            header.source_id,
            header.packet_type,
            header.priority,
            0, // Payload is inline in replay
            payload.len() as u32,
        );
        
        // Feed into WASM runtime exactly as original
        self.wasm_runtime.inject_packet(&packet, payload).await
    }
    
    async fn restore_generation(&mut self, generation: u64) -> Result<()> {
        // Load synapse state from the specified generation
        // This requires generation snapshots (see below)
        todo!()
    }
}
```

### 3. Generation Snapshots

To enable replay from any point, we periodically snapshot the synapse state:

```rust
pub struct GenerationSnapshot {
    pub generation: u64,
    pub log_sequence: u64,    // Log entry at time of snapshot
    pub timestamp_ns: u64,
    pub state_bytes: Vec<u8>, // rkyv-serialized SynapseState
    pub checksum: u32,
}

pub struct SnapshotStore {
    snapshots: BTreeMap<u64, GenerationSnapshot>, // generation -> snapshot
    snapshot_interval: u64,  // Snapshot every N generations
    path: PathBuf,
}

impl SnapshotStore {
    pub fn should_snapshot(&self, current_generation: u64) -> bool {
        current_generation % self.snapshot_interval == 0
    }
    
    pub fn save_snapshot(&mut self, snapshot: GenerationSnapshot) -> Result<()> {
        self.snapshots.insert(snapshot.generation, snapshot);
        self.persist_to_disk()?;
        Ok(())
    }
    
    /// Find the closest snapshot before a target generation
    pub fn find_nearest_snapshot(&self, target_generation: u64) -> Option<&GenerationSnapshot> {
        self.snapshots
            .range(..=target_generation)
            .next_back()
            .map(|(_, s)| s)
    }
}
```

### 4. Integration with Single Writer

The Intent Log integrates directly with the SWMR Single Writer loop:

```
[Mutation Intent] ──> [ IntentValidator ]
                           │
                    (validation passes)
                           │
                    ┌──────▼──────┐
                    │ Intent Log  │ ←── Append entry BEFORE commit
                    └──────┬──────┘
                           │
                    [ Commit/Swap ]
                           │
                    [ PreparednessNotice ]
```

Modified `run_writer_loop`:

```rust
pub async fn run_writer_loop(&mut self) -> Result<()> {
    let mut log = IntentLog::new(&self.log_path)?;
    let mut snapshots = SnapshotStore::new(&self.snapshot_path)?;
    
    while let Some(intent) = self.intent_queue.receive().await {
        if !validator.validate(&intent) {
            continue;
        }
        
        // Create log entry
        let header = LogEntryHeader {
            magic: LOG_MAGIC,
            sequence: log.entry_count(),
            timestamp_ns: current_timestamp_ns(),
            source_id: intent.source_id,
            packet_type: packet_types::INTENT,
            priority: intent.priority,
            schema_version: SCHEMA_VERSION as u16,
            payload_length: intent.value.len() as u32,
            checksum: compute_checksum(&intent.value),
            generation: self.generation.generation(),
        };
        
        // Append to log BEFORE committing
        log.ensure_capacity(LOG_ENTRY_HEADER_SIZE + intent.value.len())?;
        log.append(&header, &intent.value)?;
        
        // Snapshot if needed
        if snapshots.should_snapshot(self.generation.generation()) {
            let state = self.read_current_state(&self.mmap.read().await)?;
            let snapshot = GenerationSnapshot {
                generation: self.generation.generation(),
                log_sequence: log.entry_count(),
                timestamp_ns: current_timestamp_ns(),
                state_bytes: rkyv::to_bytes::<_, 256>(&state)?,
                checksum: 0,
            };
            snapshots.save_snapshot(snapshot)?;
        }
        
        // Now commit (existing logic)
        // ...
    }
    
    Ok(())
}
```

### 5. CLI Replay Tool

```bash
# Replay entire log
aaroneous replay --log intent.log --full

# Replay from sequence 10000
aaroneous replay --log intent.log --from 10000

# Step through single entry with debug output
aaroneous replay --log intent.log --step 10042 --verbose

# Find the entry that caused a crash
aaroneous replay --log intent.log --full --continue-on-error --report crash_report.json
```

### 6. File Format

```
Intent Log File (.intentlog)
┌─────────────────────────────────────┐
│ Entry Count (8 bytes)               │
├─────────────────────────────────────┤
│ Entry 0 Header (48 bytes)           │
│ Entry 0 Payload (variable)          │
├─────────────────────────────────────┤
│ Entry 1 Header (48 bytes)           │
│ Entry 1 Payload (variable)          │
├─────────────────────────────────────┤
│ ...                                 │
└─────────────────────────────────────┘

Snapshot File (.snapshot)
┌─────────────────────────────────────┐
│ Snapshot Count (8 bytes)            │
├─────────────────────────────────────┤
│ Snapshot 0: Generation + State      │
├─────────────────────────────────────┤
│ Snapshot 1: Generation + State      │
├─────────────────────────────────────┤
│ ...                                 │
└─────────────────────────────────────┘
```

## Implementation Order
1. Define `LogEntryHeader` struct with `#[repr(C, align(8))]`
2. Implement `IntentLog` with mmap append
3. Implement `LogReader` for sequential/random access
4. Integrate with SWMR Single Writer loop
5. Implement `GenerationSnapshot` and `SnapshotStore`
6. Build `LogReplayer` engine
7. Create CLI replay tool
8. Add dashboard panel for log browsing
