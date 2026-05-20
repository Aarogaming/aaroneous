pub mod dag;
pub mod embedding;
pub mod analyzer;
pub mod task_spec;
pub mod distillation;

pub use dag::{
    SovereignGraph, Node, Edge, NodeKind, EdgeKind, TaskStatus,
    GraphError, task_dag_from_odin_output, model_lineage_graph,
};
pub use embedding::{EmbeddingStore, EmbeddedMemory, SimilarMemory, cosine_similarity};
pub use analyzer::{GGUFAnalyzer, ModelAnalysis, BlockProfile, analysis_to_genome_json};
pub use task_spec::{
    SovereignTaskSpec, TaskCapability, ModelTier, OutputFormat,
    sovereign_task_specs, spec_for, print_roster_summary,
};
pub use distillation::{
    TrainingExample, LoraTrainingSpec, generate_distillation_plan, print_distillation_plan,
};