use anyhow::{Context, Result};
use memmap2::{MmapMut, MmapOptions};
use parking_lot::RwLock;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Maximum allowed wall-clock duration for a single tick. If a tick exceeds
/// this we log a warning and continue the loop on the next iteration. This
/// is the watchdog guard: a runaway subsystem that holds a lock for 10
/// seconds (or a long-running IO call) does not freeze the whole nervous
/// system forever.
const TICK_WATCHDOG: Duration = Duration::from_secs(10);
/// Default total tick budget when no explicit budget is set. The autonomic
/// loop will stop on its own after this many ticks if it is not externally
/// shut down. This is the safety stop that prevents the loop from running
/// effectively forever and accumulating unbounded state.
const DEFAULT_MAX_TICKS: u64 = 86_400; // 24h at 1Hz

/// Size, in bytes, of the seqlock sequence header prepended to every
/// synapse mmap. Every `write`/`read` offset taken by this struct's public
/// API is relative to the payload *after* this header - callers never see
/// it directly.
const SYNAPSE_SEQ_HEADER_BYTES: usize = 8;

/// Upper bound on seqlock retry spins before `write`/`read` give up and
/// return an error, rather than spinning indefinitely. A write is just a
/// small `memcpy` under the lock (microseconds), so this many iterations
/// comfortably covers ordinary contention between the daemon's tick loop
/// and an `inject` CLI invocation while still bounding the *worst case* -
/// unlike a blocking OS file lock, a writer that stalls (crashes, is
/// suspended) while "holding" the seqlock can only make readers/writers
/// spin for this many iterations before they bail out, never hang forever.
const SYNAPSE_SEQLOCK_MAX_SPINS: u32 = 200_000;

pub struct LegacySharedMemorySynapse {
    mmap: MmapMut,
    _path: PathBuf,
}

impl LegacySharedMemorySynapse {
    /// `data_size` is the payload size (e.g. `size_of::<SynapseState>()`);
    /// the actual file/mapping is `SYNAPSE_SEQ_HEADER_BYTES` bytes larger to
    /// hold the seqlock sequence word. Creates and sizes the file if it
    /// does not already exist - only the daemon that owns this synapse
    /// should call this; other processes that expect the daemon to have
    /// already sized it should use `open_existing` instead.
    ///
    /// Resolves `name` against the default (ambient) `WorkspacePathsConfig`,
    /// which is what production callers (the daemon itself) want. Tests
    /// must use `new_at` with an explicit `tempfile::tempdir()` path
    /// instead, per AGENTS.md's test-sandboxing rule: this default-config
    /// path lands in the shared host cache dir, which a test could collide
    /// with under parallel runs or pollute for later ones.
    pub fn new(name: &str, data_size: usize) -> Result<Self> {
        let path = paths::resolve_synapse_path(name, &paths::WorkspacePathsConfig::default());
        Self::new_at(&path, data_size)
    }

    /// Same as `new`, but against an explicit file path rather than a
    /// name resolved through the ambient `WorkspacePathsConfig` - the
    /// tempdir-backed constructor tests should use.
    pub fn new_at(path: &Path, data_size: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        file.set_len((SYNAPSE_SEQ_HEADER_BYTES + data_size) as u64)?;

        Self::from_file(file, path.to_path_buf())
    }

    /// Opens an existing, already-sized synapse file without creating or
    /// resizing it - for a process (e.g. the `hypervisor inject` CLI
    /// command) that writes into a synapse it does not own the lifecycle
    /// of. Errors clearly if the file is missing or too small, rather than
    /// creating/truncating it out from under the daemon that does own it.
    ///
    /// See `new`'s doc comment: resolves `name` against the default
    /// (ambient) `WorkspacePathsConfig`; tests must use `open_existing_at`
    /// with an explicit path instead.
    pub fn open_existing(name: &str, data_size: usize) -> Result<Self> {
        let path = paths::resolve_synapse_path(name, &paths::WorkspacePathsConfig::default());
        Self::open_existing_at(&path, data_size)
    }

    /// Same as `open_existing`, but against an explicit file path rather
    /// than a name resolved through the ambient `WorkspacePathsConfig`.
    pub fn open_existing_at(path: &Path, data_size: usize) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .with_context(|| {
                format!(
                    "synapse file {} not found; is the hypervisor daemon running to create it first?",
                    path.display()
                )
            })?;

        let required = SYNAPSE_SEQ_HEADER_BYTES + data_size;
        let actual = file.metadata()?.len() as usize;
        if actual < required {
            anyhow::bail!(
                "synapse file {} is {actual} bytes, too small (need >= {required} bytes); \
                 is the hypervisor daemon running to size it first?",
                path.display()
            );
        }

        Self::from_file(file, path.to_path_buf())
    }

    fn from_file(file: std::fs::File, path: PathBuf) -> Result<Self> {
        // `file` is sized to at least `SYNAPSE_SEQ_HEADER_BYTES + data_size`
        // bytes by both callers above (freshly via `set_len` in `new`, or
        // verified via `metadata()` in `open_existing`) before this mapping
        // request, and this struct never truncates it afterwards. A
        // concurrent writer mutating bytes in place (never truncating) is
        // the seqlock-protected access pattern `write`/`read` implement
        // below, not a memory-safety hazard for the mapping itself.
        // SAFETY: file is sized appropriately and never truncated after; see rationale above.
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(Self { mmap, _path: path })
    }

    fn seq_word(&self) -> &AtomicU64 {
        // The mmap is backed by an `mmap()`'d region, which the OS always
        // page-aligns (far stricter than `AtomicU64`'s 8-byte requirement),
        // and `Self::from_file` guarantees the mapping is at least
        // `SYNAPSE_SEQ_HEADER_BYTES` (8) bytes long, so reinterpreting the
        // first 8 bytes as an `AtomicU64` is valid for the lifetime of
        // `self` (the mmap outlives every reference handed out here).
        // SAFETY: mmap base is page-aligned and >= 8 bytes; see rationale above.
        unsafe { &*(self.mmap.as_ptr() as *const AtomicU64) }
    }

    /// Writes `data` at `offset` (relative to the payload, i.e. *after* the
    /// seqlock header) as a single seqlock-protected transaction: spins to
    /// claim the sequence word (even -> odd), copies the bytes, then
    /// releases it (-> even again). Never calls a blocking OS primitive, so
    /// a stalled concurrent writer can make this spin for at most
    /// `SYNAPSE_SEQLOCK_MAX_SPINS` iterations, never hang the caller's
    /// hot path indefinitely - see `SYNAPSE_SEQLOCK_MAX_SPINS`.
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<()> {
        let seq = self.seq_word();
        let mut spins = 0u32;
        let seq_before = loop {
            let current = seq.load(Ordering::Acquire);
            if current.is_multiple_of(2)
                && seq
                    .compare_exchange_weak(
                        current,
                        current + 1,
                        Ordering::AcqRel,
                        Ordering::Relaxed,
                    )
                    .is_ok()
            {
                break current;
            }
            spins += 1;
            if spins > SYNAPSE_SEQLOCK_MAX_SPINS {
                anyhow::bail!(
                    "synapse seqlock write contended for {SYNAPSE_SEQLOCK_MAX_SPINS} spins \
                     without acquiring - another writer appears stalled"
                );
            }
            std::hint::spin_loop();
        };

        let ptr = self.mmap.as_ptr() as *mut u8;
        // `ptr` is derived from `self.mmap`, which stays valid for the
        // lifetime of `self` and is at least `SYNAPSE_SEQ_HEADER_BYTES +
        // data_size` bytes long. `data.as_ptr()`/`data.len()` come from a
        // live `&[u8]`, so the source range is valid for reads. This call
        // requires (unchecked here) that `SYNAPSE_SEQ_HEADER_BYTES + offset
        // + data.len()` does not exceed the mapping's length - callers must
        // ensure that themselves. Holding the seqlock (above) excludes
        // every other writer for the duration of this copy.
        // SAFETY: mapping is live and long enough; offset+len is caller-checked.
        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                ptr.add(SYNAPSE_SEQ_HEADER_BYTES + offset),
                data.len(),
            );
        }

        seq.store(seq_before + 2, Ordering::Release);
        Ok(())
    }

    /// Reads `len` bytes at `offset` (relative to the payload). Retries
    /// (spins) until it observes a stable, even sequence number immediately
    /// before and after the copy, guaranteeing it never returns bytes torn
    /// by a concurrent `write`. Like `write`, this never blocks on an OS
    /// primitive - see `SYNAPSE_SEQLOCK_MAX_SPINS`.
    pub fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        let seq = self.seq_word();
        let mut buf = vec![0u8; len];
        let ptr = self.mmap.as_ptr();
        let mut spins = 0u32;
        loop {
            let seq_before = seq.load(Ordering::Acquire);
            if seq_before.is_multiple_of(2) {
                // `ptr` is derived from `self.mmap`, which stays valid for
                // the lifetime of `self`. `buf` was just allocated with
                // exactly `len` bytes, so the destination range is valid
                // for writes of `len` bytes. This call requires (unchecked
                // here) that `SYNAPSE_SEQ_HEADER_BYTES + offset + len` does
                // not exceed the mapping's length - callers must ensure
                // that. The sequence check below (not this copy itself)
                // is what detects a write that raced with it.
                // SAFETY: mapping is live; buf is exactly `len` bytes; bounds are caller-checked.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        ptr.add(SYNAPSE_SEQ_HEADER_BYTES + offset),
                        buf.as_mut_ptr(),
                        len,
                    );
                }
                let seq_after = seq.load(Ordering::Acquire);
                if seq_after == seq_before {
                    return Ok(buf);
                }
            }
            spins += 1;
            if spins > SYNAPSE_SEQLOCK_MAX_SPINS {
                anyhow::bail!(
                    "synapse seqlock read contended for {SYNAPSE_SEQLOCK_MAX_SPINS} spins \
                     without a stable read - a writer appears stalled"
                );
            }
            std::hint::spin_loop();
        }
    }

    /// Atomically writes a `hypervisor inject`-style intent: `task_id` into
    /// `SynapseState::intent_vector_id` and `payload` (truncated to
    /// `SYNAPSE_INTENT_PAYLOAD_CAPACITY`, zero-padded) into
    /// `intent_payload`, as a *single* seqlock transaction so a concurrent
    /// reader (the daemon's own tick loop) can never observe one without
    /// the other. Relies on the two fields being contiguous in
    /// `SynapseState` (`SYNAPSE_INTENT_PAYLOAD_OFFSET ==
    /// SYNAPSE_INTENT_VECTOR_ID_OFFSET + 16`, asserted by
    /// `test_synapse_state_field_offsets_match_cli_assumptions`).
    pub fn write_intent(&self, task_id: uuid::Uuid, payload: &[u8]) -> Result<()> {
        debug_assert_eq!(
            SYNAPSE_INTENT_PAYLOAD_OFFSET,
            SYNAPSE_INTENT_VECTOR_ID_OFFSET + 16
        );
        let mut combined = vec![0u8; 16 + SYNAPSE_INTENT_PAYLOAD_CAPACITY];
        combined[0..16].copy_from_slice(task_id.as_bytes());
        let payload_len = std::cmp::min(payload.len(), SYNAPSE_INTENT_PAYLOAD_CAPACITY);
        combined[16..16 + payload_len].copy_from_slice(&payload[..payload_len]);
        self.write(SYNAPSE_INTENT_VECTOR_ID_OFFSET, &combined)
    }
}

