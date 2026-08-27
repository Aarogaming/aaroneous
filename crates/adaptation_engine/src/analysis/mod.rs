// Scientific Analysis Pipeline
// Deterministic loop: OBSERVE → HYPOTHESIS → EXPERIMENT → VERIFY → CONSTELLATION
// With batch tensor operations for parallel analysis

pub mod ast_parser;
pub mod batch_tensor;
pub mod experiment;
pub mod hypothesis;
pub mod pipeline;
pub mod tensor_extractor;
pub mod verifier;

pub use ast_parser::{AstObservation, CodeStructure, FunctionSignature};
pub use batch_tensor::{
    batch_compute_similarity, batch_extract_features, batch_generate_hypotheses,
    batch_run_experiments, batch_verify, compute_code_information_flow, detect_code_clones,
    prioritize_tests, run_batch_analysis, BatchAnalysisReport,
};
pub use experiment::{ExperimentResult, TestOutcome};
pub use hypothesis::{ExperimentDesign, Hypothesis};
pub use pipeline::{AnalysisReport, PipelineSummary, ScientificPipeline};
pub use verifier::{
    ConfidenceUpdate, ConstellationUpdate, NodeColor, NodeStatus, Verdict, VerificationResult,
};
