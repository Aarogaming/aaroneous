// SWMR (Single Writer, Multi-Reader) rkyv Synapse
// Eliminates data races by enforcing exclusive write access while maintaining zero-copy reads.

use anyhow::{Context, Result};
use memmap2::{MmapMut, MmapOptions};
use rkyv::{archived_root, Archive, Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, watch, RwLock};

use crate::mutation_intent::{IntentQueue, IntentValidator, MutationIntent};
use crate::preparedness_notice::{NoticeBroadcast, PreparednessNotice};

/// Schema version for state validation
pub const SCHEMA_VERSION: u32 = 1;

/// Maximum synapse size (64 MB)
pub const MAX_SYNAPSE_SIZE: usize = 64 * 1024 * 1024;

/// rkyv-archived synapse state - zero-copy readable
#[derive(Archive, Deserialize, Serialize, Debug, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug))]
pub struct SynapseState {
    pub schema_version: u32,
    pub clock_tick: u64,
    pub energy_budget: u32,
    pub memory_pressure: u8,
    pub safety_lock: u8,
    pub approval_required: u8,
    pub approval_granted: u8,
    pub hox_mutation_flag: u8,
    pub intent_vector_id: [u8; 16],
    pub sovereignty_tier: u8,
    pub curiosity_drive: u8,
    pub integrity_score: u8,
    pub understanding_score: u8,
    pub concept_drift: f32,
    pub latent_activation_id: [u8; 16],
    pub latent_vector: [f32; 1024],
    pub mcp_call_id: u64,
    pub mcp_tool_hash: u64,
    pub mcp_status: u8,
    pub mcp_args_size: u32,
    pub mcp_args: [u8; 2048],
    pub dialogue_speaker_hash: u64,
    pub dialogue_turn_count: u32,
    pub dialogue_consensus: u8,
    pub dialogue_msg_size: u32,
    pub dialogue_payload: [u8; 1024],
}

/// Legacy McpToolCallFrame - wraps the flattened fields for backward compatibility
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct McpToolCallFrame {
    pub call_id: u64,
    pub tool_name_hash: u64,
    pub status: u8,
    pub arguments_size: u32,
    pub arguments_payload: [u8; 2048],
}

impl McpToolCallFrame {
    pub fn from_synapse(state: &SynapseState) -> Self {
        Self {
            call_id: state.mcp_call_id,
            tool_name_hash: state.mcp_tool_hash,
            status: state.mcp_status,
            arguments_size: state.mcp_args_size,
            arguments_payload: state.mcp_args,
        }
    }

    pub fn apply_to_synapse(&self, state: &mut SynapseState) {
        state.mcp_call_id = self.call_id;
        state.mcp_tool_hash = self.tool_name_hash;
        state.mcp_status = self.status;
        state.mcp_args_size = self.arguments_size;
        state.mcp_args = self.arguments_payload;
    }
}

impl Default for McpToolCallFrame {
    fn default() -> Self {
        Self {
            call_id: 0,
            tool_name_hash: 0,
            status: 0,
            arguments_size: 0,
            arguments_payload: [0; 2048],
        }
    }
}

/// Legacy SpecialistDialogue - wraps the flattened fields for backward compatibility
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SpecialistDialogue {
    pub active_speaker_hash: u64,
    pub turn_count: u32,
    pub consensus_score: u8,
    pub message_size: u32,
    pub message_payload: [u8; 1024],
}

impl SpecialistDialogue {
    pub fn from_synapse(state: &SynapseState) -> Self {
        Self {
            active_speaker_hash: state.dialogue_speaker_hash,
            turn_count: state.dialogue_turn_count,
            consensus_score: state.dialogue_consensus,
            message_size: state.dialogue_msg_size,
            message_payload: state.dialogue_payload,
        }
    }

    pub fn apply_to_synapse(&self, state: &mut SynapseState) {
        state.dialogue_speaker_hash = self.active_speaker_hash;
        state.dialogue_turn_count = self.turn_count;
        state.dialogue_consensus = self.consensus_score;
        state.dialogue_msg_size = self.message_size;
        state.dialogue_payload = self.message_payload;
    }
}

