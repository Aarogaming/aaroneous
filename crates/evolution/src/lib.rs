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

pub use continuous_evolution::{
    ContinuousSelfEvolutionEngine, SelfEvolutionConfig, SelfEvolutionCycleReport,
};
pub use neurochemistry::{
    AutonomicImpulse, ImpulseKind, NeurochemicalHomeostasisEngine, NeurochemicalLevels,
    SpecialistTokenAllocation,
};

// Re-export candle & neural persona
pub use candle_persona_engine::{
    CandlePersonaEngine, DiscoveredGgufModel, GenerationConfig, GgufModelMetadata,
};

// Re-export genetics
pub use genetics::{GeneticCategory, GeneticLocus, LociSource, SpecialistGenome};

// Re-export digestion & personas
pub use self_digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceProfile,
    NarrativeProfile, PersonalityProfile, RelationalProfile, SpecialistPersona,
};

// Re-export skills & leveling
pub use skills::{
    FusedSkill, Skill, SkillOrigin, SkillRegistry, SkillType, SpecialistSkillSet, PersonaRank,
};
pub use persona_fusion::{
    CompositePersonaVector, ExperiencePersonaLayer, FusedEmergentSkill, NarrativeVoiceLayer,
    PersonalityPersonaLayer, RelationalPersonaLayer, PersonaFusionEngine, SpecialistPersonaLayer,
};
pub use workspace::WorkspacePaths;
