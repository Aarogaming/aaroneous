//! crates/chimera
//! Universal software adaptation, binary deconstruction, AST mutation, and code repair engine for Aaroneous.

pub mod analysis;
pub mod ast_parser;
pub mod auto_wrapper;
pub mod autonomous_scientific;
pub mod dev_tools;
pub mod disassembly;
pub mod error_interceptor;
pub mod mutation;
pub mod parallel_scanner;
pub mod pattern_rewriter;
pub mod protocol_bridge;
pub mod repo_watcher;
pub mod sandbox;
pub mod scientific_loop;
pub mod self_rebuild;
pub mod self_repair;

pub use auto_wrapper::{
    AutoWrapperEngine, NativeOrganRunner, OrganResponse, ProbeValidationReport,
    TargetCapabilityManifest, TargetProgramType,
};
pub use autonomous_scientific::{
    AutonomousScientificEngine, HypothesisCategory, ScientificCycleReport, TestedHypothesis,
};
pub use analysis::{
    batch_compute_similarity, batch_extract_features, batch_generate_hypotheses,
    batch_run_experiments, batch_verify, compute_code_information_flow, detect_code_clones,
    prioritize_tests, run_batch_analysis, AnalysisReport, BatchAnalysisReport, ConfidenceUpdate,
    ConstellationUpdate, ExperimentDesign, ExperimentResult, Hypothesis, PipelineSummary,
    ScientificPipeline, TestOutcome, Verdict, VerificationResult,
};
pub use ast_parser::{AstObservation, AstParser, FunctionSignature};
pub use dev_tools::{CompilerDiagnosticItem, DevToolsEngine, WorkspaceFileItem};
pub use disassembly::{BasicBlock, BinaryInspector, BinaryManifest, BinarySection, DisassembledInstruction};
pub use error_interceptor::{InterceptedProcessError, ProcessErrorInterceptor};
pub use mutation::{CodeMutator, PatchProposal};
pub use parallel_scanner::{BatchScanReport, ParallelScanner};
pub use pattern_rewriter::{PatternMatch, PatternRewriter, StructuralPatch};
pub use protocol_bridge::{ChimeraProtocolBridge, MnlpPatchPacket};
pub use repo_watcher::{RepoWatcher, SourceChangeEvent};
pub use sandbox::ShadowSandbox;
pub use scientific_loop::{AdaptationHypothesis, ScientificLoop, VerificationReport};
pub use self_rebuild::{RebuildReport, SelfRebuildEngine};
pub use self_repair::{CompilerDiagnostic, SelfRepairEngine, SelfRepairReport};

use anyhow::Result;
use nervous_system::SynapseState;

/// Master Chimera Engine interface for universal software adaptation and AST repair
pub struct ChimeraEngine;

impl ChimeraEngine {
    /// Ingests and analyzes a source file
    pub fn inspect_source(file_path: &str, code: &str) -> Result<AstObservation> {
        AstParser::parse_source(file_path, code)
    }

    /// Dissects a target binary or shared library
    pub fn inspect_binary(file_path: &str, raw_bytes: &[u8]) -> Result<BinaryManifest> {
        BinaryInspector::inspect_binary(file_path, raw_bytes)
    }

    /// Structural pattern-based search and rewrite
    pub fn rewrite_pattern(
        file_path: &str,
        source_code: &str,
        search_pattern: &str,
        replace_template: &str,
    ) -> Result<(String, Vec<StructuralPatch>)> {
        PatternRewriter::rewrite_source(file_path, source_code, search_pattern, replace_template)
    }

    /// Autonomous self-repair loop testing in shadow sandbox with dopamine feedback
    pub fn self_repair(
        file_path: &str,
        original_source: &str,
        known_error: &str,
        synapse: &mut SynapseState,
    ) -> Result<SelfRepairReport> {
        let engine = SelfRepairEngine::new()?;
        engine.attempt_repair(file_path, original_source, known_error, synapse)
    }

    /// Runs a scientific adaptation loop to patch a problem in code
    pub fn adapt_code(
        file_path: &str,
        code: &str,
        target_pattern: &str,
        replacement: &str,
    ) -> Result<(AstObservation, AdaptationHypothesis, VerificationReport)> {
        ScientificLoop::execute_adaptation_cycle(file_path, code, target_pattern, replacement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chimera_engine_e2e() {
        let code = r#"
pub fn broken_routine() {
    panic!("fatal memory condition");
}
"#;
        let (obs, _hyp, report) = ChimeraEngine::adapt_code(
            "kernel.rs",
            code,
            "panic!(\"fatal memory condition\");",
            "tracing::error!(\"handled memory condition\");",
        ).unwrap();

        assert_eq!(obs.functions.len(), 1);
        assert!(report.success);
        assert_eq!(report.verdict, "ADAPTATION_ACCEPTED");
    }
}