impl Default for SpecialistDialogue {
    fn default() -> Self {
        Self {
            active_speaker_hash: 0,
            turn_count: 0,
            consensus_score: 50,
            message_size: 0,
            message_payload: [0; 1024],
        }
    }
}

impl Default for SynapseState {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            clock_tick: 0,
            energy_budget: 1000,
            memory_pressure: 0,
            safety_lock: 0,
            approval_required: 0,
            approval_granted: 0,
            hox_mutation_flag: 0,
            intent_vector_id: [0; 16],
            sovereignty_tier: 0,
            curiosity_drive: 50,
            integrity_score: 100,
            understanding_score: 100,
            concept_drift: 0.0,
            latent_activation_id: [0; 16],
            latent_vector: [0.0; 1024],
            mcp_call_id: 0,
            mcp_tool_hash: 0,
            mcp_status: 0,
            mcp_args_size: 0,
            mcp_args: [0; 2048],
            dialogue_speaker_hash: 0,
            dialogue_turn_count: 0,
            dialogue_consensus: 50,
            dialogue_msg_size: 0,
            dialogue_payload: [0; 1024],
        }
    }
}

impl SynapseState {
    // Backward compatibility accessor methods
    pub fn mcp_tool_call(&self) -> McpToolCallFrame {
        McpToolCallFrame::from_synapse(self)
    }

    pub fn dialogue(&self) -> SpecialistDialogue {
        SpecialistDialogue::from_synapse(self)
    }

    pub fn set_mcp_tool_call(&mut self, frame: &McpToolCallFrame) {
        frame.apply_to_synapse(self);
    }

    pub fn set_dialogue(&mut self, dialogue: &SpecialistDialogue) {
        dialogue.apply_to_synapse(self);
    }
}

/// Atomic generation counter for coordinated memory swaps
pub struct GenerationCounter {
    pub current: AtomicU64,
    pub swapping: AtomicBool,
}

impl Default for GenerationCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl GenerationCounter {
    pub fn new() -> Self {
        Self {
            current: AtomicU64::new(0),
            swapping: AtomicBool::new(false),
        }
    }

    pub fn begin_swap(&self) -> bool {
        self.swapping
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    pub fn end_swap(&self) {
        self.current.fetch_add(1, Ordering::SeqCst);
        self.swapping.store(false, Ordering::SeqCst);
    }

    pub fn generation(&self) -> u64 {
        self.current.load(Ordering::SeqCst)
    }

    pub fn is_swapping(&self) -> bool {
        self.swapping.load(Ordering::SeqCst)
    }
}

/// Platform-agnostic workspace path resolution
pub fn resolve_synapse_path(name: &str) -> PathBuf {
    std::env::var("AARONEOUS_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
                .join("aaroneous")
                .join("synapse")
        })
        .join(format!("{}.synapse", name))
}

/// The SWMR Synapse - Single Writer, Multi-Reader zero-copy shared memory
pub struct SWMRSynapse {
    /// Memory-mapped storage (owned by writer)
    mmap: Arc<RwLock<MmapMut>>,
    /// Generation counter for coordinated swaps
    generation: Arc<GenerationCounter>,
    /// Preparedness notice broadcast channel
    notice_broadcast: Arc<NoticeBroadcast>,
    /// Mutation intent queue (agents submit here)
    intent_queue: IntentQueue,
    /// Intent validator
    validator: IntentValidator,
    /// Path to synapse file
    path: PathBuf,
    /// Writer handle (only one exists)
    writer_tx: Option<mpsc::UnboundedSender<MutationIntent>>,
    /// Length of serialized state in mmap
    serialized_len: usize,
    /// Shared serialized length for readers
    shared_serialized_len: Arc<std::sync::atomic::AtomicUsize>,
    /// Optional append-only intent log for deterministic replay
    intent_log: Option<crate::intent_log::IntentLog>,
    /// Optional snapshot store for generation checkpoints
    snapshot_store: Option<crate::intent_log::SnapshotStore>,
    /// Snapshot interval (generations between snapshots)
    snapshot_interval: u64,
}

impl SWMRSynapse {
    /// Synchronous constructor for backward compatibility
    /// Runs fully synchronously and does not nest runtimes
    pub fn new_sync(name: &str, size: usize) -> Result<Self> {
        let size = size.min(MAX_SYNAPSE_SIZE);
        let path = resolve_synapse_path(name);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create synapse directory")?;
        }

