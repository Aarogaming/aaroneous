//! crates/evolution
//! Biological and Persona Evolution Engine for Aaroneous.
//! Unifies chromosome genetics, GGUF model persona extraction, and skill ranking.

pub mod candle_persona_engine;
pub mod continuous_evolution;
pub mod genetics;
pub mod neurochemistry;
pub mod self_digestion;
pub mod skills;
pub mod persona_fusion;
pub mod workspace;

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
pub use skills::{
    CapabilityMaturityLevel, FusedSkill, PersonaRank, Skill, SkillOrigin, SkillRegistry, SkillType,
    SpecialistSkillSet,
};
pub use persona_fusion::{
    CapabilityFusionEngine, CompositePersonaVector, CompoundAgentProfile, ExperiencePersonaLayer,
    FusedEmergentSkill, NarrativeVoiceLayer, OlympianPersonaVector, PersonalityPersonaLayer,
    PersonaFusionEngine, RelationalPersonaLayer, SpecialistPersonaLayer,
};
pub use workspace::WorkspacePaths;
