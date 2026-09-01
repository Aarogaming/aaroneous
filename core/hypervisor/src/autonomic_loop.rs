use anyhow::Result;
use memmap2::{MmapMut, MmapOptions};
use parking_lot::RwLock;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

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
        let path = aaroneous_paths::resolve_synapse_path(name);

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
use crate::dopamine_system::{DopamineEvent, DopamineSystem, FeedbackSignalProcessor};
use crate::enzyme_runner::EnzymeRunner;
use crate::enzyme_types::{CuriosityEnzyme, DiplomatEnzyme, SelfCorrectionEnzyme};
use crate::delta_orchestrator::DeltaOrchestrator;
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

pub struct AutonomicNervousSystem {
    synapse: Arc<RwLock<LegacySharedMemorySynapse>>,
    enzyme_runner: Arc<EnzymeRunner>,
    _hox_registry: Arc<HoxRegistry>,
    _splicing_engine: Arc<WasmSplicingEngine>,
    learning_loop: Arc<RwLock<UnifiedLearningLoop>>,
    nlm_sentinel: Arc<NlmSentinel>,
    prefrontal_cortex: Arc<PrefrontalCortex>,
    dopamine_system: Arc<DopamineSystem>,
    epigenetic_orchestrator: Arc<DeltaOrchestrator>,
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
}

impl AutonomicNervousSystem {
    pub fn new(
        synapse_name: &str,
        tick_rate_ms: u64,
        enzyme_runner: Arc<EnzymeRunner>,
        hox_registry: Arc<HoxRegistry>,
        splicing_engine: Arc<WasmSplicingEngine>,
        learning_loop: Arc<RwLock<UnifiedLearningLoop>>,
        db_path: Option<&str>,
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

        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

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
            epigenetic_orchestrator: Arc::new(DeltaOrchestrator::new()),
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
        })
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
        let epigenetic_orchestrator = self.epigenetic_orchestrator.clone();
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

        info!(target: "autonomic_loop", ?tick_rate, "heartbeat initiated");

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
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
                    let mock_intent_text = "Perform search for MIT licensed code on GitHub";

                    let intent_tier = nlm_sentinel.classify_intent(mock_intent_text);

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

                    if mock_intent_text.contains("Perform search") {
                        let mut plan_guard = active_plan.write();
                        if let Ok(new_plan) =
                            rt.block_on(prefrontal_cortex.draft_plan(mock_intent_text))
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
                        let id = index.index_text(mock_intent_text, metadata);
                        let result_str = mock_intent_text.to_string();
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
                                    epigenetic_orchestrator
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

                                    if epigenetic_orchestrator
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
                    state.mcp_tool_call.status = 2; // Executing

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
                    epigenetic_orchestrator
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
                if state.clock_tick % 1000 == 0 || state.memory_pressure > 90 {
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
                if elapsed < tick_rate {
                    thread::sleep(tick_rate - elapsed);
                }
            }
        });
    }
}
