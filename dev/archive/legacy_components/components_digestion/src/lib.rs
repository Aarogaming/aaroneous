pub mod self_digestion;
pub mod workspace;
pub use self_digestion::{
    DigestionConfig, DigestionEngine, DigestionEvent, DigestionTask, ExperienceSoul, NarrativeSoul,
    PersonalitySoul, RelationalSoul, SpecialistSoul,
};
pub use workspace::WorkspacePaths;
