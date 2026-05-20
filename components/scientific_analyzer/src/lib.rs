// Scientific Analysis Pipeline
// Deterministic loop: OBSERVE → HYPOTHESIS → EXPERIMENT → VERIFY → CONSTELLATION
// With batch tensor operations for parallel analysis

pub mod ast_parser;
pub mod hypothesis;
pub mod experiment;
pub mod verifier;
pub mod pipeline;
pub mod batch_tensor;
pub mod tensor_extractor;

pub use ast_parser::{AstObservation, CodeStructure, FunctionSignature};
pub use hypothesis::{Hypothesis, ExperimentDesign};
pub use experiment::{ExperimentResult, TestOutcome};
pub use verifier::{VerificationResult, ConfidenceUpdate, ConstellationUpdate, Verdict, NodeColor, NodeStatus};
pub use pipeline::{ScientificPipeline, AnalysisReport, PipelineSummary};
pub use batch_tensor::{
    batch_extract_features, batch_compute_similarity, prioritize_tests,
    batch_generate_hypotheses, batch_run_experiments, batch_verify,
    compute_code_information_flow, detect_code_clones, run_batch_analysis,
    BatchAnalysisReport,
};