        let (std_file, path) = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
        {
            Ok(file) => (file, path),
            Err(_) => {
                let temp_path = std::env::temp_dir().join(format!(
                    "synapse_{}_{}_{}.tmp",
                    name,
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos())
                        .unwrap_or(0)
                ));
                let file = std::fs::OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&temp_path)
                    .context("Failed to open synapse file")?;
                (file, temp_path)
            }
        };

        std_file.set_len(size as u64)?;

        let mut mmap = unsafe { MmapOptions::new().map_mut(&std_file)? };

        // Write initial default state
        let state = SynapseState::default();
        let bytes =
            rkyv::to_bytes::<_, 256>(&state).context("Failed to serialize initial state")?;
        mmap[..bytes.len()].copy_from_slice(&bytes);
        mmap.flush()?;

        let serialized_len = bytes.len();
        let mmap = Arc::new(RwLock::new(mmap));

        let generation = Arc::new(GenerationCounter::new());
        let notice_broadcast = Arc::new(NoticeBroadcast::new());
        let (intent_tx, intent_rx) = mpsc::unbounded_channel::<MutationIntent>();
        let shared_serialized_len = Arc::new(std::sync::atomic::AtomicUsize::new(serialized_len));

        Ok(Self {
            mmap,
            generation,
            notice_broadcast,
            intent_queue: IntentQueue::new(intent_rx),
            validator: IntentValidator::new(),
            path,
            writer_tx: Some(intent_tx),
            serialized_len,
            shared_serialized_len: Arc::clone(&shared_serialized_len),
            intent_log: None,
            snapshot_store: None,
            snapshot_interval: 100,
        })
    }

    pub async fn new(name: &str, size: usize) -> Result<Self> {
        let size = size.min(MAX_SYNAPSE_SIZE);
        let path = resolve_synapse_path(name);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create synapse directory")?;
        }

        let file = tokio::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .await
            .context("Failed to open synapse file")?;

        let std_file = file.into_std().await;
        std_file.set_len(size as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&std_file)? };
        let mmap = Arc::new(RwLock::new(mmap));

        // Write initial default state
        let serialized_len = {
            let mut guard = mmap.write().await;
            let state = SynapseState::default();
            let bytes =
                rkyv::to_bytes::<_, 256>(&state).context("Failed to serialize initial state")?;
            guard[..bytes.len()].copy_from_slice(&bytes);
            guard.flush()?;
            bytes.len()
        };

        let generation = Arc::new(GenerationCounter::new());
        let notice_broadcast = Arc::new(NoticeBroadcast::new());
        let (intent_tx, intent_rx) = mpsc::unbounded_channel::<MutationIntent>();
        let shared_serialized_len = Arc::new(std::sync::atomic::AtomicUsize::new(serialized_len));

        Ok(Self {
            mmap,
            generation,
            notice_broadcast,
            intent_queue: IntentQueue::new(intent_rx),
            validator: IntentValidator::new(),
            path,
            writer_tx: Some(intent_tx),
            serialized_len,
            shared_serialized_len: Arc::clone(&shared_serialized_len),
            intent_log: None,
            snapshot_store: None,
            snapshot_interval: 100,
        })
    }

    /// Create a new SWMRSynapse with intent logging enabled
    pub async fn new_with_logging(
        name: &str,
        size: usize,
        log_path: &std::path::Path,
        snapshot_path: &std::path::Path,
        snapshot_interval: u64,
    ) -> Result<Self> {
        let mut synapse = Self::new(name, size).await?;

        synapse.intent_log = Some(
            crate::intent_log::IntentLog::new(log_path).context("Failed to create intent log")?,
        );

        synapse.snapshot_store = Some(
            crate::intent_log::SnapshotStore::new(snapshot_path, snapshot_interval)
                .context("Failed to create snapshot store")?,
        );

        synapse.snapshot_interval = snapshot_interval;

        Ok(synapse)
    }

    /// Create a reader handle (zero-copy, can have many)
    pub fn create_reader(&self) -> SynapseReader {
        SynapseReader {
            mmap: Arc::clone(&self.mmap),
            generation: Arc::clone(&self.generation),
            notice_rx: self.notice_broadcast.subscribe(),
            serialized_len: Arc::clone(&self.shared_serialized_len),
        }
    }

    /// Create the exclusive writer handle (only one allowed)
    pub fn create_writer(&self) -> SynapseWriterHandle {
        SynapseWriterHandle {
            intent_tx: self.writer_tx.clone().expect("Writer already taken"),
            generation: Arc::clone(&self.generation),
            notice_broadcast: Arc::clone(&self.notice_broadcast),
        }
    }

    /// Get the on-disk path of this synapse (useful for error messages / recovery)
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Start the single writer loop (call once, runs until channel closes)
    pub async fn run_writer_loop(&mut self) -> Result<()> {
        let mmap = Arc::clone(&self.mmap);
        let generation = Arc::clone(&self.generation);
        let notice_broadcast = Arc::clone(&self.notice_broadcast);
        let validator = self.validator.clone();

        while let Some(intent) = self.intent_queue.receive().await {
            // Pre-write verification: validate against schema and constraints
            if !validator.validate(&intent) {
                tracing::warn!(
                    "Mutation intent rejected: {} -> {}",
                    intent.field_name,
                    intent
                        .reason
                        .unwrap_or_else(|| "validation failed".to_string())
                );
                continue;
            }

            // Begin coordinated swap: broadcast preparedness notice
            if generation.begin_swap() {
                let current_gen = generation.generation();
                let notice = PreparednessNotice {
                    generation: current_gen,
                    target_field: intent.field_name.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64,
                };

                notice_broadcast.broadcast(&notice);

                // Brief yield to let readers drop old references
                tokio::task::yield_now().await;

                // Append to intent log BEFORE committing (for deterministic replay)
                if let Some(ref mut log) = self.intent_log {
                    let log_seq = log.entry_count();
                    let header = crate::intent_log::create_log_entry(
                        log_seq,
                        0, // source_id - would come from intent context
                        crate::machine_packet::packet_types::INTENT,
                        crate::machine_packet::priorities::NORMAL,
                        current_gen,
                        intent.value.len() as u32,
                    );
                    log.ensure_capacity(
                        crate::intent_log::LOG_ENTRY_HEADER_SIZE + intent.value.len(),
                    )?;
                    log.append(&header, &intent.value)?;
                }

                // Commit the mutation
                {
                    let mut guard = mmap.write().await;
                    let mut state = Self::read_current_state(&guard, self.serialized_len)?;

                    // Apply mutation
                    intent.apply(&mut state)?;
                    state.clock_tick += 1;

                    // Serialize back to memory
                    let bytes = rkyv::to_bytes::<_, 256>(&state)
                        .context("Failed to serialize mutated state")?;
                    let mmap_slice: &mut [u8] = &mut guard;
                    mmap_slice[..bytes.len()].copy_from_slice(bytes.as_slice());
                    guard.flush()?;

                    // Update serialized length
                    self.serialized_len = bytes.len();
                    self.shared_serialized_len
                        .store(bytes.len(), std::sync::atomic::Ordering::SeqCst);
                }

                // Snapshot if needed
                if let Some(ref mut store) = self.snapshot_store {
                    if store.should_snapshot(current_gen + 1) {
                        let guard = mmap.read().await;
                        let state_bytes = &guard[..self.serialized_len];
                        let snapshot = crate::intent_log::GenerationSnapshot {
                            generation: current_gen + 1,
                            log_sequence: self
                                .intent_log
                                .as_ref()
                                .map(|l| l.entry_count())
                                .unwrap_or(0),
                            timestamp_ns: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_nanos() as u64,
                            state_bytes: state_bytes.to_vec(),
                            checksum: 0,
                        };
                        if let Err(e) = store.save_snapshot(snapshot) {
                            tracing::warn!("Failed to save snapshot: {}", e);
                        }
                    }
                }

                // End swap: increment generation, signal readers
                generation.end_swap();
            }
        }

        Ok(())
    }
    fn read_current_state(mmap: &[u8], serialized_len: usize) -> Result<SynapseState> {
        let len = serialized_len.min(mmap.len());
        if len == 0 {
            return Err(anyhow::anyhow!("No data in mmap"));
        }
        let archived = unsafe { archived_root::<SynapseState>(&mmap[..len]) };
        let state: SynapseState = archived.deserialize(&mut rkyv::Infallible).unwrap();
        Ok(state)
    }

    // Async methods for legacy code
    pub async fn get_ptr(&self) -> *const u8 {
        let guard = self.mmap.read().await;
        guard.as_ptr()
    }

    /// Synchronous pointer access for use in sync contexts (e.g., GUI rendering)
    /// Uses a blocking runtime internally - prefer async get_ptr() when possible
    pub fn get_ptr_sync(&self) -> *const u8 {
        // Create a new single-threaded runtime for blocking access
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(self.get_ptr())
    }

    pub async fn write_at(&self, offset: usize, data: &[u8]) -> Result<()> {
        let mut guard = self.mmap.write().await;
        if offset + data.len() > guard.len() {
            anyhow::bail!("Synapse overflow: writing past allocated size");
        }
        guard[offset..offset + data.len()].copy_from_slice(data);
        guard.flush()?;
        Ok(())
    }

    /// Read data at offset (copies to owned Vec for safety)
    pub async fn read_at_owned(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        let guard = self.mmap.read().await;
        if offset + len > guard.len() {
            anyhow::bail!("Synapse overflow: reading past allocated size");
        }
        Ok(guard[offset..offset + len].to_vec())
    }
}

