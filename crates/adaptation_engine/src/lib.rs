//! crates/adaptation_engine
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
pub use ast_parser::{AstDiffResult, AstObservation, AstParser, FunctionSignature, SourceLanguage};
pub use dev_tools::{CompilerDiagnosticItem, DevToolsEngine, WorkspaceFileItem};
pub use disassembly::{BasicBlock, BinaryInspector, BinaryManifest, BinarySection, DisassembledInstruction};
pub use error_interceptor::{InterceptedProcessError, ProcessErrorInterceptor};
pub use mutation::{CodeMutator, PatchProposal};
pub use parallel_scanner::{BatchScanReport, ParallelScanner};
pub use pattern_rewriter::{PatternMatch, PatternRewriter, StructuralPatch};
pub use protocol_bridge::{ChimeraProtocolBridge, MnlpPatchPacket, MnlpProtocolBridge};
pub use repo_watcher::{RepoWatcher, SourceChangeEvent};
pub use sandbox::ShadowSandbox;
pub use scientific_loop::{AdaptationHypothesis, ScientificLoop, VerificationReport};
pub use self_rebuild::{RebuildReport, SelfRebuildEngine};
pub use self_repair::{CompilerDiagnostic, SelfRepairEngine, SelfRepairReport};

pub extern crate ipc_bus as nervous_system;
pub use ipc_bus;
pub extern crate governance as biology;
pub use governance;

use anyhow::Result;
use ipc_bus::SynapseState;

/// Master Adaptation Engine interface for universal software adaptation and AST repair
pub struct AdaptationEngine;

impl AdaptationEngine {
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
    fn test_adaptation_engine_e2e() {
        let code = r#"
pub fn broken_routine() {
    panic!("fatal memory condition");
}
"#;
        let (obs, _hyp, report) = AdaptationEngine::adapt_code(
            "kernel.rs",
            code,
            "panic!(\"fatal memory condition\");",
            "tracing::error!(\"handled memory condition\");",
        ).unwrap();

        assert_eq!(obs.functions.len(), 1);
        assert!(report.success);
        assert_eq!(report.verdict, "ADAPTATION_ACCEPTED");
    }

    #[test]
    fn test_inspect_source_valid() {
        let code = r#"
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let obs = AdaptationEngine::inspect_source("math.rs", code).unwrap();
        assert_eq!(obs.file_path, "math.rs");
        assert!(!obs.functions.is_empty());
        assert_eq!(obs.functions[0].name, "add");
    }

    #[test]
    fn test_inspect_source_empty() {
        let obs = AdaptationEngine::inspect_source("empty.rs", "").unwrap();
        assert!(obs.functions.is_empty());
    }

    #[test]
    fn test_inspect_source_multiple_functions() {
        let code = r#"
pub fn first() {}
pub fn second() {}
pub fn third() {}
"#;
        let obs = AdaptationEngine::inspect_source("multi.rs", code).unwrap();
        assert_eq!(obs.functions.len(), 3);
    }

    #[test]
    fn test_rewrite_pattern_basic() {
        let code = "fn main() { println!(\"hello\"); }";
        let (rewritten, patches) = AdaptationEngine::rewrite_pattern(
            "main.rs",
            code,
            "println!(\"hello\")",
            "tracing::info!(\"hello\")",
        ).unwrap();
        assert!(rewritten.contains("tracing::info"));
        assert!(!patches.is_empty());
    }

    #[test]
    fn test_rewrite_pattern_no_match() {
        let code = "fn main() { }";
        let (rewritten, patches) = AdaptationEngine::rewrite_pattern(
            "main.rs",
            code,
            "nonexistent_pattern",
            "replacement",
        ).unwrap();
        assert_eq!(rewritten, code);
        assert!(patches.is_empty());
    }

    #[test]
    fn test_rewrite_pattern_multiple_occurrences() {
        let code = "fn a() { panic!(\"error\"); }\nfn b() { panic!(\"error\"); }";
        let (_, patches) = AdaptationEngine::rewrite_pattern(
            "multi.rs",
            code,
            "panic!(\"error\")",
            "return Err(\"error\");",
        ).unwrap();
        assert!(patches.len() >= 2, "Should match both occurrences");
    }

    #[test]
    fn test_inspect_binary_valid_pe() {
        let mut bytes = vec![0u8; 1024];
        bytes[0] = b'M';
        bytes[1] = b'Z';
        bytes[0x3C] = 64;
        bytes[64] = b'P';
        bytes[65] = b'E';
        bytes[66] = 0;
        bytes[67] = 0;

        let manifest = AdaptationEngine::inspect_binary("test.exe", &bytes).unwrap();
        assert_eq!(manifest.file_path, "test.exe");
        assert!(manifest.file_size_bytes > 0);
    }
}