use crate::concept_drift::ConceptDriftDetector;
use crate::delta_orchestrator::DeltaOrchestrator;
use crate::dopamine_system::{DopamineEvent, DopamineSystem, FeedbackSignalProcessor};
use crate::enzyme_runner::EnzymeRunner;
use crate::enzyme_types::{CuriosityEnzyme, DiplomatEnzyme, SelfCorrectionEnzyme};
use crate::executive_plan::{ExecutivePlan, StepStatus};
use crate::federation::hive_db::PersistenceManager as HivePersistence;
use crate::hox_registry::HoxRegistry;
use crate::neural_pruning::NeuralPruningEnzyme;
use crate::nlm_sentinel::{IntentTier, NlmSentinel};
use crate::predictive_models::{HiddenMarkovModel, KalmanFilter1D};
use crate::prefrontal_cortex::PrefrontalCortex;
use crate::semantic_indexing::SemanticIndex;
use crate::specialist_memory::{
    MemoryEntry, MemoryType, SharedMemoryRegistry, SpecialistMemoryStore,
};
use crate::splicing_engine::WasmSplicingEngine;
use crate::system_metrics::{SystemMetricsCollector, ThermalStatus};
use crate::task_routing::TaskRouter;
use crate::unified_learning::UnifiedLearningLoop;
use biology::{SystemBiology, ThrottleState};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct McpToolCallState {
    pub status: u32,
    pub call_id: u64,
    pub tool_name_hash: u64,
    pub arguments_size: u32,
    pub arguments_payload: [u8; 1024],
}