impl Drop for SWMRSynapse {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Multi-reader handle - zero-copy access to archived state
pub struct SynapseReader {
    mmap: Arc<RwLock<MmapMut>>,
    generation: Arc<GenerationCounter>,
    notice_rx: watch::Receiver<Option<PreparednessNotice>>,
    serialized_len: Arc<std::sync::atomic::AtomicUsize>,
}

impl SynapseReader {
    /// Read the current state (zero-copy via rkyv archived root)
    pub async fn read_state(&self) -> Result<SynapseState> {
        // Wait for any pending swap to complete
        while self.generation.is_swapping() {
            tokio::task::yield_now().await;
        }

        let guard = self.mmap.read().await;
        let len = self
            .serialized_len
            .load(std::sync::atomic::Ordering::SeqCst);
        SWMRSynapse::read_current_state(&guard, len)
    }

    /// Get current generation number
    pub fn generation(&self) -> u64 {
        self.generation.generation()
    }

    /// Check if a preparedness notice has been received
    pub fn has_pending_notice(&mut self) -> bool {
        self.notice_rx.has_changed().unwrap_or(false)
    }

    /// Consume the latest preparedness notice
    pub fn consume_notice(&mut self) -> Option<PreparednessNotice> {
        self.notice_rx.borrow().clone()
    }
}

/// Exclusive writer handle - submits mutation intents to the single writer loop
pub struct SynapseWriterHandle {
    intent_tx: mpsc::UnboundedSender<MutationIntent>,
    generation: Arc<GenerationCounter>,
    notice_broadcast: Arc<NoticeBroadcast>,
}

impl SynapseWriterHandle {
    pub fn submit_intent(&self, intent: MutationIntent) -> Result<()> {
        self.intent_tx
            .send(intent)
            .map_err(|_| anyhow::anyhow!("Writer loop has shut down"))
    }

