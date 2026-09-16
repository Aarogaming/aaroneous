//! crates/evolution
//! Biological and Persona Evolution Engine for Aaroneous.
//! Unifies chromosome genetics, GGUF model persona extraction, and skill ranking.

pub mod artifact_pruning;
pub mod candle_persona_engine;
pub mod capability_spec;
pub mod continuous_evolution;
pub mod loss_metrics;
pub use artifact_pruning as self_digestion;
pub use capability_spec as genetics;
pub use loss_metrics as neurochemistry;
pub mod persona_fusion;
pub mod skills;
pub mod streaming_adaptation;
pub mod workspace;

pub use streaming_adaptation::{
    AdaptationError, AutonomousPacingRegulator, PacingConfig, PacingDecision, PacingTier,
    StreamingSelfCorrectionFilter, ThermodynamicTelemetry,
};

pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;

pub use continuous_evolution::{
    ContinuousSelfEvolutionEngine, SelfEvolutionConfig, SelfEvolutionCycleReport,
};
pub use neurochemistry::{
    AdaptationHomeostasisLevels, AutonomicHomeostasisEngine, AutonomicImpulse, ImpulseKind,
    NeurochemicalHomeostasisEngine, NeurochemicalLevels, SpecialistTokenAllocation,
};

// Re-export candle & neural persona
pub use candle_persona_engine::{
    CandlePersonaEngine, DiscoveredGgufModel, GenerationConfig, GgufModelMetadata,
};

// Re-export parameter configuration & genetics
pub use genetics::{
    AgentConfigProfile, GeneticCategory, GeneticLocus, LociSource, ParameterGenome, ParameterLocus,
    SpecialistGenome,
};

// Re-export digestion & personas
pub use self_digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceProfile,
    NarrativeProfile, PersonalityProfile, RelationalProfile, SpecialistPersona,
};

// Re-export skills & leveling
pub use persona_fusion::{
    CapabilityFusionEngine, CompositePersonaVector, CompoundAgentProfile, ExperiencePersonaLayer,
    FusedEmergentSkill, NarrativeVoiceLayer, OlympianPersonaVector, PersonaFusionEngine,
    PersonalityPersonaLayer, RelationalPersonaLayer, SpecialistPersonaLayer,
};
pub use skills::{
    CapabilityMaturityLevel, FusedSkill, PersonaRank, Skill, SkillOrigin, SkillRegistry, SkillType,
    SpecialistSkillSet,
};
pub use workspace::WorkspacePaths;