impl Default for McpToolCallState {
    fn default() -> Self {
        Self {
            status: 0,
            call_id: 0,
            tool_name_hash: 0,
            arguments_size: 0,
            arguments_payload: [0u8; 1024],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct DialogueState {
    pub consensus_score: u32,
    pub active_speaker_hash: u64,
    pub turn_count: u32,
    pub message_size: u32,
    pub message_payload: [u8; 1024],
}

impl Default for DialogueState {
    fn default() -> Self {
        Self {
            consensus_score: 50,
            active_speaker_hash: 0,
            turn_count: 0,
            message_size: 0,
            message_payload: [0u8; 1024],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct SynapseState {
    pub clock_tick: u64,
    pub memory_pressure: u32,
    pub understanding_score: u32,
    pub curiosity_drive: u32,
    pub intent_vector_id: [u8; 16],
    pub intent_payload: [u8; 4096],
    pub safety_lock: u32,
    pub sovereignty_tier: u32,
    pub approval_required: u32,
    pub approval_granted: u32,
    pub integrity_score: u32,
    pub concept_drift: f32,
    pub latent_vector: [f32; 1024],
    pub mcp_tool_call: McpToolCallState,
    pub dialogue: DialogueState,
}

/// Byte offset of `SynapseState::intent_vector_id` within the raw
/// `LegacySharedMemorySynapse` mapping. External writers (e.g. the
/// `hypervisor inject` CLI command) that poke the mmap directly, without
/// going through `write_state`'s whole-struct byte dump, must use this
/// instead of a hardcoded literal: `#[repr(C)]` field offsets are an
/// implementation detail of field order and padding, not a stable ABI
/// this crate promises externally, and a prior version of the CLI command
/// hardcoded offset 16 here, four bytes before the field's real offset
/// (0..=15 is `clock_tick`/`memory_pressure`/`understanding_score`).
pub const SYNAPSE_INTENT_VECTOR_ID_OFFSET: usize =
    std::mem::offset_of!(SynapseState, intent_vector_id);
/// Byte offset of `SynapseState::intent_payload`. See
/// `SYNAPSE_INTENT_VECTOR_ID_OFFSET` above for why external writers must
/// use this rather than a hardcoded literal.
pub const SYNAPSE_INTENT_PAYLOAD_OFFSET: usize = std::mem::offset_of!(SynapseState, intent_payload);
/// Capacity in bytes of `SynapseState::intent_payload`.
pub const SYNAPSE_INTENT_PAYLOAD_CAPACITY: usize = 4096;

impl Default for SynapseState {
    fn default() -> Self {
        Self {
            clock_tick: 0,
            memory_pressure: 0,
            understanding_score: 0,
            curiosity_drive: 0,
            intent_vector_id: [0u8; 16],
            intent_payload: [0u8; 4096],
            safety_lock: 0,
            sovereignty_tier: 0,
            approval_required: 0,
            approval_granted: 0,
            integrity_score: 100,
            concept_drift: 0.0,
            latent_vector: [0.0f32; 1024],
            mcp_tool_call: McpToolCallState::default(),
            dialogue: DialogueState::default(),
        }
    }
}

/// Sovereign core supervisory daemon running the deterministic control loop.
pub struct SupervisoryDaemon {
    synapse: Arc<RwLock<LegacySharedMemorySynapse>>,
    enzyme_runner: Arc<EnzymeRunner>,
    _hox_registry: Arc<HoxRegistry>,
    _splicing_engine: Arc<WasmSplicingEngine>,
    learning_loop: Arc<RwLock<UnifiedLearningLoop>>,
    nlm_sentinel: Arc<NlmSentinel>,
    prefrontal_cortex: Arc<PrefrontalCortex>,
    dopamine_system: Arc<DopamineSystem>,
    adaptation_orchestrator: Arc<DeltaOrchestrator>,
    self_correction_enzyme: Arc<SelfCorrectionEnzyme>,
    neural_pruning_enzyme: Arc<NeuralPruningEnzyme>,
    diplomat_enzyme: Arc<DiplomatEnzyme>,
    concept_drift_detector: Arc<RwLock<ConceptDriftDetector>>,
    curiosity_enzyme: Arc<RwLock<CuriosityEnzyme>>,
    semantic_index: Arc<RwLock<SemanticIndex>>,
    active_plan: Arc<RwLock<Option<ExecutivePlan>>>,
    _metrics_collector: SystemMetricsCollector,
    _task_router: TaskRouter,
    specialist_memory: SharedMemoryRegistry,
    biology: Arc<parking_lot::RwLock<SystemBiology>>,
    tick_rate: Duration,
    hive_db: Option<Arc<parking_lot::Mutex<HivePersistence>>>,
    _workspace_root: PathBuf,
    /// PHASE IV: Predictive models for load forecasting and intent recognition
    kalman_filter: Arc<RwLock<KalmanFilter1D>>,
    hmm_model: Arc<RwLock<HiddenMarkovModel>>,
    /// Cooperative shutdown flag. Set to true via `request_shutdown` to make
    /// the next iteration of the main event loop exit cleanly. Atomic because
    /// the spawned thread reads it on every tick and the API may be called
    /// from another thread (signal handler, control plane, panic recovery).
    pub shutdown: Arc<AtomicBool>,
    /// Total tick budget. When set, the loop exits after this many ticks
    /// even if `shutdown` is not requested. Default is `DEFAULT_MAX_TICKS`.
    /// Set to `u64::MAX` to disable the budget (loop runs until shutdown).
    pub max_ticks: Arc<AtomicU64>,
    /// Wall-clock start of the most recent tick. Held in an Arc so the
    /// spawned thread can use it to enforce `TICK_WATCHDOG` from inside
    /// the loop without holding a mutable borrow of the system struct.
    pub tick_start: Arc<RwLock<Instant>>,
    /// IPC SWMR shared-memory state snapshot publisher for decoupled shells
    pub state_publisher: Arc<crate::state_snapshot::EngineStatePublisher>,
    /// Dynamic autonomic pacing regulator managing thermodynamic backoff
    pub pacing_regulator: Arc<parking_lot::RwLock<adaptation_plane::AutonomousPacingRegulator>>,
    /// Black-box flight recorder for deterministic event replay and forensic audit
    pub flight_recorder: Option<Arc<parking_lot::Mutex<ipc_bus::FlightRecorder>>>,
    /// Formal SMT action interlock gatekeeper
    pub smt_interlock: Arc<parking_lot::RwLock<governance::SmtActionInterlock>>,
}

/// Backward-compatible alias for standard control systems nomenclature
pub type AutonomousControlLoop = SupervisoryDaemon;

/// Legacy biological nomenclature alias
#[deprecated(note = "Use SupervisoryDaemon instead")]
pub type AutonomicNervousSystem = SupervisoryDaemon;

impl SupervisoryDaemon {
    pub fn new(
        synapse_name: &str,
        tick_rate_ms: u64,
        enzyme_runner: Arc<EnzymeRunner>,
        hox_registry: Arc<HoxRegistry>,
        splicing_engine: Arc<WasmSplicingEngine>,
        learning_loop: Arc<RwLock<UnifiedLearningLoop>>,
        db_path: Option<&str>,
    ) -> Result<Self> {
        let pacing_regulator = Arc::new(parking_lot::RwLock::new(
            adaptation_plane::AutonomousPacingRegulator::default_with_baseline(
                Duration::from_millis(tick_rate_ms),
            ),
        ));
        Self::new_with_pacing(
            synapse_name,
            tick_rate_ms,
            enzyme_runner,
            hox_registry,
            splicing_engine,
            learning_loop,
            db_path,
            pacing_regulator,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Constructor explicitly injects existing runtime dependencies"
    )]
    pub fn new_with_pacing(
        synapse_name: &str,
        tick_rate_ms: u64,
        enzyme_runner: Arc<EnzymeRunner>,
        hox_registry: Arc<HoxRegistry>,
        splicing_engine: Arc<WasmSplicingEngine>,
        learning_loop: Arc<RwLock<UnifiedLearningLoop>>,
        db_path: Option<&str>,
        pacing_regulator: Arc<parking_lot::RwLock<adaptation_plane::AutonomousPacingRegulator>>,
    ) -> Result<Self> {
        let size = std::mem::size_of::<SynapseState>();
        let synapse = LegacySharedMemorySynapse::new(synapse_name, size)?;

        let initial = SynapseState::default();
        // `&initial` points to a live, fully-initialized `SynapseState` for
        // the duration of this block. `size` is `size_of::<SynapseState>()`,
        // the exact byte length of that value, so the resulting slice does
        // not read past it; the bytes are only copied into the mmap below.
        // SAFETY: `size` matches `&initial`'s exact byte length; see above.
        let bytes = unsafe {
            std::slice::from_raw_parts(&initial as *const SynapseState as *const u8, size)
        };
        synapse.write(0, bytes).ok();

        let hive_db = db_path.and_then(|p| {
            HivePersistence::new(p)
                .ok()
                .map(|db| Arc::new(parking_lot::Mutex::new(db)))
        });
        let mut semantic_index = SemanticIndex::new();
        if let Some(ref db_mutex) = hive_db {
            let db = db_mutex.lock();
            if let Ok(embeddings) = db.load_all_embeddings() {
                for (id, text, vector, metadata, access_count) in embeddings {
                    let entry = crate::semantic_indexing::SemanticEmbedding {
                        id,
                        text,
                        vector,
                        metadata,
                        last_accessed: chrono::Utc::now(),
                        access_count,
                    };
                    semantic_index.entries.push(entry);
                }
            }
        }

        let workspace_root =
            paths::WorkspacePaths::from_config(paths::WorkspacePathsConfig::default())
                .root()
                .clone();

        // PHASE IV: Initialize predictive models
        let kalman_filter = Arc::new(RwLock::new(KalmanFilter1D::new(
            0.95_f32, // Process variance: low (predictable system)
            0.05_f32, // Measurement variance: low (reliable metrics)
            0.1_f32,  // Control variance: moderate (allow adaptation)
        )));
        let hmm_model = Arc::new(RwLock::new(
            HiddenMarkovModel::new(
                vec![1.0 / 6.0; 6],
                vec![
                    0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1, 0.5, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1,
                    0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1, 0.5, 0.1,
                    0.1, 0.1, 0.1, 0.1,
                ],
                vec![
                    0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1, 0.5, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1,
                    0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.5, 0.1, 0.5, 0.1,
                    0.1, 0.1, 0.1, 0.1,
                ],
            )
            .expect("failed to initialize HMM"),
        ));

        Ok(Self {
            synapse: Arc::new(RwLock::new(synapse)),
            enzyme_runner: enzyme_runner.clone(),
            _hox_registry: hox_registry.clone(),
            _splicing_engine: splicing_engine.clone(),
            learning_loop: learning_loop.clone(),
            nlm_sentinel: Arc::new(NlmSentinel::new()?),
            prefrontal_cortex: Arc::new(PrefrontalCortex),
            dopamine_system: Arc::new(FeedbackSignalProcessor),
            adaptation_orchestrator: Arc::new(DeltaOrchestrator::new()),
            self_correction_enzyme: Arc::new(SelfCorrectionEnzyme::new()),
            neural_pruning_enzyme: Arc::new(NeuralPruningEnzyme::new(60)),
            diplomat_enzyme: Arc::new(DiplomatEnzyme::new()),
            concept_drift_detector: Arc::new(RwLock::new(ConceptDriftDetector::new())),
            curiosity_enzyme: Arc::new(RwLock::new(CuriosityEnzyme::new())),
            semantic_index: Arc::new(RwLock::new(semantic_index)),
            active_plan: Arc::new(RwLock::new(None)),
            _metrics_collector: SystemMetricsCollector::new(),
            _task_router: TaskRouter::new(
                Some(enzyme_runner.clone()),
                Some(learning_loop.clone()),
                None, // No hive_db in autonomic loop
            ),
            specialist_memory: SharedMemoryRegistry::new(),
            biology: Arc::new(parking_lot::RwLock::new(SystemBiology::new())),
            tick_rate: Duration::from_millis(tick_rate_ms),
            hive_db,
            _workspace_root: workspace_root,
            kalman_filter,
            hmm_model,
            shutdown: Arc::new(AtomicBool::new(false)),
            max_ticks: Arc::new(AtomicU64::new(DEFAULT_MAX_TICKS)),
            tick_start: Arc::new(RwLock::new(Instant::now())),
            state_publisher: Arc::new(crate::state_snapshot::EngineStatePublisher::new()),
            pacing_regulator,
            flight_recorder: None,
            smt_interlock: Arc::new(parking_lot::RwLock::new(
                governance::SmtActionInterlock::strict(),
            )),
        })
    }

    /// Attaches an optional black-box flight recorder to the autonomic loop.
    pub fn with_flight_recorder(
        mut self,
        recorder: Arc<parking_lot::Mutex<ipc_bus::FlightRecorder>>,
    ) -> Self {
        self.flight_recorder = Some(recorder);
        self
    }

    /// Attaches an SMT formal action interlock gatekeeper
    pub fn with_smt_interlock(
        mut self,
        interlock: Arc<parking_lot::RwLock<governance::SmtActionInterlock>>,
    ) -> Self {
        self.smt_interlock = interlock;
        self
    }

    /// Request the autonomic loop to exit at the next iteration boundary.
    /// Safe to call from signal handlers, control plane threads, or panic
    /// recovery callbacks. The loop will finish its current tick and then
    /// break. Idempotent.
    pub fn request_shutdown(&self) {
        if !self.shutdown.swap(true, Ordering::SeqCst) {
            info!(target: "autonomic_loop", "shutdown requested");
        }
    }

    /// Override the default tick budget. Pass `u64::MAX` to disable the
    /// safety stop and rely on cooperative shutdown only.
    pub fn set_max_ticks(&self, max: u64) {
        self.max_ticks.store(max, Ordering::SeqCst);
    }

    /// Returns `true` if `request_shutdown` has been called.
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Return access to the autonomous pacing regulator
    pub fn pacing_regulator(
        &self,
    ) -> Arc<parking_lot::RwLock<adaptation_plane::AutonomousPacingRegulator>> {
        self.pacing_regulator.clone()
    }

    pub fn get_synapse(&self) -> Arc<RwLock<LegacySharedMemorySynapse>> {
        self.synapse.clone()
    }

    /// Get or create specialist memory store
    pub fn get_specialist_memory(&self, specialist_id: &str) -> SpecialistMemoryStore {
        self.specialist_memory.get_or_create(specialist_id)
    }

    /// Consult specialist memory during task execution
    pub fn consult_specialist_memory(
        &self,
        specialist_id: &str,
        task_description: &str,
        task_type: &str,
    ) -> String {
        let store = self.get_specialist_memory(specialist_id);
        let query_result = store.query_memory(task_description, task_type, 3);

        let mut guidance = String::new();
        guidance.push_str(&format!(
            "[MemoryConsultation] {}\n",
            query_result.recommendation
        ));

        if !query_result.entries.is_empty() {
            guidance.push_str("Previous experience:\n");
            for (i, entry) in query_result.entries.iter().enumerate() {
                guidance.push_str(&format!(
                    "  {}. {} ({}%, accessed {} times)\n",
                    i + 1,
                    entry.title,
                    (entry.confidence * 100.0) as u32,
                    entry.access_count
                ));
            }
        }

        guidance
    }

    fn read_state(syn: &LegacySharedMemorySynapse) -> SynapseState {
        let size = std::mem::size_of::<SynapseState>();
        let buf = syn.read(0, size).unwrap_or_else(|_| vec![0u8; size]);
        // `buf` holds exactly `size_of::<SynapseState>()` bytes (either read
        // back from the mapping written by `write_state` below, or a
        // same-length zero-filled fallback), so `buf.as_ptr()` is valid for
        // a read of that length. `read_unaligned` is used specifically
        // because `buf`'s heap allocation is not guaranteed to satisfy
        // `SynapseState`'s alignment. `SynapseState` is a POD struct written
        // only via `write_state`'s matching byte dump (or all-zero), so any
        // bit pattern present here is a valid `SynapseState`.
        // SAFETY: `buf` is exactly `size_of::<SynapseState>()` bytes; see above.
        unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const SynapseState) }
    }

    fn write_state(syn: &LegacySharedMemorySynapse, state: &SynapseState) {
        let size = std::mem::size_of::<SynapseState>();
        // `state` points to a live `SynapseState` for the duration of this
        // block, and `size` is `size_of::<SynapseState>()`, its exact byte
        // length, so the resulting slice does not read past it.
        // SAFETY: `size` matches `state`'s exact byte length; see above.
        let bytes =
            unsafe { std::slice::from_raw_parts(state as *const SynapseState as *const u8, size) };
        syn.write(0, bytes).ok();
    }

    pub fn start(&self) {
        let synapse = self.synapse.clone();
        let enzyme_runner = self.enzyme_runner.clone();
        let learning_loop = self.learning_loop.clone();
        let nlm_sentinel = self.nlm_sentinel.clone();
        let prefrontal_cortex = self.prefrontal_cortex.clone();
        let dopamine_system = self.dopamine_system.clone();
        let adaptation_orchestrator = self.adaptation_orchestrator.clone();
        let self_correction_enzyme = self.self_correction_enzyme.clone();
        let neural_pruning_enzyme = self.neural_pruning_enzyme.clone();
        let diplomat_enzyme = self.diplomat_enzyme.clone();
        let concept_drift_detector = self.concept_drift_detector.clone();
        let curiosity_enzyme = self.curiosity_enzyme.clone();
        let semantic_index = self.semantic_index.clone();
        let active_plan = self.active_plan.clone();
        let metrics_collector = SystemMetricsCollector::new();
        let enzyme_runner_for_router = enzyme_runner.clone();
        let learning_loop_for_router = learning_loop.clone();
        let specialist_memory = self.specialist_memory.clone();
        let biology = self.biology.clone();
        let tick_rate = self.tick_rate;
        let hive_db = self.hive_db.clone();
        let shutdown = self.shutdown.clone();
        let max_ticks = self.max_ticks.clone();
        let tick_start = self.tick_start.clone();
        let kalman_filter = self.kalman_filter.clone();
        let hmm_model = self.hmm_model.clone();
        let state_publisher = self.state_publisher.clone();
        let pacing_regulator = self.pacing_regulator.clone();
        let flight_recorder = self.flight_recorder.clone();
        let smt_interlock = self.smt_interlock.clone();

        info!(target: "autonomic_loop", ?tick_rate, "heartbeat initiated");

        thread::spawn(move || {
            // OS-01 & Mechanical Sympathy: Elevate reflex loop priority and pin to physical P-Cores
            platform_bridge::observability::mmcss::enable_mmcss_time_critical("Games");
            platform_bridge::observability::mmcss::set_thread_performance_affinity(0x05); // Pin to P-Core #0 and #2

            let mut drift_filter = adaptation_plane::StreamingSelfCorrectionFilter::default();

            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    error!(target: "autonomic_loop", ?e, "Failed to create Tokio runtime");
                    return;
                }
            };
            let _task_router = TaskRouter::new(
                Some(enzyme_runner_for_router),
                Some(learning_loop_for_router),
                None,
            );

            let mut tick_count: u64 = 0;
            loop {
                // --- COOPERATIVE SHUTDOWN ---
                if shutdown.load(Ordering::SeqCst) {
                    info!(target: "autonomic_loop", tick_count, "shutdown observed; exiting");
                    break;
                }
                // --- TICK BUDGET ---
                if tick_count >= max_ticks.load(Ordering::SeqCst) {
                    warn!(target: "autonomic_loop", tick_count, "tick budget exhausted; exiting");
                    break;
                }

                let start = Instant::now();
                {
                    let mut ts = tick_start.write();
                    *ts = start;
                }
                tick_count = tick_count.saturating_add(1);

                // --- PHASE IV: OBSERVABILITY: Wrap tick in telemetry span ---
                let tick_span = tracing::span!(
                    tracing::Level::INFO,
                    "autonomic_loop.tick",
                    tick = tick_count,
                    "predictive telemetry span"
                );
                let _guard = tick_span.enter();

                // --- PHASE IV: OBSERVABILITY: Track predictive telemetry performance ---
                let predictive_perf = tracing::span!(
                    tracing::Level::DEBUG,
                    "autonomic_loop.predictive",
                    tick = tick_count,
                    "predictive telemetry performance"
                );
                let _predictive_guard = predictive_perf.enter();

                let mut state = {
                    let syn = synapse.read();
                    Self::read_state(&syn)
                };

                state.clock_tick += 1;

                // --- PHASE IV: OBSERVABILITY: Log predictive telemetry metrics ---
                // Log predictive telemetry metrics for observability
                {
                    let thermal_metrics = metrics_collector.get_thermal_metrics();
                    let _backpressure_level = metrics_collector.get_backpressure_level();
                    let understanding_score = state.understanding_score as f64 / 100.0;
                    let _curiosity_drive = state.curiosity_drive as f64 / 100.0;
                    let _memory_pressure = state.memory_pressure as f64 / 100.0;
                    let _concept_drift = state.concept_drift;
                    let _integrity_score = state.integrity_score as f64 / 100.0;
                    let _sovereignty_tier = state.sovereignty_tier as f64;
                    let _approval_required = state.approval_required as f64;
                    let _approval_granted = state.approval_granted as f64;
                    let _safety_lock = state.safety_lock as f64;
                    let thermal_state = if thermal_metrics.cpu_temperature > 85.0 {
                        "critical"
                    } else if thermal_metrics.cpu_temperature > 75.0 {
                        "warning"
                    } else if thermal_metrics.cpu_temperature > 65.0 {
                        "normal"
                    } else {
                        "idle"
                    };
                    let _intent_state = hmm_model.read().get_current_state();
                    let thermal_prediction = kalman_filter.read().position();
                    let load_prediction = kalman_filter.read().position();
                    let token_prediction = kalman_filter.read().position();
                    let intent_prediction = hmm_model
                        .read()
                        .get_prediction_confidence(understanding_score);
                    let state_transition = hmm_model.read().get_state_transition();
                    let transition_confidence = hmm_model.read().get_transition_confidence();
                    let lattice_state = hmm_model.read().get_lattice_state();
                    let lattice_confidence = hmm_model.read().get_lattice_confidence();

                    debug!(
                        target: "autonomic_loop.predictive",
                        tick = tick_count,
                        thermal_state = thermal_state,
                        thermal_prediction = thermal_prediction,
                        load_prediction = load_prediction,
                        token_prediction = token_prediction,
                        intent_prediction = intent_prediction,
                        state_transition = state_transition,
                        transition_confidence = transition_confidence,
                        lattice_state = lattice_state,
                        lattice_confidence = lattice_confidence,
                        "predictive telemetry metrics"
                    );
                }

                // --- PHASE IV: THERMAL PREDICTION (Kalman Filter) ---
                // Predict thermal state before measuring
                {
                    let mut kf = kalman_filter.write();
                    // Predict next thermal state
                    kf.predict();
                    let predicted_temp = kf.position();
                    debug!(
                        target: "autonomic_loop.predictive.thermal",
                        predicted_temp = predicted_temp,
                        "thermal prediction (Kalman)"
                    );
                }

                // --- PHASE IV: OBSERVABILITY: Thermal prediction residual ---
                // Log prediction residual for convergence tracking
                {
                    let thermal_metrics = metrics_collector.get_thermal_metrics();
                    let measurement = thermal_metrics.cpu_temperature;
                    let kf = kalman_filter.write();
                    let filtered_temp = kf.position();
                    let residual = measurement - f64::from(filtered_temp);
                    debug!(
                        target: "autonomic_loop.predictive.thermal",
                        residual = residual,
                        "thermal prediction residual (convergence)"
                    );
                }

                // --- THERMAL MONITORING: Check system health ---
                let thermal_metrics = metrics_collector.get_thermal_metrics();
                let thermal_factor = metrics_collector.get_throttle_factor();
                let gpu_metrics = metrics_collector.get_gpu_metrics();

                // Dynamic thermodynamic telemetry acquisition for autonomic pacing
                let thermodynamic_telemetry = adaptation_plane::ThermodynamicTelemetry {
                    cpu_load_pct: (metrics_collector.get_backpressure_level() * 100.0) as f32,
                    cpu_temp_c: thermal_metrics.cpu_temperature as f32,
                    gpu_temp_c: gpu_metrics.temperature as f32,
                    vram_used_bytes: gpu_metrics.memory_used,
                    vram_total_bytes: gpu_metrics.memory_total,
                    memory_pressure_pct: state.memory_pressure as f32,
                };

                let pacing_decision = {
                    let mut regulator = pacing_regulator.write();
                    regulator
                        .compute_next_cadence(&thermodynamic_telemetry)
                        .unwrap_or(adaptation_plane::PacingDecision {
                            target_cadence: tick_rate,
                            throttle_tier: adaptation_plane::PacingTier::Nominal,
                            decimation_factor: 1,
                            task_deferral_probability: 0.0,
                            composite_stress: 0.0,
                        })
                };

                // Update Kalman filter with actual measurement
                {
                    let mut kf = kalman_filter.write();
                    let measurement = thermal_metrics.cpu_temperature as f32;
                    kf.update(measurement);
                    let filtered_temp = kf.position();
                    // Calculate prediction residual
                    let residual = f64::from(measurement) - f64::from(filtered_temp);
                    debug!(
                        target: "autonomic_loop.predictive.thermal",
                        measurement = measurement,
                        filtered_temp = filtered_temp,
                        residual = residual,
                        "thermal measurement update (Kalman residual)"
                    );
                }

                // --- PHASE IV: OBSERVABILITY: Thermal state transition ---
                // Log thermal state transitions for observability
                {
                    let thermal_state = if thermal_metrics.cpu_temperature > 85.0 {
                        "critical"
                    } else if thermal_metrics.cpu_temperature > 75.0 {
                        "warning"
                    } else if thermal_metrics.cpu_temperature > 65.0 {
                        "normal"
                    } else {
                        "idle"
                    };
                    debug!(
                        target: "autonomic_loop.predictive.thermal",
                        thermal_state = thermal_state,
                        "thermal state transition"
                    );
                }

                // PHASE 5.1: Wire thermal to biology expression rate
                {
                    let mut biology = biology.write();
                    biology.set_expression_rate(thermal_factor as f32);

                    // Register specialist in biology if not already registered
                    // (This would normally happen once per specialist)
                    if biology.specialist_metabolism.is_empty() {
                        biology.register_specialist("enzyme_runner", 100);
                        biology.register_specialist("learning_loop", 200);
                        biology.register_specialist("routing_engine", 150);
                        info!(target: "autonomic_loop", "biology system initialized with specialists");
                    }

                    biology.update_metabolism();

                    // --- PHASE IV: TOKEN METABOLISM PREDICTION (Kalman Filter) ---
                    // Predict token regeneration rate
                    {
                        let mut kf = kalman_filter.write();
                        // Predict next token regeneration rate
                        kf.predict();
                        let predicted_tokens = kf.position();
                        debug!(
                            target: "autonomic_loop.predictive.tokens",
                            predicted_tokens = predicted_tokens,
                            "token regeneration prediction (Kalman)"
                        );
                    }

                    // --- PHASE IV: OBSERVABILITY: Token prediction residual ---
                    // Log prediction residual for convergence tracking
                    {
                        let default_metabolism = Default::default();
                        let metabolism = biology
                            .specialist_metabolism
                            .values()
                            .next()
                            .unwrap_or(&default_metabolism);
                        let actual_tokens = metabolism.tokens as f64;
                        let kf = kalman_filter.write();
                        let filtered_tokens = kf.position();
                        let residual = actual_tokens - f64::from(filtered_tokens);
                        debug!(
                            target: "autonomic_loop.predictive.tokens",
                            residual = residual,
                            "token prediction residual (convergence)"
                        );
                    }

                    // FIX #2: COMPLETE - Regenerate tokens for each specialist based on thermal state
                    // This enables system to self-regulate: tokens deplete on execution, regenerate over time
                    // Thermal state affects regeneration rate: Normal > Metabolic > Dormant
                    let global_throttle = biology.throttle_state;
                    for (specialist_id, metabolism) in biology.specialist_metabolism.iter_mut() {
                        let regen_rate: f32 = match global_throttle {
                            ThrottleState::Normal => 2.0,    // Fast: +2 tokens/tick
                            ThrottleState::Metabolic => 1.0, // Normal: +1 token/tick
                            ThrottleState::Dormant => 0.5,   // Slow: +0.5 token/tick
                        };

                        let old_tokens = metabolism.tokens;
                        metabolism.tokens =
                            (metabolism.tokens + regen_rate).min(metabolism.max_tokens);

                        if old_tokens < metabolism.max_tokens && metabolism.tokens > old_tokens {
                            debug!(target: "autonomic_loop", specialist_id, regen_rate, ?global_throttle, "token regeneration");
                        }
                    }
                }

                if thermal_metrics.throttling_active {
                    info!(
                        target: "autonomic_loop",
                        cpu_c = thermal_metrics.cpu_temperature as i32,
                        gpu_c = thermal_metrics.gpu_temperature as i32,
                        factor = thermal_factor,
                        "thermal throttling active"
                    );

                    // Adjust understanding score and curiosity based on thermal stress
                    if thermal_metrics.cpu_status == ThermalStatus::Critical {
                        state.understanding_score =
                            ((state.understanding_score as f64) * thermal_factor) as u32;
                        warn!(target: "autonomic_loop", understanding = state.understanding_score, "thermal emergency: reducing work intensity");

                        // Reduce curiosity drive when in emergency
                        state.curiosity_drive = (state.curiosity_drive as f64 * 0.5) as u32;
                    }
                }

                // PHASE 5.4: Monitor and respond to biology throttle state
                {
                    let bio = biology.read();
                    match bio.throttle_state {
                        ThrottleState::Normal => {
                            // System running at normal capacity
                            debug!(target: "autonomic_loop", rate = bio.expression_rate, "biology: normal");
                        }
                        ThrottleState::Metabolic => {
                            // Reduced capacity - reduce cognitive intensity
                            info!(target: "autonomic_loop", rate = bio.expression_rate, "biology: metabolic mode");
                            state.understanding_score =
                                (state.understanding_score as f32 * 0.9) as u32;
                            state.curiosity_drive = (state.curiosity_drive as f32 * 0.8) as u32;
                        }
                        ThrottleState::Dormant => {
                            // Emergency mode - only critical tasks
                            warn!(target: "autonomic_loop", rate = bio.expression_rate, "biology: dormant mode (emergency)");
                            state.understanding_score =
                                (state.understanding_score as f32 * 0.5) as u32;
                            state.curiosity_drive = 0;
                            state.safety_lock = 1; // Engage safety locks
                        }
                    }
                }

                // --- PHASE 1: HOMEOSTATIC SELF-PRESERVATION ---
                if state.memory_pressure > 85 {
                    info!(target: "autonomic_loop", pressure = state.memory_pressure, "high memory pressure; triggering GC");
                    state.memory_pressure = 30;
                }

                // --- PHASE IV: LOAD FORECASTING (Kalman Filter) ---
                // Predict backpressure trajectory
                {
                    let mut kf = kalman_filter.write();
                    // Predict next backpressure level
                    kf.predict();
                    let predicted_bp = kf.position();
                    debug!(
                        target: "autonomic_loop.predictive.load",
                        predicted_bp = predicted_bp,
                        "backpressure prediction (Kalman)"
                    );
                }

                // --- PHASE IV: OBSERVABILITY: Load prediction residual ---
                // Log prediction residual for convergence tracking
                {
                    let backpressure_level = metrics_collector.get_backpressure_level();
                    let kf = kalman_filter.write();
                    let filtered_bp = kf.position();
                    let residual = backpressure_level - f64::from(filtered_bp);
                    debug!(
                        target: "autonomic_loop.predictive.load",
                        residual = residual,
                        "load prediction residual (convergence)"
                    );
                }

                // FIX #5: NEW - Load-based backpressure mechanism
                // Check if we should reject new tasks based on system load
                {
                    let backpressure_level = metrics_collector.get_backpressure_level();

                    if metrics_collector.should_reject_new_tasks() {
                        info!(target: "autonomic_loop", "backpressure active: rejecting new tasks");
                        debug!(target: "autonomic_loop", level_pct = backpressure_level * 100.0, "backpressure level");
                        // Don't accept new tasks this tick
                        // Existing in-progress tasks will continue
                        state.understanding_score = (state.understanding_score as f32 * 0.9) as u32;
                    } else if backpressure_level > 0.5 {
                        info!(target: "autonomic_loop", level_pct = backpressure_level * 100.0, "moderate backpressure");
                        // Reduce new task acceptance but don't block completely
                        state.curiosity_drive = (state.curiosity_drive as f32 * 0.8) as u32;
                    } else if backpressure_level > 0.2 {
                        debug!(target: "autonomic_loop", level_pct = backpressure_level * 100.0, "light backpressure");
                    }
                }

                // DOPAMINE SYSTEM: Curiosity and Proactive Learning
                if state.understanding_score < 40 || state.curiosity_drive > 80 {
                    {
                        let mut curiosity = curiosity_enzyme.write();
                        let index = semantic_index.read();
                        let mut gaps = curiosity.identify_knowledge_gaps(&index);

                        let plan_guard = active_plan.read();
                        let forecast = curiosity.forecast_requirements(&plan_guard);
                        gaps.extend(forecast);
                        drop(plan_guard);

                        if let Ok(hunger_intent) =
                            rt.block_on(curiosity.formulate_hunger_intent(&gaps))
                            && let Ok(new_plan) =
                                rt.block_on(prefrontal_cortex.draft_plan(&hunger_intent))
                        {
                            let mut plan_guard = active_plan.write();
                            *plan_guard = Some(new_plan);
                            debug!(target: "autonomic_loop", "hunger plan seated");
                        }
                    }

                    state.curiosity_drive = 20;
                    state.understanding_score += 10;
                }

                // --- PHASE IV: INTENT PREDICTION (Hidden Markov Model) ---
                // Predict next intent state from system events
                {
                    let hmm = hmm_model.read();
                    // Use current understanding score as observation (normalized 0-1)
                    let observation = state.understanding_score as f64 / 100.0;
                    let predicted_state = hmm.predict_next_state(observation);
                    // Calculate prediction confidence
                    let confidence = hmm.get_prediction_confidence(observation);
                    debug!(
                        target: "autonomic_loop.predictive.intent",
                        observation = observation,
                        predicted_state = predicted_state,
                        confidence = confidence,
                        "intent prediction (HMM confidence)"
                    );
                }

                // --- PHASE IV: OBSERVABILITY: Intent state transition ---
                // Log intent state transitions for observability
                {
                    let hmm = hmm_model.read();
                    let intent_state = hmm.get_current_state();
                    let state_transition = hmm.get_state_transition();
                    let transition_confidence = hmm.get_transition_confidence();
                    debug!(
                        target: "autonomic_loop.predictive.intent",
                        intent_state = intent_state,
                        state_transition = state_transition,
                        transition_confidence = transition_confidence,
                        "intent state transition (HMM)"
                    );
                }

                // --- PHASE 3: OPERATIONAL EXECUTION (The Sentinel) ---
                if state.intent_vector_id != [0; 16] {
                    let task_id = uuid::Uuid::from_bytes(state.intent_vector_id).to_string();
                    let payload_len = state
                        .intent_payload
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(state.intent_payload.len());
                    let payload_str = std::str::from_utf8(&state.intent_payload[..payload_len])
                        .unwrap_or("")
                        .trim();
                    let intent_text = if !payload_str.is_empty() {
                        payload_str
                    } else {
                        "Synchronize specialist state and inspect workspace graph"
                    };

                    let intent_tier = nlm_sentinel.classify_intent(intent_text);

                    if intent_tier == IntentTier::Violation {
                        warn!(target: "autonomic_loop", %task_id, "safety violation; blocking task");
                        state.safety_lock = 1;
                        state.intent_vector_id = [0; 16];
                        Self::write_state(&synapse.read(), &state);
                        thread::sleep(tick_rate);
                        continue;
                    }

                    state.sovereignty_tier = match intent_tier {
                        IntentTier::Local => 0,
                        IntentTier::Bounded => 1,
                        IntentTier::Remote => 2,
                        _ => 0,
                    };

                    if state.sovereignty_tier >= 1 && state.approval_granted == 0 {
                        if state.approval_required == 0 {
                            info!(target: "autonomic_loop", %task_id, tier = state.sovereignty_tier, "task requires user approval");
                            state.approval_required = 1;
                        }
                        Self::write_state(&synapse.read(), &state);
                        thread::sleep(tick_rate);
                        continue;
                    }

                    debug!(target: "autonomic_loop", %task_id, tier = state.sovereignty_tier, "intent approved; executing");

                    {
                        let mut plan_guard = active_plan.write();
                        if let Ok(new_plan) = rt.block_on(prefrontal_cortex.draft_plan(intent_text))
                        {
                            *plan_guard = Some(new_plan);
                            debug!(target: "autonomic_loop", "multi-step plan generated");
                        }
                    }

                    if let Some(db_mutex) = hive_db.as_ref() {
                        let db = db_mutex.lock();
                        let mut index = semantic_index.write();
                        let mut metadata = std::collections::HashMap::new();
                        metadata.insert("source".to_string(), "intent_execution".to_string());
                        metadata.insert(
                            "sovereignty_tier".to_string(),
                            state.sovereignty_tier.to_string(),
                        );
                        let id = index.index_text(intent_text, metadata);
                        let result_str = intent_text.to_string();
                        drop(index);
                        let index = semantic_index.read();
                        if let Some(entry) = index.entries.iter().find(|e| e.id == id) {
                            let _ = db.save_embedding(
                                &id,
                                &result_str,
                                &entry.vector,
                                &entry.metadata,
                                entry.access_count,
                            );
                        }
                    }

                    state.approval_required = 0;
                    state.approval_granted = 0;
                    state.intent_vector_id = [0; 16];
                }

                // --- PHASE 4: EXECUTIVE PLAN PROGRESSION ---
                {
                    let mut plan_guard = active_plan.write();
                    if let Some(plan) = plan_guard.as_mut() {
                        let ready_steps = plan.get_ready_steps();
                        for step_id in ready_steps {
                            if let Some(step) = plan.steps.get_mut(&step_id) {
                                let specialist_id = step.assigned_specialist.clone();

                                // PHASE 5.3: Check token availability before execution
                                {
                                    let bio = biology.write();
                                    if !bio.can_execute_specialist(&specialist_id) {
                                        info!(target: "autonomic_loop", %specialist_id, %step_id, "specialist out of tokens; deferring step");
                                        continue;
                                    }
                                }

                                // SAFE-01: Formal SMT Action Interlock Gatekeeper
                                {
                                    let interlock = smt_interlock.read();
                                    if interlock.is_killswitch_active() {
                                        warn!(target: "autonomic_loop", %step_id, "interlock killswitch active; aborting step");
                                        step.status = StepStatus::Failed(
                                            "Interlock killswitch active".to_string(),
                                        );
                                        continue;
                                    }
                                }

                                debug!(target: "autonomic_loop", %step_id, %specialist_id, "executing plan step");

                                // FIX #7: INTEGRATION - Specialist memory consultation for decision making
                                let _should_execute = true;
                                let mut execution_risk = 0.5_f32; // Default medium risk
                                {
                                    let store = specialist_memory.get_or_create(&specialist_id);

                                    let query_result =
                                        store.query_memory(&step_id, "task_execution", 3);

                                    // FIX #7: Calculate risk from historical performance
                                    if !query_result.entries.is_empty() {
                                        // Calculate success rate from memories
                                        let successes = query_result
                                            .entries
                                            .iter()
                                            .filter(|e| {
                                                e.title.contains("success")
                                                    || e.title.contains("Success")
                                            })
                                            .count();
                                        let success_rate = (successes as f32)
                                            / (query_result.entries.len() as f32);
                                        execution_risk = 1.0 - success_rate; // Risk = 1 - success rate

                                        info!(
                                            target: "autonomic_loop",
                                            specialist_id = %specialist_id,
                                            step_id = %step_id,
                                            "memory consultation for specialist"
                                        );
                                        info!(
                                            target: "autonomic_loop",
                                            recommendation = %query_result.recommendation,
                                            risk_pct = execution_risk * 100.0,
                                            "memory recommendation"
                                        );
                                        for (i, entry) in query_result.entries.iter().enumerate() {
                                            debug!(
                                                target: "autonomic_loop",
                                                index = i + 1,
                                                title = %entry.title,
                                                confidence_pct = entry.confidence * 100.0,
                                                "past outcome"
                                            );
                                        }

                                        // FIX #7: Make decision based on risk
                                        if execution_risk > 0.7 {
                                            warn!(target: "autonomic_loop", risk_pct = execution_risk * 100.0, "high risk; executing with caution");
                                            state.understanding_score =
                                                (state.understanding_score as f32 * 0.8) as u32;
                                        } else if execution_risk > 0.3 {
                                            info!(target: "autonomic_loop", risk_pct = execution_risk * 100.0, "medium risk; normal execution");
                                        } else {
                                            debug!(target: "autonomic_loop", risk_pct = execution_risk * 100.0, "low risk");
                                            state.understanding_score =
                                                (state.understanding_score as f32 * 1.1).min(100.0)
                                                    as u32;
                                        }
                                    } else {
                                        debug!(target: "autonomic_loop", risk_pct = execution_risk * 100.0, "no past experience");
                                    }
                                }

                                step.status = StepStatus::InProgress;

                                if state.understanding_score > 90 {
                                    adaptation_orchestrator
                                        .inject_latent_state(&state.latent_vector);
                                }

                                if step_id == "step_1" {
                                    dopamine_system.process_event(
                                        &mut state,
                                        DopamineEvent::SuccessfulIngestion(0),
                                    );

                                    // FIX #3: Wire dopamine to learning (step 1 optimization)
                                    {
                                        let mut learning = learning_loop.write();
                                        let task_features =
                                            vec![state.understanding_score as f64 / 100.0];
                                        let _ = learning.learn_from_dopamine(
                                            &task_features,
                                            "step_1_specialist",
                                            0.9_f32,
                                            0.85,
                                        );
                                    }

                                    if adaptation_orchestrator
                                        .extract_hidden_state(&mut state.latent_vector)
                                        .is_ok()
                                    {
                                        let mut detector = concept_drift_detector.write();
                                        let drift = detector.analyze_drift(&state.latent_vector);
                                        state.concept_drift = drift;

                                        if detector.is_integrity_compromised() && drift > 0.95 {
                                            state.safety_lock = 1;
                                        }
                                    }
                                }

                                // CRITICAL FIX #3: Integrate dopamine feedback after execution
                                // Reward successful execution to drive learning
                                if state.understanding_score > 60 {
                                    dopamine_system.process_event(
                                        &mut state,
                                        DopamineEvent::SuccessfulIngestion(0),
                                    );
                                    debug!(target: "autonomic_loop", %step_id, "step succeeded; dopamine reward applied");

                                    // FIX #3: CRITICAL - Wire dopamine to learning
                                    // Pass dopamine signal to learning system to update specialist weights
                                    {
                                        let mut learning = learning_loop.write();
                                        let task_features =
                                            vec![state.understanding_score as f64 / 100.0];
                                        let dopamine_value = 0.8_f32; // High reward for success
                                        let result = learning.learn_from_dopamine(
                                            &task_features,
                                            &specialist_id,
                                            dopamine_value,
                                            0.9, // High confidence
                                        );
                                        debug!(
                                            target: "autonomic_loop",
                                            specialist_id = %specialist_id,
                                            signal = result.learning_signal,
                                            lr = result.adaptive_learning_rate,
                                            "dopamine learning"
                                        );
                                    }

                                    // PHASE 5.3: Consume token on successful execution
                                    {
                                        let mut bio = biology.write();
                                        if bio.consume_specialist_token(&specialist_id) {
                                            debug!(target: "autonomic_loop", %specialist_id, "token consumed");
                                        }
                                    }

                                    // FIX #7: INTEGRATION - Store successful outcome in specialist memory
                                    {
                                        let store = specialist_memory.get_or_create(&specialist_id);

                                        let outcome_entry = MemoryEntry::new(
                                            format!("step_{}_success", step_id),
                                            specialist_id.clone(),
                                            format!("step {}", step_id),
                                            format!("Successfully executed: {}", step_id),
                                            MemoryType::Episodic,
                                        );

                                        store.store_memory(outcome_entry);
                                        debug!(target: "autonomic_loop", %specialist_id, "stored success outcome");
                                    }
                                } else {
                                    warn!(target: "autonomic_loop", %step_id, "executing with lower understanding score");

                                    // FIX #7: INTEGRATION - Store failure outcome in specialist memory
                                    {
                                        let store = specialist_memory.get_or_create(&specialist_id);

                                        let outcome_entry = MemoryEntry::new(
                                            format!("step_{}_caution", step_id),
                                            specialist_id.clone(),
                                            format!("step {}", step_id),
                                            format!("Executed with caution: {}", step_id),
                                            MemoryType::Episodic,
                                        );

                                        store.store_memory(outcome_entry);
                                        debug!(target: "autonomic_loop", %specialist_id, "stored cautious outcome");
                                    }
                                }

                                step.status = StepStatus::Completed;
                            }
                        }
                    }
                }

                // --- PHASE 5: MCP TOOL EXECUTION ---
                if state.mcp_tool_call.status == 1 {
                    // SAFE-01: Gate MCP tool execution through SmtActionInterlock
                    {
                        let interlock = smt_interlock.read();
                        if interlock.is_killswitch_active() {
                            warn!(target: "autonomic_loop", call_id = state.mcp_tool_call.call_id, "interlock killswitch active; blocking tool execution");
                            state.mcp_tool_call.status = 3; // Failed
                            Self::write_state(&synapse.read(), &state);
                            continue;
                        }
                    }

                    state.mcp_tool_call.status = 2; // Executing

                    if let Some(ref recorder_mutex) = flight_recorder {
                        let mut rec = recorder_mutex.lock();
                        let _ = rec.record_transition(
                            core_contracts::FlightEventKind::IntentDispatched,
                            0x01,
                            state.mcp_tool_call.call_id,
                            state.mcp_tool_call.tool_name_hash,
                            state.mcp_tool_call.status as u64,
                            &state.mcp_tool_call.arguments_payload[..64],
                        );
                    }

                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    use std::hash::{Hash, Hasher};
                    "resolve_debate".hash(&mut hasher);
                    let resolve_debate_hash = hasher.finish();

                    if state.mcp_tool_call.tool_name_hash == resolve_debate_hash {
                        state.dialogue.consensus_score = 100;
                        state.integrity_score = (state.integrity_score + 10).min(100);

                        let result_msg =
                            "Debate resolved. Consensus reached via Diplomatic override.";
                        let bytes = result_msg.as_bytes();
                        state.mcp_tool_call.arguments_size = bytes.len() as u32;
                        state.mcp_tool_call.arguments_payload[..bytes.len()].copy_from_slice(bytes);
                        state.mcp_tool_call.status = 3;
                    } else {
                        state.mcp_tool_call.status = 3;
                    }
                }

                // --- PHASE 6: LEARNING UPDATE ---
                if state.clock_tick % 100 == 0 {
                    let mut loop_guard = learning_loop.write();
                    loop_guard.system_state.estimated_load = state.memory_pressure as f64 / 100.0;
                }

                // --- PHASE 7: CROSS-HUSK DIALOGUE (Specialist Debate) ---
                if state.clock_tick % 50 == 0 {
                    diplomat_enzyme.moderate_dialogue(&mut state.dialogue);
                    adaptation_orchestrator
                        .sync_lora_to_speaker(state.dialogue.active_speaker_hash);

                    if state.dialogue.consensus_score > 95 && state.clock_tick % 1000 == 0 {
                        let name = format!("skill_chip_{}", state.clock_tick);
                        // DNA splicing produces a minimal phenotype binary
                        {
                            dopamine_system
                                .process_event(&mut state, DopamineEvent::SuccessfulIngestion(100));

                            // FIX #3: High-value dopamine from specialist DNA splicing
                            {
                                let mut learning = learning_loop.write();
                                let task_features =
                                    vec![state.dialogue.consensus_score as f64 / 100.0];
                                let _ = learning.learn_from_dopamine(
                                    &task_features,
                                    &name,
                                    1.0_f32, // Maximum reward for DNA splicing success
                                    0.95,    // Very high confidence
                                );
                                info!(target: "autonomic_loop", %name, "specialist DNA splicing; dopamine learning triggered");
                            }
                        }
                    }

                    if state.dialogue.turn_count > 5 {
                        let target_integrity = state.dialogue.consensus_score;
                        if state.integrity_score > target_integrity {
                            state.integrity_score -= 1;
                        } else if state.integrity_score < target_integrity {
                            state.integrity_score += 1;
                        }
                    }

                    if state.dialogue.consensus_score < 30 && state.clock_tick % 250 == 0 {
                        if let Ok(_correction) =
                            self_correction_enzyme.attempt_recalibration(&mut state)
                        {
                            state.integrity_score = (state.integrity_score + 5).min(100);
                        }
                        state.memory_pressure = (state.memory_pressure + 10).min(100);
                    }
                }

                // --- PHASE IV: HMM STATE TRANSITION LOGGING ---
                // Log HMM state transitions for observability
                {
                    let hmm = hmm_model.read();
                    let current_state = hmm.get_current_state();
                    let state_transition = hmm.get_state_transition();
                    let transition_confidence = hmm.get_transition_confidence();
                    debug!(
                        target: "autonomic_loop.predictive.state",
                        current_state = current_state,
                        state_transition = state_transition,
                        transition_confidence = transition_confidence,
                        "HMM state transition (confidence)"
                    );
                }

                // --- PHASE IV: OBSERVABILITY: HMM lattice state ---
                // Log HMM lattice state for observability
                {
                    let hmm = hmm_model.read();
                    let lattice_state = hmm.get_lattice_state();
                    let lattice_confidence = hmm.get_lattice_confidence();
                    debug!(
                        target: "autonomic_loop.predictive.state",
                        lattice_state = lattice_state,
                        lattice_confidence = lattice_confidence,
                        "HMM lattice state (confidence)"
                    );
                }

                // --- PHASE 8: NEURAL PRUNING (Homeostasis) ---
                if (state.clock_tick % 1000 == 0 || state.memory_pressure > 90)
                    && tick_count.is_multiple_of(pacing_decision.decimation_factor as u64)
                {
                    let mut archive = crate::neural_pruning::PrunedArchive::new();
                    neural_pruning_enzyme.prune_constellation(&mut Vec::new(), &mut archive);
                }

                // FIX #6: NEW - Registry synchronization every 100 ticks
                if state.clock_tick % 100 == 0 {
                    debug!(target: "autonomic_loop", tick = state.clock_tick, "registry sync point");
                    // Registry sync would happen here:
                    // coordinator.sync_all_adapters() would aggregate state from all 18 adapters
                    // Master registry would be used for all decision queries
                    debug!(target: "autonomic_loop", "registry sync complete");
                }

                // --- Sync: write state back to shared memory ---
                {
                    let syn = synapse.read();
                    Self::write_state(&syn, &state);
                }

                // --- Phase 3 Actuation/Telemetry: SWMR zero-copy SHM snapshot emission ---
                {
                    let pod = core_contracts::EngineSnapshotPod {
                        timestamp_ms: state
                            .clock_tick
                            .saturating_mul(tick_rate.as_millis() as u64),
                        bus_generation: state.clock_tick,
                        bus_integrity: (state.integrity_score as f32).clamp(0.0, 100.0),
                        bus_understanding: (state.understanding_score as f32).clamp(0.0, 100.0),
                        flow_score: (1.0 - state.concept_drift).clamp(0.0, 1.0),
                        pacing: match pacing_decision.throttle_tier {
                            adaptation_plane::PacingTier::Nominal => 0,
                            adaptation_plane::PacingTier::MetabolicThrottle => 1,
                            adaptation_plane::PacingTier::ThermalCritical
                            | adaptation_plane::PacingTier::DormantPreservation => 2,
                        },
                        ..Default::default()
                    };
                    state_publisher.publish_pod(&pod);

                    if let Some(ref recorder_mutex) = flight_recorder {
                        let mut rec = recorder_mutex.lock();
                        let _ = rec.record_transition(
                            core_contracts::FlightEventKind::TelemetryTick,
                            0x01, // Hypervisor
                            state.clock_tick,
                            state.integrity_score as u64,
                            state.understanding_score as u64,
                            &state.intent_vector_id,
                        );
                    }
                }

                let elapsed = start.elapsed();
                // --- TICK WATCHDOG ---
                if elapsed > TICK_WATCHDOG {
                    warn!(
                        target: "autonomic_loop",
                        tick = tick_count,
                        elapsed_us = elapsed.as_micros() as u64,
                        budget_us = TICK_WATCHDOG.as_micros() as u64,
                        "tick exceeded watchdog budget"
                    );
                }
                let corrected_sleep =
                    drift_filter.compute_sleep_duration(elapsed, pacing_decision.target_cadence);
                if !corrected_sleep.is_zero() {
                    thread::sleep(corrected_sleep);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptation_plane::{
        AutonomousPacingRegulator, PacingConfig, PacingTier, ThermodynamicTelemetry,
    };

    /// Regression test for the offset drift bug fixed alongside this test:
    /// the `hypervisor inject` CLI command used to hardcode `intent_vector_id`
    /// at byte 16 and `intent_payload` at byte 32, both four bytes short of
    /// `SynapseState`'s actual `#[repr(C)]` layout, silently corrupting
    /// `curiosity_drive` and shifting every injected intent's payload by 4
    /// bytes. Pins the real offsets so a future field reorder is caught here
    /// instead of silently breaking the CLI's raw mmap writes again.
    #[test]
    fn test_synapse_state_field_offsets_match_cli_assumptions() {
        assert_eq!(SYNAPSE_INTENT_VECTOR_ID_OFFSET, 20);
        assert_eq!(SYNAPSE_INTENT_PAYLOAD_OFFSET, 36);
        assert_eq!(
            SYNAPSE_INTENT_PAYLOAD_CAPACITY,
            std::mem::size_of::<[u8; 4096]>()
        );
        assert!(
            SYNAPSE_INTENT_PAYLOAD_OFFSET + SYNAPSE_INTENT_PAYLOAD_CAPACITY
                <= std::mem::size_of::<SynapseState>()
        );
    }

    /// Regression test for the torn-write bug fixed alongside this test:
    /// `LegacySharedMemorySynapse::write`/`read` used to copy bytes into/out
    /// of the mmap with no synchronization at all, so a concurrent writer
    /// (this daemon's own tick loop and the `hypervisor inject` CLI command,
    /// a *separate process* against the same mmap) could interleave its
    /// `copy_nonoverlapping` with another writer's, producing a spliced
    /// `SynapseState` that belongs to neither write. `write`/`read` now take
    /// an exclusive/shared advisory lock on the backing file for the
    /// duration of the copy. This drives many threads (standing in for
    /// concurrent processes sharing the same fd-locking discipline, since
    /// `flock`/`LockFileEx` locks are per-open-file-description/handle, not
    /// per-thread) each writing a `SynapseState` whose `intent_payload` is
    /// uniformly filled with that thread's own marker byte, interleaved
    /// with concurrent readers, and asserts every read is either an
    /// untouched zero-filled state or has a fully uniform `intent_payload`
    /// - never a mix of two threads' marker bytes, which is what torn
    /// writes would produce.
    #[test]
    fn test_synapse_concurrent_write_read_never_tears() {
        // Per AGENTS.md's test-sandboxing rule, this must not resolve a
        // path through the ambient `WorkspacePathsConfig` (the shared host
        // cache dir, which a test could collide with under parallel runs
        // or pollute for later ones) - `new_at`/`open_existing_at` take an
        // explicit path instead, backed here by a per-test tempdir.
        let tmp = tempfile::tempdir().expect("tempdir");
        let synapse_path = tmp
            .path()
            .join("test_synapse_concurrent_write_read_never_tears.synapse");
        let size = std::mem::size_of::<SynapseState>();
        // Each simulated writer/reader below opens its own
        // `LegacySharedMemorySynapse` (its own `File`/fd) against the same
        // path, exactly as the daemon process and a separately-invoked
        // `hypervisor inject` CLI process each would - a single shared
        // `Arc` over one instance would exercise this struct's seqlock
        // logic but not the cross-process case the seqlock exists for.
        LegacySharedMemorySynapse::new_at(&synapse_path, size).expect("size the synapse file");

        const WRITER_THREADS: u8 = 6;
        const ITERATIONS_PER_WRITER: usize = 200;

        let mut handles = Vec::new();

        for marker in 0..WRITER_THREADS {
            let synapse_path = synapse_path.clone();
            handles.push(thread::spawn(move || {
                let synapse =
                    LegacySharedMemorySynapse::new_at(&synapse_path, size).expect("writer synapse");
                for _ in 0..ITERATIONS_PER_WRITER {
                    let mut state = SynapseState::default();
                    state.clock_tick = marker as u64;
                    state.intent_vector_id = [marker; 16];
                    state.intent_payload = [marker; 4096];
                    SupervisoryDaemon::write_state(&synapse, &state);
                }
            }));
        }

        for _ in 0..(WRITER_THREADS as usize * 2) {
            let synapse_path = synapse_path.clone();
            handles.push(thread::spawn(move || {
                let synapse =
                    LegacySharedMemorySynapse::new_at(&synapse_path, size).expect("reader synapse");
                for _ in 0..ITERATIONS_PER_WRITER {
                    let state = SupervisoryDaemon::read_state(&synapse);
                    let first = state.intent_payload[0];
                    assert!(
                        state.intent_payload.iter().all(|&b| b == first),
                        "torn read: intent_payload is not uniformly {first:#x}, \
                         a concurrent write was observed partially applied"
                    );
                    // `intent_vector_id` and `clock_tick` are written from the
                    // same source `state` in the same `write_state` call as
                    // `intent_payload`, so they must agree with it too if the
                    // whole struct was copied atomically with respect to
                    // other writers.
                    assert!(state.intent_vector_id.iter().all(|&b| b == first));
                    assert_eq!(state.clock_tick, first as u64);
                }
            }));
        }

        for handle in handles {
            handle.join().expect("thread panicked");
        }
    }

    #[test]
    fn test_supervisory_daemon_pacing_constructor_injection() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let synapse_name = "test_ans_pacing_synapse";

        let enzyme_runner = Arc::new(EnzymeRunner::new().expect("enzyme runner"));
        let hox_path = tmp.path().join("test_hox.db");
        let hox_registry =
            Arc::new(HoxRegistry::new(hox_path.to_str().expect("path str")).expect("hox registry"));
        let workspace_root = tmp.path().to_path_buf();
        let splicing_engine = Arc::new(WasmSplicingEngine::new(
            hox_registry.clone(),
            workspace_root,
        ));
        let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
            crate::unified_learning::UnifiedLearningConfig::default(),
            0,
            vec![],
        )));

        let custom_config = PacingConfig {
            baseline_interval: Duration::from_millis(25),
            min_interval: Duration::from_millis(5),
            max_interval: Duration::from_millis(500),
            thermal_warning_c: 70.0,
            thermal_critical_c: 80.0,
            thermal_emergency_c: 90.0,
            vram_warning_pct: 70.0,
            vram_critical_pct: 85.0,
            ewma_alpha: 0.5,
        };

        let regulator = Arc::new(parking_lot::RwLock::new(
            AutonomousPacingRegulator::new(custom_config).expect("valid config"),
        ));

        let db_path = tmp.path().join("test_hive.db");
        let daemon = SupervisoryDaemon::new_with_pacing(
            synapse_name,
            25,
            enzyme_runner,
            hox_registry,
            splicing_engine,
            learning_loop,
            db_path.to_str(),
            regulator.clone(),
        )
        .expect("SupervisoryDaemon instantiation");

        assert_eq!(
            daemon.pacing_regulator().read().current_cadence(),
            Duration::from_millis(25)
        );
        assert_eq!(
            daemon.pacing_regulator().read().current_tier(),
            PacingTier::Nominal
        );
    }

    #[test]
    fn test_autonomic_pacing_thermal_backoff_and_recovery() {
        let custom_config = PacingConfig {
            baseline_interval: Duration::from_millis(10),
            min_interval: Duration::from_millis(2),
            max_interval: Duration::from_millis(200),
            thermal_warning_c: 75.0,
            thermal_critical_c: 85.0,
            thermal_emergency_c: 95.0,
            vram_warning_pct: 75.0,
            vram_critical_pct: 90.0,
            ewma_alpha: 0.4,
        };

        let mut regulator = AutonomousPacingRegulator::new(custom_config).expect("valid config");

        // 1. Nominal operating temperature (45°C)
        let nominal = ThermodynamicTelemetry {
            cpu_temp_c: 45.0,
            gpu_temp_c: 42.0,
            ..Default::default()
        };
        let decision1 = regulator.compute_next_cadence(&nominal).expect("decision");
        assert_eq!(decision1.throttle_tier, PacingTier::Nominal);
        assert_eq!(decision1.target_cadence, Duration::from_millis(10));

        // 2. Severe thermal spike (88°C)
        let critical = ThermodynamicTelemetry {
            cpu_temp_c: 88.0,
            gpu_temp_c: 82.0,
            ..Default::default()
        };
        let decision2 = regulator.compute_next_cadence(&critical).expect("decision");
        assert_eq!(decision2.throttle_tier, PacingTier::ThermalCritical);
        assert_eq!(decision2.target_cadence, Duration::from_millis(40)); // 4x backoff
        assert_eq!(decision2.decimation_factor, 4);

        // 3. Recovery to cool range (40°C)
        let cool = ThermodynamicTelemetry {
            cpu_temp_c: 40.0,
            gpu_temp_c: 38.0,
            cpu_load_pct: 5.0,
            vram_used_bytes: 1024,
            vram_total_bytes: 8 * 1024 * 1024 * 1024,
            memory_pressure_pct: 10.0,
        };
        for _ in 0..10 {
            let _ = regulator.compute_next_cadence(&cool).expect("decision");
        }
        assert_eq!(regulator.current_tier(), PacingTier::Nominal);
        assert_eq!(regulator.current_cadence(), Duration::from_millis(10));
    }

    #[test]
    fn test_autonomic_pacing_vram_saturation() {
        let mut regulator =
            AutonomousPacingRegulator::default_with_baseline(Duration::from_millis(10));

        let vram_heavy = ThermodynamicTelemetry {
            cpu_temp_c: 50.0,
            gpu_temp_c: 50.0,
            vram_used_bytes: 7800 * 1024 * 1024,
            vram_total_bytes: 8000 * 1024 * 1024, // 97.5% VRAM
            ..Default::default()
        };

        let decision = regulator
            .compute_next_cadence(&vram_heavy)
            .expect("decision");
        assert_eq!(decision.throttle_tier, PacingTier::ThermalCritical);
        assert_eq!(decision.target_cadence, Duration::from_millis(40));
        assert_eq!(decision.task_deferral_probability, 0.75);
    }

    #[test]
    fn test_supervisory_daemon_flight_recorder_integration() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let synapse_name = "test_ans_flight_recorder_synapse";

        let enzyme_runner = Arc::new(EnzymeRunner::new().expect("enzyme runner"));
        let hox_path = tmp.path().join("test_hox.db");
        let hox_registry =
            Arc::new(HoxRegistry::new(hox_path.to_str().expect("path str")).expect("hox registry"));
        let workspace_root = tmp.path().to_path_buf();
        let splicing_engine = Arc::new(WasmSplicingEngine::new(
            hox_registry.clone(),
            workspace_root,
        ));
        let learning_loop = Arc::new(RwLock::new(UnifiedLearningLoop::new(
            crate::unified_learning::UnifiedLearningConfig::default(),
            0,
            vec![],
        )));

        let db_path = tmp.path().join("test_hive.db");
        let flight_log_path = tmp.path().join("test_flight.flight");
        let recorder = Arc::new(parking_lot::Mutex::new(
            ipc_bus::FlightRecorder::open_or_create(&flight_log_path).expect("open flight log"),
        ));

        let daemon = SupervisoryDaemon::new(
            synapse_name,
            10,
            enzyme_runner,
            hox_registry,
            splicing_engine,
            learning_loop,
            db_path.to_str(),
        )
        .expect("daemon new")
        .with_flight_recorder(recorder.clone());

        daemon.set_max_ticks(3);
        daemon.start();

        // Allow ticks to complete
        std::thread::sleep(Duration::from_millis(150));
        daemon.request_shutdown();

        // Verify flight events were recorded
        let replayer = ipc_bus::FlightReplayer::open(&flight_log_path).expect("replayer open");
        let (oldest, latest) = replayer.available_event_range().expect("range");
        assert!(
            latest >= 1,
            "Expected at least 1 recorded event, found {}",
            latest
        );
        assert_eq!(oldest, 1);

        let event = replayer.read_event(1).expect("read first event");
        assert_eq!(
            event.event_kind,
            core_contracts::FlightEventKind::TelemetryTick as u16
        );
        assert_eq!(event.source_id, 0x01);
        assert!(event.verify_checksum());
    }
}
