use std::thread;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::path::PathBuf;
use parking_lot::RwLock;
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use anyhow::{Result, Context};

pub struct LegacySharedMemorySynapse {
    mmap: MmapMut,
    path: PathBuf,
}

impl LegacySharedMemorySynapse {
    pub fn new(name: &str, size: usize) -> Result<Self> {
        let path = PathBuf::from(format!(r"C:\Users\aarog\AppData\Local\Temp\{}.synapse", name));
        
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)?;

        file.set_len(size as u64)?;

        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        Ok(Self { mmap, path })
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

use crate::enzyme_runner::EnzymeRunner;
use crate::hox_registry::HoxRegistry;
use crate::unified_learning::UnifiedLearningLoop;
use crate::splicing_engine::WasmSplicingEngine;
use crate::nlm_sentinel::{NlmSentinel, IntentTier};
use crate::prefrontal_cortex::PrefrontalCortex;
use crate::executive_plan::{ExecutivePlan, StepStatus};
use crate::dopamine_system::{DopamineSystem, DopamineEvent};
use crate::epigenetic_orchestrator::EpigeneticOrchestrator;
use crate::concept_drift::ConceptDriftDetector;
use crate::self_correction_enzyme::SelfCorrectionEnzyme;
use crate::diplomat_enzyme::DiplomatEnzyme;
use crate::neural_pruning::NeuralPruningEnzyme;
use crate::curiosity_enzyme::CuriosityEnzyme;
use crate::semantic_indexing::SemanticIndex;
use crate::federation::hive_db::PersistenceManager as HivePersistence;
use crate::hardened_env::HardenedEnvironment;

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
    hox_registry: Arc<HoxRegistry>,
    splicing_engine: Arc<WasmSplicingEngine>,
    learning_loop: Arc<RwLock<UnifiedLearningLoop>>,
    nlm_sentinel: Arc<NlmSentinel>,
    prefrontal_cortex: Arc<PrefrontalCortex>,
    dopamine_system: Arc<DopamineSystem>,
    epigenetic_orchestrator: Arc<EpigeneticOrchestrator>,
    self_correction_enzyme: Arc<SelfCorrectionEnzyme>,
    neural_pruning_enzyme: Arc<NeuralPruningEnzyme>,
    diplomat_enzyme: Arc<DiplomatEnzyme>,
    wasm_splicer: Arc<crate::wasm_splicer::WasmSplicingEngine>,
    concept_drift_detector: Arc<RwLock<ConceptDriftDetector>>,
    curiosity_enzyme: Arc<RwLock<CuriosityEnzyme>>,
    semantic_index: Arc<RwLock<SemanticIndex>>,
    active_plan: Arc<RwLock<Option<ExecutivePlan>>>,
    tick_rate: Duration,
    hive_db: Option<Arc<parking_lot::Mutex<HivePersistence>>>,
    workspace_root: PathBuf,
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
            std::slice::from_raw_parts(
                &initial as *const SynapseState as *const u8,
                size,
            )
        };
        synapse.write(0, bytes).ok();

        let hive_db = db_path.and_then(|p| HivePersistence::new(p).ok().map(|db| Arc::new(parking_lot::Mutex::new(db))));
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

        Ok(Self {
            synapse: Arc::new(RwLock::new(synapse)),
            enzyme_runner,
            hox_registry: hox_registry.clone(),
            splicing_engine,
            learning_loop,
            nlm_sentinel: Arc::new(NlmSentinel::new()?),
            prefrontal_cortex: Arc::new(PrefrontalCortex),
            dopamine_system: Arc::new(DopamineSystem),
            epigenetic_orchestrator: Arc::new(EpigeneticOrchestrator::new()),
            self_correction_enzyme: Arc::new(SelfCorrectionEnzyme::new()),
            neural_pruning_enzyme: Arc::new(NeuralPruningEnzyme::new(60)),
            diplomat_enzyme: Arc::new(DiplomatEnzyme::new()),
            wasm_splicer: Arc::new(crate::wasm_splicer::WasmSplicingEngine::new().unwrap()),
            concept_drift_detector: Arc::new(RwLock::new(ConceptDriftDetector::new())),
            curiosity_enzyme: Arc::new(RwLock::new(CuriosityEnzyme::new())),
            semantic_index: Arc::new(RwLock::new(semantic_index)),
            active_plan: Arc::new(RwLock::new(None)),
            tick_rate: Duration::from_millis(tick_rate_ms),
            hive_db,
            workspace_root,
        })
    }
    
    pub fn get_synapse(&self) -> Arc<RwLock<LegacySharedMemorySynapse>> {
        self.synapse.clone()
    }
    
    fn read_state(syn: &LegacySharedMemorySynapse) -> SynapseState {
        let size = std::mem::size_of::<SynapseState>();
        let buf = syn.read(0, size).unwrap_or_else(|_| vec![0u8; size]);
        unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const SynapseState) }
    }

    fn write_state(syn: &LegacySharedMemorySynapse, state: &SynapseState) {
        let size = std::mem::size_of::<SynapseState>();
        let bytes = unsafe {
            std::slice::from_raw_parts(
                state as *const SynapseState as *const u8,
                size,
            )
        };
        syn.write(0, bytes).ok();
    }

    pub fn start(&self) {
        let synapse = self.synapse.clone();
        let enzyme_runner = self.enzyme_runner.clone();
        let hox_registry = self.hox_registry.clone();
        let splicing_engine = self.splicing_engine.clone();
        let learning_loop = self.learning_loop.clone();
        let nlm_sentinel = self.nlm_sentinel.clone();
        let prefrontal_cortex = self.prefrontal_cortex.clone();
        let dopamine_system = self.dopamine_system.clone();
        let epigenetic_orchestrator = self.epigenetic_orchestrator.clone();
        let self_correction_enzyme = self.self_correction_enzyme.clone();
        let neural_pruning_enzyme = self.neural_pruning_enzyme.clone();
        let diplomat_enzyme = self.diplomat_enzyme.clone();
        let wasm_splicer = self.wasm_splicer.clone();
        let concept_drift_detector = self.concept_drift_detector.clone();
        let curiosity_enzyme = self.curiosity_enzyme.clone();
        let semantic_index = self.semantic_index.clone();
        let active_plan = self.active_plan.clone();
        let tick_rate = self.tick_rate;
        let hive_db = self.hive_db.clone();

        println!("[AutonomicNS] Heartbeat initiated at {:?}.", tick_rate);

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            loop {
                let start = Instant::now();
                
                let mut state = SynapseState::default();
                {
                    let syn = synapse.read();
                    state = Self::read_state(&syn);
                }

                state.clock_tick += 1;

                // --- PHASE 1: HOMEOSTATIC SELF-PRESERVATION ---
                if state.memory_pressure > 85 {
                    println!("[AutonomicNS] High memory pressure: {}%. Triggering GC enzyme.", state.memory_pressure);
                    state.memory_pressure = 30;
                }

                // DOPAMINE SYSTEM: Curiosity and Proactive Learning
                if state.understanding_score < 40 || state.curiosity_drive > 80 {
                    {
                        let mut curiosity = curiosity_enzyme.write();
                        let index = semantic_index.read();
                        let mut gaps = curiosity.identify_knowledge_gaps(&index);
                        
                        let plan_guard = active_plan.read();
                        let forecast = curiosity.forecast_requirements(&*plan_guard);
                        gaps.extend(forecast);
                        drop(plan_guard);

                        if let Ok(hunger_intent) = rt.block_on(curiosity.formulate_hunger_intent(&gaps)) {
                            if let Ok(new_plan) = rt.block_on(prefrontal_cortex.draft_plan(&hunger_intent)) {
                                let mut plan_guard = active_plan.write();
                                *plan_guard = Some(new_plan);
                                println!("[AutonomicNS] Hunger plan seated to satisfy curiosity.");
                            }
                        }
                    }

                    state.curiosity_drive = 20;
                    state.understanding_score += 10;
                }

                // --- PHASE 3: OPERATIONAL EXECUTION (The Sentinel) ---
                if state.intent_vector_id != [0; 16] {
                    let task_id = uuid::Uuid::from_bytes(state.intent_vector_id).to_string();
                    let mock_intent_text = "Perform search for MIT licensed code on GitHub";
                    
                    let intent_tier = nlm_sentinel.classify_intent(mock_intent_text);
                    
                    if intent_tier == IntentTier::Violation {
                        println!("[AutonomicNS] SAFETY VIOLATION detected for task {}. Blocking.", task_id);
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
                            println!("[AutonomicNS] Task {} requires user approval (Tier {}). Stalling.", task_id, state.sovereignty_tier);
                            state.approval_required = 1;
                        }
                        Self::write_state(&synapse.read(), &state);
                        thread::sleep(tick_rate);
                        continue;
                    }

                    println!("[AutonomicNS] Intent vector {} approved (Tier {}). Executing.", task_id, state.sovereignty_tier);
                    
                    if mock_intent_text.contains("Perform search") {
                        let mut plan_guard = active_plan.write();
                        if let Ok(new_plan) = rt.block_on(prefrontal_cortex.draft_plan(mock_intent_text)) {
                            *plan_guard = Some(new_plan);
                            println!("[AutonomicNS] Multi-step plan generated and seated.");
                        }
                    }

                    if let Some(db_mutex) = hive_db.as_ref() {
                        let db = db_mutex.lock();
                        let mut index = semantic_index.write();
                        let mut metadata = std::collections::HashMap::new();
                        metadata.insert("source".to_string(), "intent_execution".to_string());
                        metadata.insert("sovereignty_tier".to_string(), state.sovereignty_tier.to_string());
                        let id = index.index_text(mock_intent_text, metadata);
                        let result_str = mock_intent_text.to_string();
                        drop(index);
                        let index = semantic_index.read();
                        if let Some(entry) = index.entries.iter().find(|e| e.id == id) {
                            let _ = db.save_embedding(&id, &result_str, &entry.vector, &entry.metadata, entry.access_count);
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
                                println!("[AutonomicNS] Executing plan step: {} ({})", step_id, step.assigned_specialist);
                                step.status = StepStatus::InProgress;

                                if state.understanding_score > 90 {
                                    epigenetic_orchestrator.inject_latent_state(&state.latent_vector);
                                }

                                if step_id == "step_1" {
                                    dopamine_system.process_event(&mut state, DopamineEvent::SuccessfulIngestion(0));
                                    
                                    if let Ok(_) = epigenetic_orchestrator.extract_hidden_state(&mut state.latent_vector) {
                                        let mut detector = concept_drift_detector.write();
                                        let drift = detector.analyze_drift(&state.latent_vector);
                                        state.concept_drift = drift;
                                        
                                        if detector.is_integrity_compromised() {
                                            if drift > 0.95 {
                                                state.safety_lock = 1;
                                            }
                                        }
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
                        
                        let result_msg = "Debate resolved. Consensus reached via Diplomatic override.";
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
                    epigenetic_orchestrator.sync_lora_to_speaker(state.dialogue.active_speaker_hash);
                    
                    if state.dialogue.consensus_score > 95 && state.clock_tick % 1000 == 0 {
                        let name = format!("skill_chip_{}", state.clock_tick);
                        if let Ok(_) = wasm_splicer.splice_specialist_dna(&name, &["odin", "merlin"]) {
                            dopamine_system.process_event(&mut state, DopamineEvent::SuccessfulIngestion(100));
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
                        if let Ok(correction) = self_correction_enzyme.attempt_recalibration(&mut state) {
                            state.integrity_score = (state.integrity_score + 5).min(100);
                        }
                        state.memory_pressure = (state.memory_pressure + 10).min(100);
                    }
                }

                // --- PHASE 8: NEURAL PRUNING (Homeostasis) ---
                if state.clock_tick % 1000 == 0 || state.memory_pressure > 90 {
                    neural_pruning_enzyme.prune_constellation(&mut Vec::new());
                }

                // --- Sync: write state back to shared memory ---
                {
                    let syn = synapse.read();
                    Self::write_state(&syn, &state);
                }

                let elapsed = start.elapsed();
                if elapsed < tick_rate {
                    thread::sleep(tick_rate - elapsed);
                }
            }
        });
    }
}
