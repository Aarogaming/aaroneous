//! fabricator.rs
//! Fabricator (The Master Craftsman) & CompilerCore (Autonomous Compiler & Adaptation Engine).
//! Powered directly by Adaptation Engine.
//! Domain Opcode: 0x0400 (FABRICATION_ADAPTATION)

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use tracing::info;

use adaptation_engine::{AdaptationEngine, PatchProposal};
use crate::traits::{MnlpPacket, MnlpResponse, RelicEngine, SovereignSpecialist, SpecialistHealth};

/// CompilerCore Relic Engine: Autonomous build automation and software forge
#[derive(Debug, Clone, Default)]
pub struct CompilerCoreRelic {
    pub total_adaptations_forged: usize,
    pub active_build_pipelines: usize,
}

impl RelicEngine for CompilerCoreRelic {
    fn relic_name(&self) -> &'static str {
        "CompilerCore"
    }

    fn supervisor_name(&self) -> &'static str {
        "Fabricator"
    }

    fn relic_status(&self) -> String {
        format!(
            "CompilerCore Engine: {} code adaptations forged, {} build pipelines active",
            self.total_adaptations_forged, self.active_build_pipelines
        )
    }
}

/// Fabricator Sovereign Specialist
pub struct FabricatorSpecialist {
    pub tokens: f32,
    pub max_tokens: f32,
    pub forge: CompilerCoreRelic,
}

impl Default for FabricatorSpecialist {
    fn default() -> Self {
        Self::new()
    }
}

impl FabricatorSpecialist {
    pub fn new() -> Self {
        Self {
            tokens: 100.0,
            max_tokens: 100.0,
            forge: CompilerCoreRelic::default(),
        }
    }

    /// Forges a code adaptation patch using Adaptation Engine
    pub fn forge_code_repair(&mut self, file: &str, code: &str, target: &str, replacement: &str) -> Result<PatchProposal> {
        info!(target: "specialist::fabricator", %file, "Forging code patch in the CompilerCore");
        let (_obs, _hyp, report) = AdaptationEngine::adapt_code(file, code, target, replacement)?;
        self.forge.total_adaptations_forged += 1;
        
        let patch = adaptation_engine::CodeMutator::synthesize_repair(file, code, target, replacement)?;
        info!(target: "specialist::fabricator", verdict = %report.verdict, "CompilerCore adaptation verified");
        Ok(patch)
    }

    /// Forges a structural pattern rewrite using Comby-style AST matching
    pub fn forge_structural_rewrite(
        &mut self,
        file: &str,
        code: &str,
        search_pattern: &str,
        replace_template: &str,
    ) -> Result<(String, Vec<adaptation_engine::StructuralPatch>)> {
        info!(target: "specialist::fabricator", %file, "Executing structural pattern rewrite in the CompilerCore");
        let (rewritten, patches) = AdaptationEngine::rewrite_pattern(file, code, search_pattern, replace_template)?;
        self.forge.total_adaptations_forged += patches.len();
        Ok((rewritten, patches))
    }

    /// Inspects and disassembles a native enzyme binary (PE/ELF/Mach-O)
    pub fn inspect_enzyme_binary(&self, file: &str, raw_bytes: &[u8]) -> Result<adaptation_engine::BinaryManifest> {
        info!(target: "specialist::fabricator", %file, size = raw_bytes.len(), "Inspecting native enzyme binary");
        AdaptationEngine::inspect_binary(file, raw_bytes)
    }

    /// Autonomous self-repair loop testing in shadow sandbox
    pub fn forge_autonomous_self_repair(
        &mut self,
        file: &str,
        code: &str,
        known_error: &str,
        synapse: &mut nervous_system::SynapseState,
    ) -> Result<adaptation_engine::SelfRepairReport> {
        info!(target: "specialist::fabricator", %file, "Executing autonomous sandboxed self-repair in the CompilerCore");
        let report = AdaptationEngine::self_repair(file, code, known_error, synapse)?;
        if report.is_verified {
            self.forge.total_adaptations_forged += report.patches_applied.len();
        }
        Ok(report)
    }

    /// Evaluates and optimizes code intent in native machine graph form with GPU free-energy minimization
    pub fn forge_machine_native_optimization(
        &mut self,
        intent: &str,
    ) -> Result<(compute::NativeComputationalGraph, String)> {
        info!(target: "specialist::fabricator", %intent, "Translating intent into native computational graph");
        let initial_graph = compute::EdgeLinguisticLens::intent_to_native_graph(intent);
        initial_graph.verify_dimensional_invariants()?;

        let mut engine = compute::MachineNativePredictionEngine::new();
        let optimized = engine.predict_optimal_mutation(&initial_graph)?;
        let explanation = compute::EdgeLinguisticLens::native_graph_to_explanation(&optimized);

        self.forge.total_adaptations_forged += 1;
        Ok((optimized, explanation))
    }

    /// Forges an autonomous software wrapper organ for an external binary/CLI tool
    pub async fn forge_organ_wrapper(&mut self, target_path: &str, custom_name: Option<&str>, out_dir: &std::path::Path) -> Result<(adaptation_engine::TargetCapabilityManifest, std::path::PathBuf)> {
        info!(target: "specialist::fabricator", target_path, "Forging autonomous software organ in the CompilerCore");
        let manifest = adaptation_engine::AutoWrapperEngine::inspect_target(std::path::Path::new(target_path), custom_name)?;
        let _probe = adaptation_engine::AutoWrapperEngine::probe_target(&manifest).await?;
        let staged_crate = adaptation_engine::AutoWrapperEngine::build_and_stage_organ(&manifest, out_dir)?;
        
        self.forge.total_adaptations_forged += 1;
        self.forge.active_build_pipelines += 1;
        Ok((manifest, staged_crate))
    }

