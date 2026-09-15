pub mod automata;
pub mod bayesian;
pub mod burn_gpu;
pub mod category;
pub mod cognitive_equilibrium;
pub mod control;
pub mod cranelift_jit;
pub mod crucible;
pub mod denormal;
pub mod dynamics;
pub mod entropy;
pub mod entropy_metrics;
pub mod episodic_memory;
pub mod ffi_kernels;
pub mod game_theory;
pub mod graph;
pub mod hippo;
pub mod information;
pub mod isolated_desktop;
pub mod kalman;
pub mod latent_guardrail;
pub mod latent_router;
pub mod linalg;
pub mod machine_native;
pub mod macro_ssm;
pub mod mdps;
pub mod mpc;
pub mod multimodal_ssm;
pub mod optimize;
pub mod predictive_coding;
pub mod reflex_worker;
pub mod si_binary;
pub mod si_decoder;
pub mod si_distillation_harness;
pub mod si_forge;
pub mod si_jit;
pub mod si_macro;
pub mod si_model;
pub mod si_moe_register;
pub mod si_motor_tree;
pub mod si_packer;
pub mod si_self_play;
pub mod si_skill_tree;
pub mod si_solid_state;
pub mod si_spec;
pub mod si_ssm;
pub mod si_tool;
pub mod si_trainer;
pub mod signal;
pub mod silicon_backend;
pub mod state_bank;
pub mod stochastic;
pub mod tensor_buffer;
pub mod tensor_kernel;
pub use entropy_metrics as thermodynamics;
pub mod token_consumer;
pub mod topology;
pub mod translation_dataset;
pub mod user_baseline;
pub mod wx_memory;

pub use user_baseline::{AttentionState, KinematicBiomarkers, UserIdentityEngine, UserProfile};

pub use tensor_buffer::{TensorBuffer, UniversalTensorView};

pub use silicon_backend::{
    CpuSimdBackend, DynamicSiliconRouter, NpuTensorBackend, SiliconHardwareType,
    SiliconTelemetryReport, UniversalTensorBackend,
};
pub use state_bank::{
    STATE_BANK_MAGIC, STATE_BANK_VERSION, StateBankHeader, StateBankRecord, UniversalStateBank,
};

pub use cognitive_equilibrium::{
    AttentionSpectrum, CognitiveEquilibriumCoordinator, SomaticVitals, TriModalDecisionReport,
};
pub use cranelift_jit::{CraneliftJitEngine, NativeExecutionFn};
pub use crucible::{CrucibleDuelReport, CrucibleSandbox, VirtualScenario};
pub use denormal::{DenormalGuard, denormal_flush_scope, with_denormals_flushed};
pub use dynamics::{
    ComputeError, DynamicalSystem, EffortFlowPair, HarmonicOscillator, HarmonicOscillatorDual,
    PhysicalDomain,
};
pub use episodic_memory::{
    AcousticReflexMatcher, EpisodicMemoryFabric, LATENT_VECTOR_DIM, SearchResult,
    TrajectoryMetadata, simd_cosine_similarity_256, simd_dot_product_256,
};
pub use hippo::{
    HippoLegendreMatrices, discretize_bilinear, generate_hippo_discretized, generate_hippo_legendre,
};
pub use macro_ssm::{ContinuousMacroSsm, MACRO_LATENT_DIM, MACRO_STATE_DIM, MacroSsmConfig};
pub use si_moe_register::{
    CartridgeDescriptor, DEFAULT_MAX_EXPERT_SLOTS, DEFAULT_MAX_ORGAN_SLOTS, ExpertSlot,
    MoEExecutionReport, OrganDescriptor, OrganSlot, SiMoERegister,
};
pub use wx_memory::WxMemoryRegion;

