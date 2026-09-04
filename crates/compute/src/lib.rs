pub mod automata;
pub mod bayesian;
pub mod burn_gpu;
pub mod category;
pub mod control;
pub mod entropy;
pub mod game_theory;
pub mod graph;
pub mod information;
pub mod kalman;
pub mod linalg;
pub mod machine_native;
pub mod mdps;
pub mod mpc;
pub mod optimize;
pub mod predictive_coding;
pub mod signal;
pub mod stochastic;
pub mod thermodynamics;
pub mod topology;
pub mod si_binary;
pub mod si_model;
pub mod si_trainer;
pub mod si_macro;
pub mod si_ssm;
pub mod si_skill_tree;
pub mod si_tool;
pub mod si_solid_state;
pub mod latent_guardrail;
pub mod si_self_play;
pub mod si_jit;
pub mod multimodal_ssm;
pub mod translation_dataset;
pub mod si_distillation_harness;
pub mod si_packer;
pub mod si_forge;
pub mod latent_router;
pub mod reflex_worker;
pub mod si_decoder;
pub mod si_motor_tree;
pub mod isolated_desktop;
pub mod si_spec;
pub mod wx_memory;
pub mod cranelift_jit;
pub mod cognitive_equilibrium;
pub mod crucible;
pub mod ffi_kernels;
pub mod episodic_memory;
pub mod hippo;
pub mod macro_ssm;
pub mod si_moe_register;
pub mod silicon_backend;
pub mod state_bank;
pub mod tensor_buffer;

pub use tensor_buffer::{TensorBuffer, UniversalTensorView};

pub use silicon_backend::{
    CpuSimdBackend, DynamicSiliconRouter, NpuTensorBackend, SiliconHardwareType,
    SiliconTelemetryReport, UniversalTensorBackend,
};
pub use state_bank::{
    StateBankHeader, StateBankRecord, UniversalStateBank, STATE_BANK_MAGIC, STATE_BANK_VERSION,
};

pub use cognitive_equilibrium::{
    AttentionSpectrum, CognitiveEquilibriumCoordinator, SomaticVitals, TriModalDecisionReport,
};
pub use crucible::{CrucibleDuelReport, CrucibleSandbox, VirtualScenario};
pub use wx_memory::WxMemoryRegion;
pub use cranelift_jit::{CraneliftJitEngine, NativeExecutionFn};
pub use episodic_memory::{
    simd_cosine_similarity_256, simd_dot_product_256, AcousticReflexMatcher, EpisodicMemoryFabric,
    SearchResult, TrajectoryMetadata, LATENT_VECTOR_DIM,
};
pub use hippo::{
    generate_hippo_discretized, generate_hippo_legendre, discretize_bilinear,
    HippoLegendreMatrices,
};
pub use macro_ssm::{ContinuousMacroSsm, MacroSsmConfig, MACRO_LATENT_DIM, MACRO_STATE_DIM};
pub use si_moe_register::{
    CartridgeDescriptor, ExpertSlot, MoEExecutionReport, OrganDescriptor, OrganSlot, SiMoERegister,
    DEFAULT_MAX_EXPERT_SLOTS, DEFAULT_MAX_ORGAN_SLOTS,
};

