pub mod dag;
pub mod task_spec;
pub mod analyzer;
pub mod distillation;
pub mod embedding;

pub use dag::{SovereignGraph, Node, Edge, NodeKind, EdgeKind, TaskStatus};
pub use task_spec::{SovereignTaskSpec, sovereign_task_specs, spec_for};
pub use analyzer::{GGUFAnalyzer, ModelAnalysis, TensorStats, BlockProfile};
pub use distillation::{GenerationReport, generate_training_examples, generate_distillation_plan};
pub use embedding::{EmbeddingStore, cosine_similarity};

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