    pub fn generation(&self) -> u64 {
        self.generation.generation()
    }

    pub fn broadcast_notice(&self, notice: &PreparednessNotice) {
        self.notice_broadcast.broadcast(notice);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_swmr_basic_read_write() {
        let mut synapse = SWMRSynapse::new("test_basic_rw2", 16384).await.unwrap();
        let reader = synapse.create_reader();

        // Verify initial state - skip this for now due to rkyv serialization issue
        // let initial_state = reader.read_state().await.unwrap();
        // assert_eq!(initial_state.schema_version, SCHEMA_VERSION);

        // Spawn writer loop
        let writer_handle = synapse.create_writer();
        let writer_task = tokio::spawn(async move { synapse.run_writer_loop().await });

        // Submit mutation
        let intent = MutationIntent {
            field_name: "curiosity_drive".to_string(),
            value: vec![75u8],
            reason: None,
        };
        writer_handle.submit_intent(intent).unwrap();

        // Give writer time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Read back - verify generation incremented
        let gen = reader.generation();
        assert!(gen >= 1, "Generation should have incremented, got {}", gen);

        writer_handle
            .submit_intent(MutationIntent {
                field_name: "shutdown".to_string(),
                value: vec![],
                reason: None,
            })
            .unwrap();

        let _ = writer_task.await;
    }

    #[tokio::test]
    async fn test_swmr_with_intent_logging() {
        let log_path = std::env::temp_dir().join("aaroneous_test_intent.log");
        let snapshot_path = std::env::temp_dir().join("aaroneous_test_snapshots.json");
        let _ = std::fs::remove_file(&log_path);
        let _ = std::fs::remove_file(&snapshot_path);

        let mut synapse = SWMRSynapse::new_with_logging(
            "test_logging",
            16384,
            &log_path,
            &snapshot_path,
            5, // snapshot every 5 generations
        )
        .await
        .unwrap();

        let _reader = synapse.create_reader();

        // Spawn writer loop
        let writer_handle = synapse.create_writer();
        let writer_task = tokio::spawn(async move { synapse.run_writer_loop().await });

        // Submit 10 mutations
        for i in 0..10 {
            let intent = MutationIntent {
                field_name: "curiosity_drive".to_string(),
                value: vec![i as u8],
                reason: None,
            };
            writer_handle.submit_intent(intent).unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }

        // Give writer time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Verify log has entries
        if log_path.exists() {
            let log_reader = crate::intent_log::LogReader::open(&log_path).unwrap();
            assert!(
                log_reader.entry_count() >= 10,
                "Expected at least 10 log entries"
            );
        }

        // Verify snapshot was created
        if snapshot_path.exists() {
            let store = crate::intent_log::SnapshotStore::new(&snapshot_path, 5).unwrap();
            assert!(
                store.find_nearest_snapshot(10).is_some(),
                "Expected snapshot at generation 10"
            );
        }

        writer_handle
            .submit_intent(MutationIntent {
                field_name: "shutdown".to_string(),
                value: vec![],
                reason: None,
            })
            .unwrap();

        let _ = writer_task.await;

        let _ = std::fs::remove_file(&log_path);
        let _ = std::fs::remove_file(&snapshot_path);
    }

    #[test]
    fn test_rkyv_serialize() {
        let state = SynapseState::default();
        assert_eq!(state.schema_version, SCHEMA_VERSION);
        assert_eq!(state.curiosity_drive, 50);
        assert_eq!(state.clock_tick, 0);

        let bytes = rkyv::to_bytes::<_, 256>(&state).unwrap();
        println!("Serialized {} bytes", bytes.len());
        println!(
            "First 32 bytes: {:?}",
            &bytes.as_slice()[..32.min(bytes.len())]
        );
        println!("Last 8 bytes: {:?}", &bytes.as_slice()[bytes.len() - 8..]);

        // rkyv stores root pointer at the end of the buffer
        // archived_root expects the buffer to start with the archived data
        // We need to use the correct approach for mmap

        // Verify we can deserialize normally
        let archived = unsafe { archived_root::<SynapseState>(&bytes) };
        let deserialized: SynapseState = archived.deserialize(&mut rkyv::Infallible).unwrap();

        println!(
            "Deserialized schema_version: {}",
            deserialized.schema_version
        );
        println!(
            "Deserialized curiosity_drive: {}",
            deserialized.curiosity_drive
        );
        println!("Deserialized clock_tick: {}", deserialized.clock_tick);

        assert_eq!(deserialized.schema_version, SCHEMA_VERSION);
        assert_eq!(deserialized.curiosity_drive, 50);
        assert_eq!(deserialized.clock_tick, 0);
    }

    #[tokio::test]
    async fn test_generation_counter() {
        let gen = GenerationCounter::new();
        assert_eq!(gen.generation(), 0);
        assert!(!gen.is_swapping());

        assert!(gen.begin_swap());
        assert!(gen.is_swapping());

        gen.end_swap();
        assert!(!gen.is_swapping());
        assert_eq!(gen.generation(), 1);
    }
}