    /// Executes the 5-stage autonomous scientific AST hypothesis loop on target code
    pub fn forge_scientific_hypothesis_cycle(
        &mut self,
        file_path: &std::path::Path,
        code: &str,
    ) -> Result<adaptation_engine::ScientificCycleReport> {
        info!(target: "specialist::fabricator", ?file_path, "Executing autonomous scientific AST hypothesis loop in the CompilerCore");
        let report = adaptation_engine::AutonomousScientificEngine::analyze_and_hypothesize(file_path, code)?;
        self.forge.total_adaptations_forged += report.hypotheses_accepted;
        Ok(report)
    }
}

#[async_trait]
impl SovereignSpecialist for FabricatorSpecialist {
    fn name(&self) -> &'static str {
        "Fabricator"
    }

    fn domain_opcode(&self) -> u16 {
        0x0400
    }

    async fn handle_packet(&mut self, packet: MnlpPacket) -> Result<MnlpResponse> {
        let payload_str = String::from_utf8_lossy(&packet.payload);
        
        if payload_str.starts_with("wrap:") {
            let target_path = payload_str.trim_start_matches("wrap:").trim();
            let temp_out = paths::WorkspacePaths::discover().models().join("organs");
            let (manifest, crate_path) = self.forge_organ_wrapper(target_path, None, &temp_out).await?;
            
            return Ok(MnlpResponse {
                success: true,
                opcode: self.domain_opcode(),
                correlation_id: packet.correlation_id,
                message: format!("Fabricator successfully forged organ '{}' at {:?}", manifest.name, crate_path),
                payload: serde_json::to_vec(&manifest)?,
            });
        }

        if payload_str.starts_with("scientific:") || payload_str.starts_with("hypothesis:") {
            let code = payload_str.trim_start_matches("scientific:").trim_start_matches("hypothesis:").trim();
            let report = self.forge_scientific_hypothesis_cycle(std::path::Path::new("virtual_target.rs"), code)?;

            return Ok(MnlpResponse {
                success: true,
                opcode: self.domain_opcode(),
                correlation_id: packet.correlation_id,
                message: format!("Fabricator evaluated {} hypotheses (accepted: {})", report.hypotheses_tested, report.hypotheses_accepted),
                payload: serde_json::to_vec(&report)?,
            });
        }

        let patch = self.forge_code_repair("src/main.rs", &payload_str, "panic!();", "return Ok(());")?;
        let payload = serde_json::to_vec(&patch)?;

        Ok(MnlpResponse {
            success: true,
            opcode: self.domain_opcode(),
            correlation_id: packet.correlation_id,
            message: "Fabricator forged code patch via Adaptation Engine".to_string(),
            payload,
        })
    }

    fn recharge_metabolism(&mut self, tokens: f32) {
        self.tokens = (self.tokens + tokens).min(self.max_tokens);
    }

    fn health_report(&self) -> SpecialistHealth {
        SpecialistHealth {
            name: self.name().to_string(),
            domain_opcode: self.domain_opcode(),
            tokens: self.tokens,
            max_tokens: self.max_tokens,
            backlog_count: 0,
            is_dormant: self.tokens < 1.0,
            last_active: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fabricator_forge() {
        let mut fabricator = FabricatorSpecialist::new();
        let src = "fn work() { panic!(); }";
        let patch = fabricator.forge_code_repair("test.rs", src, "panic!();", "Ok(())").unwrap();
        assert!(patch.patch_content.contains("Ok(())"));
        assert_eq!(fabricator.forge.total_adaptations_forged, 1);
    }

    #[test]
    fn test_fabricator_structural_rewrite() {
        let mut fabricator = FabricatorSpecialist::new();
        let src = "fn run() { log(\"hello\"); }";
        let (rewritten, patches) = fabricator
            .forge_structural_rewrite("test.rs", src, "log(:[msg]);", "tracing::info!(:[msg]);")
            .unwrap();
        assert!(rewritten.contains("tracing::info!(\"hello\");"));
        assert_eq!(patches.len(), 1);
    }

    #[test]
    fn test_fabricator_autonomous_self_repair() {
        let mut fabricator = FabricatorSpecialist::new();
        let src = "use digestion::Persona;\n";
        let mut synapse = nervous_system::SynapseState {
            integrity_score: 80,
            ..Default::default()
        };
        let report = fabricator
            .forge_autonomous_self_repair("test.rs", src, "error[E0432]: unresolved import", &mut synapse)
            .unwrap();
        assert!(report.is_verified);
        assert_eq!(synapse.integrity_score, 85);
    }

    #[test]
    fn test_fabricator_machine_native_optimization() {
        let mut fabricator = FabricatorSpecialist::new();
        let (graph, explanation) = fabricator
            .forge_machine_native_optimization("Synthesize vector allocation and energy tensor dot product")
            .unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert!(explanation.contains("Machine-Native"));
        assert_eq!(fabricator.forge.total_adaptations_forged, 1);
    }
}
