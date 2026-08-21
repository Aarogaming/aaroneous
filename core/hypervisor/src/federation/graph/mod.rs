pub mod analyzer;
pub mod dag;
pub mod distillation;
pub mod embedding;
pub mod task_spec;

pub use analyzer::{BlockProfile, GGUFAnalyzer, ModelAnalysis, TensorStats};
pub use dag::{Edge, EdgeKind, Node, NodeKind, SovereignGraph, TaskStatus};
pub use distillation::{GenerationReport, generate_distillation_plan, generate_training_examples};
pub use embedding::{EmbeddingStore, cosine_similarity};
pub use task_spec::{SovereignTaskSpec, sovereign_task_specs, spec_for};

/// Convert a model analysis to genome JSON representation
pub fn analysis_to_genome_json(analysis: &ModelAnalysis) -> serde_json::Value {
    serde_json::json!({
        "tensor_count": analysis.tensor_count,
        "block_count": analysis.total_blocks,
        "total_params": analysis.total_parameters_estimate,
        "avg_sparsity": analysis.overall_sparsity,
        "depth_gradient": analysis.depth_gradient,
    })
}