pub use burn_gpu::{GpuTensorAccelerator, GpuTensorProfile};
pub use isolated_desktop::IsolatedDesktop;
pub use latent_guardrail::{
    ArgusSafetySentinel, GUARDRAIL_DIM, LatentAuditVerdict, SafeHypersphereManifold,
};
pub use latent_router::{CORTEX_INTENT_DIM, LatentOrthogonalRouter, SUBGOAL_DIM};
pub use machine_native::{
    DimensionalUnit, EdgeLinguisticLens, MachineNativePredictionEngine, MachineOpcode,
    NativeComputationNode, NativeComputationalGraph, NativeTypeLattice,
};
pub use multimodal_ssm::{
    AcousticIntentProjector, MULTIMODAL_LATENT_DIM, MultimodalSensoryFrame, PixelDiffProjector,
    TemporalModalitySynchronizer,
};
pub use reflex_worker::ReflexWorker;
pub use si_binary::{SI_MAGIC_BYTES, SiCorpusStore, SiThoughtHeader, SiThoughtPacket};
pub use si_decoder::{ActionDecoder, DECODER_INTENT_DIM, DecodedActionCommand};
pub use si_distillation_harness::{BootstrapConfig, BootstrapReport, SiDistillationHarness};
pub use si_forge::SiForge;
pub use si_jit::{
    CompiledReflexHandle, CrystallizationMetrics, JIT_INTENT_DIM, MemoryProtectionState,
    NativeExecutionContext, SiJitCompilerEngine,
};
pub use si_macro::{SiMacroEngine, SiMacroMetadata};
pub use si_model::{
    SI_MODEL_MAGIC, SI_OPCODE_VOCAB_SIZE, SiGraphLayer, SiModel, SiModelConfig, SiModelPrediction,
};
pub use si_motor_tree::{MOTOR_INTENT_DIM, MotorCortex, MotorSkillNode, SkillType, StarState};
pub use si_packer::{
    ALIGNMENT_BYTES, SINT_PACKER_MAGIC, SINT_PACKER_VERSION, SiContainerManifest, SiPacker,
    SiSolidStateLoader, SiTierFlags, TensorDescriptor, compute_padding,
};
pub use si_self_play::{AsymmetricDuelReport, DreamGoal, SelfPlayStepResult, SiSelfPlayEngine};
pub use si_skill_tree::{SiSkillModule, SkillExpansionEngine, SkillMaturityStatus};
pub use si_solid_state::{
    DynamicAdaptationMatrix, OnlineCorrectionReport, SI_SOLID_STATE_MAGIC, SI_SOLID_STATE_VERSION,
    SafetyCheckResult, SiOnlineLearner, SolidStateSiContainer,
};
pub use si_spec::{
    SI_CANONICAL_MAGIC, SI_CANONICAL_VERSION, SI_FLAG_TIER_1_CORTEX, SI_FLAG_TIER_2_ROUTER,
    SI_FLAG_TIER_3_REFLEX, SI_HEADER_SIZE, SiCartridgeDeconstructed, SiCartridgeDiffReport,
    SiCartridgeEngine, SiCartridgeHeader, SiCartridgeReport, compute_crc32,
};
pub use si_ssm::{
    SI_SSM_MAGIC, SI_SSM_VERSION, SiSsmConfig, SiStateSpaceModel, SsmLayerBlock,
    SsmStatePrediction, TreeSsmNode,
};
pub use si_tool::{SiBenchmarkReport, SiInspectorReport, SiToolEngine};
pub use si_trainer::{
    LatentGELUBottleneckBridge, SiModelTrainer, SiTrainerConfig, TrainingEpochReport, gelu,
    gelu_prime,
};
pub use state_bank::{AdaptationError, RlsState, STATE_BANK_HEADER_SIZE, update_rls};
pub use token_consumer::MachineToken;
pub use translation_dataset::{
    ROSETTA_LATENT_DIM, ROSETTA_TEACHER_DIM, RosettaTrajectoryStep, TranslationDataset,
};
pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;
use ipc_bus::SharedMemorySynapse;
use rand::SeedableRng;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

/// Test isolation guard for parallel test execution.
#[cfg(test)]
static TEST_ISOLATE_MUTEX: Mutex<()> = Mutex::new(());

/// Acquire exclusive test lock for deterministic parallel execution
#[cfg(test)]
fn acquire_test_lock() -> MutexGuard<'static, ()> {
    TEST_ISOLATE_MUTEX.lock().unwrap()
}

/// The central Compute Engine.
/// Exposes mathematical methodologies to the Synapse for zero-copy execution.
pub struct ComputeEngine {
    pub synapse: SharedMemorySynapse,
    pub rng: rand::rngs::StdRng,
}

impl Default for ComputeEngine {
    fn default() -> Self {
        // For tests, use unique synapse names to prevent file locking conflicts
        #[cfg(test)]
        {
            use std::sync::atomic::{AtomicUsize, Ordering};

            static COUNTER: AtomicUsize = AtomicUsize::new(0);
            let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
            let synapse_name = format!("TEST_SYNAPSE_{}", counter);
            Self::new(SharedMemorySynapse::new_sync(&synapse_name, 1024 * 1024).unwrap())
        }

        #[cfg(not(test))]
        {
            match SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024) {
                Ok(synapse) => Self::new(synapse),
                Err(_) => {
                    use std::sync::atomic::{AtomicUsize, Ordering};
                    static COUNTER: AtomicUsize = AtomicUsize::new(0);
                    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);
                    let fallback_name =
                        format!("SAB_STORE_FALLBACK_{}_{}", std::process::id(), counter);
                    let synapse = SharedMemorySynapse::new_sync(&fallback_name, 1024 * 1024)
                        .expect("Failed to initialize fallback compute engine synapse");
                    Self::new(synapse)
                }
            }
        }
    }
}

