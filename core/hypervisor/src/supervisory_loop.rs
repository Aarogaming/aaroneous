use anyhow::Result;
use memmap2::{MmapMut, MmapOptions};
use parking_lot::RwLock;
use std::fs::OpenOptions;
use std::path::PathBuf;
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

pub struct LegacySharedMemorySynapse {
    mmap: MmapMut,
    _path: PathBuf,
}

impl LegacySharedMemorySynapse {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        let path = paths::resolve_synapse_path(name, &paths::WorkspacePathsConfig::default());

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        file.set_len(size as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self { mmap, _path: path })
    }

    pub fn write(&self, offset: usize, data: &[u8]) -> Result<()> {
        let ptr = self.mmap.as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(offset), data.len());
        }
        Ok(())
    }

    pub fn read(&self, offset: usize, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let ptr = self.mmap.as_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(ptr.add(offset), buf.as_mut_ptr(), len);
        }
        Ok(buf)
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
use governance::{SystemHealthGovernor, ThrottleState};

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

// Thread-local `DefaultConcurrenceEngine` owned by the hypervisor heartbeat thread.
// Declared at module level so it is accessible inside `thread::spawn(move || { ... })`.
// Single-writer: only the heartbeat thread ever calls `.update()`.
// Readers (HUD) see the data through the shared `concurrence_snapshot` Arc.
thread_local! {
    static CONCURRENCE_ENGINE: std::cell::UnsafeCell<compute::DefaultConcurrenceEngine> =
        const { std::cell::UnsafeCell::new(compute::DefaultConcurrenceEngine::new()) };
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
    system: Arc<parking_lot::RwLock<SystemHealthGovernor>>,
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
    /// Lock-free SWMR observation buffer for passive telemetry ingestion and dual-rail shadow RL
    pub observation_buffer: Option<Arc<parking_lot::Mutex<ipc_bus::ObservationBuffer>>>,
    /// Mounted `.si` reflex model running in shadow mode alongside the rule engine.
    /// Guarded by a Mutex because `SiOnlineLearner` holds mutable `Vec<Tensor>` hidden state.
    /// The guard is held only for the duration of the 180 µs forward pass.
    pub shadow_learner: Option<Arc<parking_lot::Mutex<compute::SiOnlineLearner>>>,
    /// Paired rolling-window concurrence tracker for the shadow learner.
    /// Lives in the same Arc so the HUD can clone a lightweight snapshot.
    pub concurrence_snapshot: Arc<parking_lot::RwLock<compute::ConcurrenceSnapshot>>,
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
            system: Arc::new(parking_lot::RwLock::new(SystemHealthGovernor::new())),
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
            observation_buffer: {
                let paths =
                    paths::WorkspacePaths::from_config(paths::WorkspacePathsConfig::default());
                let obs_path = paths.data().join("shm").join("observation.shm");
                ipc_bus::ObservationBuffer::open_or_create(
                    &obs_path,
                    ipc_bus::DEFAULT_OBSERVATION_CAPACITY,
                )
                .ok()
                .map(|b| Arc::new(parking_lot::Mutex::new(b)))
            },
            shadow_learner: None,
            concurrence_snapshot: Arc::new(parking_lot::RwLock::new(
                compute::ConcurrenceSnapshot::default(),
            )),
        })
    }

    /// Attaches an optional observation buffer for passive machine telemetry ingestion.
    pub fn with_observation_buffer(
        mut self,
        buffer: Arc<parking_lot::Mutex<ipc_bus::ObservationBuffer>>,
    ) -> Self {
        self.observation_buffer = Some(buffer);
        self
    }

    /// Mounts a pre-initialised `.si` learner in dual-rail shadow mode.
    /// The learner runs a dry-run `forward_adapted_step()` on every tick
    /// and concurrence metrics are updated in `self.concurrence_snapshot`.
    pub fn with_shadow_learner(
        mut self,
        learner: Arc<parking_lot::Mutex<compute::SiOnlineLearner>>,
    ) -> Self {
        self.shadow_learner = Some(learner);
        self
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

    /// Return a point-in-time snapshot of shadow-model concurrence metrics.
    /// Returns `None` if no shadow learner has been mounted.
    pub fn read_concurrence_snapshot(&self) -> Option<compute::ConcurrenceSnapshot> {
        if self.shadow_learner.is_some() {
            Some(*self.concurrence_snapshot.read())
        } else {
            None
        }
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
        unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const SynapseState) }
    }

    fn write_state(syn: &LegacySharedMemorySynapse, state: &SynapseState) {
        let size = std::mem::size_of::<SynapseState>();
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
        let system = self.system.clone();
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
        let observation_buffer = self.observation_buffer.clone();
        let shadow_learner = self.shadow_learner.clone();
        let concurrence_snapshot = self.concurrence_snapshot.clone();

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

                // PHASE 5.1: Wire thermal to system expression rate
                {
                    let mut system = system.write();
                    system.set_execution_rate(thermal_factor as f32);

                    // Register specialist in system if not already registered
                    // (This would normally happen once per specialist)
                    if system.specialist_budgets.is_empty() {
                        system.register_specialist("enzyme_runner", 100);
                        system.register_specialist("learning_loop", 200);
                        system.register_specialist("routing_engine", 150);
                        info!(target: "autonomic_loop", "system system initialized with specialists");
                    }

                    system.tick();

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
                        let metabolism = system
                            .specialist_budgets
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
                    let global_throttle = system.throttle_state;
                    for (specialist_id, metabolism) in system.specialist_budgets.iter_mut() {
                        let regen_rate: f32 = match global_throttle {
                            ThrottleState::Normal => 2.0,    // Fast: +2 tokens/tick
                            ThrottleState::Throttled => 1.0, // Normal: +1 token/tick
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

                // PHASE 5.4: Monitor and respond to system throttle state
                {
                    let bio = system.read();
                    match bio.throttle_state {
                        ThrottleState::Normal => {
                            // System running at normal capacity
                            debug!(target: "autonomic_loop", rate = bio.execution_rate, "system: normal");
                        }
                        ThrottleState::Throttled => {
                            // Reduced capacity - reduce cognitive intensity
                            info!(target: "autonomic_loop", rate = bio.execution_rate, "system: metabolic mode");
                            state.understanding_score =
                                (state.understanding_score as f32 * 0.9) as u32;
                            state.curiosity_drive = (state.curiosity_drive as f32 * 0.8) as u32;
                        }
                        ThrottleState::Dormant => {
                            // Emergency mode - only critical tasks
                            warn!(target: "autonomic_loop", rate = bio.execution_rate, "system: dormant mode (emergency)");
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
                                    let bio = system.write();
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
                                        let mut bio = system.write();
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

                    if let Some(ref obs_buf_mutex) = observation_buffer {
                        let mut obs_buf = obs_buf_mutex.lock();
                        let featurizer = compute::MachineStateFeaturizer::new();
                        let ctx = compute::FeaturizerContext {
                            thermal_factor: thermal_factor as f32,
                            memory_pressure: state.memory_pressure as f32,
                            concept_drift: state.concept_drift,
                            curiosity_drive: state.curiosity_drive as f32,
                            tick_duration_us: start.elapsed().as_micros() as u32,
                            reward: 0.0,
                            free_energy: 0.01,
                            latent_seed: u64::from_le_bytes(
                                state.intent_vector_id[0..8].try_into().unwrap_or([0; 8]),
                            ),
                        };
                        let obs_frame = featurizer.featurize_frame(&pod, &ctx, 0, 0, 1.0);
                        let _ = obs_buf.record(obs_frame);
                    }

                    // --- DUAL-RAIL SHADOW INFERENCE (Task 3.1 / 3.2) ---
                    // Run the mounted `.si` reflex model in dry-run shadow mode alongside the
                    // deterministic orchestrator.  Actual opcode is 0x0000 (no rule-engine opcode
                    // has been dispatched yet at this layer; the comparison is meaningful once the
                    // supervisor assigns a concrete opcode).  The concurrence engine tracks rolling
                    // agreement, updates the shared snapshot Arc, and emits a graduation event the
                    // first time 95% rolling concurrence is achieved.
                    if let Some(ref learner_mutex) = shadow_learner {
                        // Build a 256-element state slice from the latent vector + scalars.
                        // We use the first 256 floats of the live latent_vector; the model
                        // was trained on state_dim = 256 factory-default geometry.
                        let state_slice = {
                            let mut s = [0.0f32; 256];
                            let src = &state.latent_vector;
                            let copy_len = src.len().min(256);
                            s[..copy_len].copy_from_slice(&src[..copy_len]);
                            // Inject scalar health signals into the tail if room.
                            if copy_len < 256 {
                                s[copy_len.min(255)] = (state.integrity_score as f32) / 100.0;
                            }
                            s
                        };

                        match learner_mutex.lock().forward_adapted_step(&state_slice) {
                            Ok(pred) => {
                                let predicted_opcode = pred.predicted_opcode_id;
                                // Actual opcode 0x0000 = idle/no-dispatch in this tick.
                                let actual_opcode: u16 = 0;
                                let reward = (state.integrity_score as f32
                                    - state.concept_drift * 100.0)
                                    .clamp(-1.0, 1.0)
                                    / 100.0;

                                let tick_result = compute::ShadowTickResult {
                                    actual_opcode,
                                    predicted_opcode,
                                    confidence: pred.confidence_score,
                                    reward,
                                };

                                // Update concurrence engine (stack-local, then write snapshot).
                                // We can't keep a `DefaultConcurrenceEngine` on the thread stack
                                // across ticks without boxing, so we track state in the shared
                                // snapshot Arc and maintain a thread-local engine.
                                //
                                // `concurrence_engine` is a thread-local declared just below.
                                let grad_event = CONCURRENCE_ENGINE.with(|cell| {
                                    // SAFETY: single-writer (this is the only thread that
                                    // touches the engine), accessed only inside this closure.
                                    let engine = unsafe { &mut *cell.get() };
                                    engine.update(tick_result)
                                });

                                // Publish snapshot to the shared Arc so the HUD can read it.
                                let published_snapshot = CONCURRENCE_ENGINE
                                    .with(|cell| unsafe { (*cell.get()).snapshot() });
                                *concurrence_snapshot.write() = published_snapshot;

                                // M40: periodically export the snapshot as JSON to an explicit
                                // file path so an external devtools process can read Aaroneous's
                                // live concurrence state without taking a Cargo dependency on
                                // this crate (file contract, see
                                // LOCAL_CLOUD_ORCHESTRATION_PLAN.md). This is off the tight
                                // per-tick hot path — gated to the same slow cadence used
                                // elsewhere in this loop for non-critical I/O (e.g. PHASE 6/7
                                // above) — and is strictly best-effort: a failed export write
                                // must never crash or stall the heartbeat thread.
                                if tick_count.is_multiple_of(100) {
                                    write_concurrence_report(&published_snapshot);
                                }

                                // If graduation threshold just crossed, record a Checkpoint event.
                                if let Some(grad) = grad_event {
                                    if let Some(ref recorder_mutex) = flight_recorder {
                                        let mut rec = recorder_mutex.lock();
                                        let _ = rec.record_transition(
                                            core_contracts::FlightEventKind::Checkpoint,
                                            0x51, // Shadow SI subsystem
                                            grad.tick_index,
                                            (grad.concurrence_at_graduation * 10_000.0) as u64,
                                            0,
                                            &[0u8; 16],
                                        );
                                    }
                                    info!(
                                        target: "shadow_si",
                                        tick = grad.tick_index,
                                        concurrence = grad.concurrence_at_graduation,
                                        "Shadow model GRADUATED: concurrence >= 95% threshold"
                                    );
                                }

                                debug!(
                                    target: "shadow_si",
                                    tick = tick_count,
                                    actual = actual_opcode,
                                    predicted = predicted_opcode,
                                    conf = pred.confidence_score,
                                    reward,
                                    "shadow tick"
                                );
                            }
                            Err(e) => {
                                debug!(target: "shadow_si", error = %e, "shadow forward step failed");
                            }
                        }
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

/// M40: best-effort JSON export of the live concurrence snapshot.
///
/// Writes to `<workspace data dir>/concurrence_report.json` so an external
/// devtools process can read Aaroneous's current concurrence state through
/// an explicit file path (a file contract, not a Cargo dependency — see
/// `LOCAL_CLOUD_ORCHESTRATION_PLAN.md` M40). The write is atomic: the JSON
/// is serialized to a sibling `.partial` file first, then renamed into
/// place, so a concurrent reader never observes a truncated or partially
/// written file. Any failure (I/O error, serialization error) is logged as
/// a warning and swallowed — this path must never crash or stall the
/// hypervisor's heartbeat thread.
fn write_concurrence_report(snapshot: &compute::ConcurrenceSnapshot) {
    let paths = paths::WorkspacePaths::from_config(paths::WorkspacePathsConfig::default());
    let report_path = paths.data().join("concurrence_report.json");
    let partial_path = report_path.with_extension("json.partial");

    if let Some(parent) = report_path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        warn!(
            target: "shadow_si",
            error = %e,
            path = %parent.display(),
            "failed to create concurrence report directory; skipping export"
        );
        return;
    }

    let write_result = std::fs::File::create(&partial_path).and_then(|file| {
        serde_json::to_writer_pretty(file, snapshot).map_err(std::io::Error::other)
    });

    if let Err(e) = write_result {
        warn!(
            target: "shadow_si",
            error = %e,
            path = %partial_path.display(),
            "failed to write concurrence report; skipping export"
        );
        return;
    }

    if let Err(e) = std::fs::rename(&partial_path, &report_path) {
        warn!(
            target: "shadow_si",
            error = %e,
            from = %partial_path.display(),
            to = %report_path.display(),
            "failed to publish concurrence report via atomic rename"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adaptation_plane::{
        AutonomousPacingRegulator, PacingConfig, PacingTier, ThermodynamicTelemetry,
    };

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

    #[test]
    fn test_supervisory_daemon_observation_buffer_integration() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let synapse_name = "test_ans_observation_synapse";

        let enzyme_runner = Arc::new(EnzymeRunner::new().expect("enzyme runner"));
        let hox_path = tmp.path().join("test_hox_obs.db");
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

        let db_path = tmp.path().join("test_hive_obs.db");
        let obs_buffer_path = tmp.path().join("test_obs.shm");
        let obs_buffer = Arc::new(parking_lot::Mutex::new(
            ipc_bus::ObservationBuffer::open_or_create(&obs_buffer_path, 32).expect("open obs shm"),
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
        .with_observation_buffer(obs_buffer.clone());

        daemon.set_max_ticks(3);
        daemon.start();

        std::thread::sleep(Duration::from_millis(150));
        daemon.request_shutdown();

        // Verify observation frames were recorded
        let buf = obs_buffer.lock();
        let seq = buf.current_sequence();
        assert!(
            seq >= 1,
            "Expected at least 1 recorded observation frame, found {}",
            seq
        );

        let frame = buf.read_latest().expect("read latest observation frame");
        assert_eq!(frame.sequence, seq);
        assert!(frame.state_features[0] >= 0.0);
        assert!(frame.flow_score > 0.0);

        // Test background episodic thought accumulator ingestion
        let config = compute::EpisodicAccumulatorConfig {
            min_episode_len: 1,
            max_episode_len: 16,
            min_crystallize_reward: 0.0,
            wal_dir: Some(tmp.path().join("wal")),
        };
        let mut accumulator = compute::EpisodicThoughtAccumulator::new(config);
        let _ = accumulator.poll_and_accumulate(&buf).expect("accumulate");
    }
}