pub use burn_gpu::{GpuTensorAccelerator, GpuTensorProfile};
pub use si_spec::{
    compute_crc32, SiCartridgeDeconstructed, SiCartridgeDiffReport, SiCartridgeEngine,
    SiCartridgeHeader, SiCartridgeReport, SI_CANONICAL_MAGIC, SI_CANONICAL_VERSION,
    SI_FLAG_TIER_1_CORTEX, SI_FLAG_TIER_2_ROUTER, SI_FLAG_TIER_3_REFLEX, SI_HEADER_SIZE,
};
pub use machine_native::{
    DimensionalUnit, EdgeLinguisticLens, MachineNativePredictionEngine, MachineOpcode,
    NativeComputationNode, NativeComputationalGraph, NativeTypeLattice,
};
pub use si_binary::{SiCorpusStore, SiThoughtHeader, SiThoughtPacket, SI_MAGIC_BYTES};
pub use si_model::{SiGraphLayer, SiModel, SiModelConfig, SiModelPrediction, SI_MODEL_MAGIC, SI_OPCODE_VOCAB_SIZE};
pub use si_trainer::{gelu, gelu_prime, LatentGELUBottleneckBridge, SiModelTrainer, SiTrainerConfig, TrainingEpochReport};
pub use si_macro::{SiMacroEngine, SiMacroMetadata};
pub use si_ssm::{SiSsmConfig, SiStateSpaceModel, SsmLayerBlock, SsmStatePrediction, TreeSsmNode, SI_SSM_MAGIC, SI_SSM_VERSION};
pub use si_skill_tree::{SiSkillModule, SkillExpansionEngine, SkillMaturityStatus};
pub use si_tool::{SiBenchmarkReport, SiInspectorReport, SiToolEngine};
pub use si_solid_state::{
    DynamicAdaptationMatrix, OnlineCorrectionReport, SafetyCheckResult, SiOnlineLearner, SolidStateSiContainer,
    SI_SOLID_STATE_MAGIC, SI_SOLID_STATE_VERSION,
};
pub use latent_guardrail::{ArgusSafetySentinel, LatentAuditVerdict, SafeHypersphereManifold, GUARDRAIL_DIM};
pub use si_self_play::{AsymmetricDuelReport, DreamGoal, SelfPlayStepResult, SiSelfPlayEngine};
pub use si_jit::{CompiledReflexHandle, CrystallizationMetrics, MemoryProtectionState, NativeExecutionContext, SiJitCompilerEngine, JIT_INTENT_DIM};
pub use multimodal_ssm::{AcousticIntentProjector, MultimodalSensoryFrame, PixelDiffProjector, TemporalModalitySynchronizer, MULTIMODAL_LATENT_DIM};
pub use translation_dataset::{TranslationDataset, RosettaTrajectoryStep, ROSETTA_LATENT_DIM, ROSETTA_TEACHER_DIM};
pub use si_distillation_harness::{BootstrapConfig, BootstrapReport, SiDistillationHarness};
pub use si_packer::{
    compute_padding, SiContainerManifest, SiPacker, SiSolidStateLoader, SiTierFlags,
    TensorDescriptor, ALIGNMENT_BYTES, SINT_PACKER_MAGIC, SINT_PACKER_VERSION,
};
pub use si_forge::SiForge;
pub use latent_router::{LatentOrthogonalRouter, CORTEX_INTENT_DIM, SUBGOAL_DIM};
pub use reflex_worker::ReflexWorker;
pub use si_decoder::{ActionDecoder, DecodedActionCommand, DECODER_INTENT_DIM};
pub use si_motor_tree::{MotorCortex, MotorSkillNode, SkillType, StarState, MOTOR_INTENT_DIM};
pub use isolated_desktop::IsolatedDesktop;
pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;
use ipc_bus::SharedMemorySynapse;
use rand::SeedableRng;

/// The central Compute Engine.
/// Exposes mathematical methodologies to the Synapse for zero-copy execution.
pub struct ComputeEngine {
    pub synapse: SharedMemorySynapse,
    pub rng: rand::rngs::StdRng,
}

impl Default for ComputeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeEngine {
    pub fn new() -> Self {
        Self {
            synapse: SharedMemorySynapse::new_sync("SAB_STORE", 1024 * 1024).unwrap(),
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
        let engine = ComputeEngine::new();
        drop(engine);
    }

    #[test]
    fn test_compute_engine_default() {
        let engine = ComputeEngine::default();
        drop(engine);
    }

    #[test]
    fn test_execute_monte_carlo() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.5, 0.3];
        let result = engine.execute("monte_carlo", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_execute_markov() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.7, 0.3];
        let result = engine.execute("markov", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_bayesian() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.5, 0.3, 0.2];
        let result = engine.execute("bayesian", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 4);
    }

    #[test]
    fn test_execute_entropy() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.25, 0.25, 0.25, 0.25];
        let result = engine.execute("entropy", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert!(!values.is_empty());
    }

    #[test]
    fn test_execute_cosine() {
        let mut engine = ComputeEngine::new();
        let input = vec![1.0, 0.0, 0.0, 1.0];
        let result = engine.execute("cosine", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_pid() {
        let mut engine = ComputeEngine::new();
        let input = vec![1.0, 0.5, 0.1];
        let result = engine.execute("pid", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_fft() {
        let mut engine = ComputeEngine::new();
        let input = vec![1.0, 0.0, 0.0, 0.0];
        let result = engine.execute("fft", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_nash() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.5, 0.5, 0.5];
        let result = engine.execute("nash", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_optimize_ga() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = engine.execute("optimize_ga", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_boltzmann() {
        let mut engine = ComputeEngine::new();
        let input = vec![1.0, -0.5, 0.3, -0.8];
        let result = engine.execute("boltzmann", &input);
        assert!(result.is_ok());
        let values = result.unwrap();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_execute_free_energy() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.5, 0.3, 0.2];
        let result = engine.execute("free_energy", &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);
    }

    #[test]
    fn test_execute_free_energy_insufficient_input() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.5, 0.3];
        let result = engine.execute("free_energy", &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0.0]);
    }

    #[test]
    fn test_execute_mutual_info() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.3, 0.2, 0.1];
        let result = engine.execute("mutual_info", &input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_mutual_info_insufficient() {
        let mut engine = ComputeEngine::new();
        let input = vec![0.3, 0.2];
        let result = engine.execute("mutual_info", &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0.0]);
    }

    #[test]
    fn test_execute_unknown_task() {
        let mut engine = ComputeEngine::new();
        let input = vec![1.0];
        let result = engine.execute("nonexistent_task", &input);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown compute task"));
    }
}