impl ComputeEngine {
    pub fn new(synapse: SharedMemorySynapse) -> Self {
        Self {
            synapse,
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }

    // Unified execution interface
    pub fn execute(&mut self, task: &str, input: &[f64]) -> anyhow::Result<Vec<f64>> {
        match task {
            "monte_carlo" => stochastic::monte_carlo_simulate(input, 1000, &mut self.rng),
            "markov" => mdps::markov_transition(input, &mut self.rng),
            "bayesian" => bayesian::bayesian_update(input),
            "entropy" => entropy::shannon_entropy(input),
            "cosine" => linalg::cosine_similarity(input),
            "pid" => control::pid_step(input),
            "fft" => signal::fft_industrial(input),
            "nash" => game_theory::nash_approx(input),
            "optimize_ga" => optimize::genetic_step(input, &mut self.rng),
            "boltzmann" => {
                let _n = input.len() - 1;
                let temperature = input[0];
                let energies = &input[1..];
                Ok(thermodynamics::boltzmann_distribution(
                    energies,
                    temperature,
                ))
            }
            "free_energy" => {
                if input.len() >= 3 {
                    Ok(vec![
                        thermodynamics::FreeEnergyState::new(input[0], input[1], input[2])
                            .free_energy,
                    ])
                } else {
                    Ok(vec![0.0])
                }
            }
            "mutual_info" => {
                if input.len() >= 3 {
                    Ok(vec![information::mutual_information(
                        &[
                            vec![input[0], input[1]],
                            vec![input[2], 1.0 - input[0] - input[1] - input[2]],
                        ],
                        &[
                            input[0] + input[1],
                            input[2] + (1.0 - input[0] - input[1] - input[2]),
                        ],
                        &[
                            input[0] + input[2],
                            input[1] + (1.0 - input[0] - input[1] - input[2]),
                        ],
                    )])
                } else {
                    Ok(vec![0.0])
                }
            }
            _ => anyhow::bail!("Unknown compute task: {}", task),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_engine_new() {
        let engine = ComputeEngine::default();
        drop(engine);
    }

    #[test]
    fn test_compute_engine_default() {
        let engine = ComputeEngine::default();
        drop(engine);
    }

    #[test]
    fn test_execute_monte_carlo() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.5, 0.3];
        let result = engine.execute("monte_carlo", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_execute_markov() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.7, 0.3];
        let result = engine.execute("markov", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_bayesian() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.5, 0.3, 0.2];
        let result = engine.execute("bayesian", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 4);
    }

    #[test]
    fn test_execute_entropy() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.25, 0.25, 0.25, 0.25];
        let result = engine.execute("entropy", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_execute_cosine() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![1.0, 0.0, 0.0, 1.0];
        let result = engine.execute("cosine", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_pid() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![1.0, 0.5, 0.1];
        let result = engine.execute("pid", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_fft() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![1.0, 0.0, 0.0, 0.0];
        let result = engine.execute("fft", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_nash() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.5, 0.5, 0.5];
        let result = engine.execute("nash", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_optimize_ga() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = engine.execute("optimize_ga", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_boltzmann() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![1.0, -0.5, 0.3, -0.8];
        let result = engine.execute("boltzmann", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_execute_free_energy() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.5, 0.3, 0.2];
        let result = engine.execute("free_energy", &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_execute_free_energy_insufficient_input() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.5, 0.3];
        let result = engine.execute("free_energy", &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0.0]);
    }

    #[test]
    fn test_execute_mutual_info() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.3, 0.2, 0.1];
        let result = engine.execute("mutual_info", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_mutual_info_insufficient() {
        let _test_lock = acquire_test_lock();
        let mut engine = ComputeEngine::default();
        let input = vec![0.3, 0.2];
        let result = engine.execute("mutual_info", &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0.0]);
    }

    #[test]
    fn test_execute_unknown_task() {
        let mut engine = ComputeEngine::default();
        let input = vec![1.0];
        let result = engine.execute("nonexistent_task", &input);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown compute task")
        );
    }
}
