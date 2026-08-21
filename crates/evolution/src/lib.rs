//! crates/evolution
//! Biological and Soul Evolution Engine for Aaroneous.
//! Unifies chromosome genetics, GGUF model soul extraction, and Rimuru skill ranking.

pub mod candle_soul_engine;
pub mod continuous_evolution;
pub mod genetics;
pub mod neurochemistry;
pub mod self_digestion;
pub mod skills;
pub mod soul_fusion;
pub mod workspace;

pub use continuous_evolution::{
    ContinuousSelfEvolutionEngine, SelfEvolutionConfig, SelfEvolutionCycleReport,
};
pub use neurochemistry::{
    AutonomicImpulse, ImpulseKind, NeurochemicalHomeostasisEngine, NeurochemicalLevels,
    SpecialistTokenAllocation,
};

// Re-export candle & neural soul
pub use candle_soul_engine::{
    CandleSoulEngine, DiscoveredGgufModel, GenerationConfig, GgufModelMetadata,
};

// Re-export genetics
pub use genetics::{GeneticCategory, GeneticLocus, LociSource, SpecialistGenome};

// Re-export digestion & souls
pub use self_digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceSoul, NarrativeSoul,
    PersonalitySoul, RelationalSoul, SpecialistSoul,
};

// Re-export skills & leveling
pub use skills::{
    FusedSkill, Skill, SkillOrigin, SkillRegistry, SkillType, SpecialistSkillSet, SoulRank,
};
pub use soul_fusion::{
    CompositeSoulVector, ExperienceSoulLayer, FusedEmergentSkill, NarrativeSoulLayer,
    PersonalitySoulLayer, RelationalSoulLayer, SoulFusionEngine, SpecialistSoulLayer,
};
pub use workspace::WorkspacePaths;
