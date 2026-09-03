//! crates/transpiler
//! Synthetic Intelligence (SI) to Conventional AI Inter-Intelligence Translation and Model Conversion Bridge.

pub mod ai_to_si;
pub mod model_converter;
pub mod polyglot;
pub mod prefix_cache_integration;
pub mod reflection_loop;
pub mod si_distiller;
pub mod si_to_ai;

pub use ai_to_si::{AiToSiTranspiler, ExtractedCodePayload};
pub use model_converter::{ModelConverter, ModelManifest, QuantizationType};
pub use polyglot::{
    CircuitBreakerState, MetricsCollector, PolyglotCapsule, PolyglotFoundry, SelfHealingState,
    TelemetryBuffer, TelemetryEntry, TelemetryLevel,
};
pub use prefix_cache_integration::{
    parse_nl_to_opcode_dag, GgufModelRunner, PrefixCache, PrefixCacheEntry, PromptPrefixKey,
};
pub use reflection_loop::{ReflectionFeedback, ReflectionLoopEngine};
pub use si_distiller::{DistillationBatchReport, SiDistillationMiner};
pub use si_to_ai::{AiPromptContext, SiToAiTranspiler};

use anyhow::Result;

/// Master Transpiler Engine bridging SI and AI
pub struct TranspilerEngine;

impl TranspilerEngine {
    /// Serializes an SI task intent and tensor context to an AI model prompt
    pub fn si_to_ai_prompt(
        task_id: &str,
        domain: &str,
        tensor_context: &[f32],
        goal_description: &str,
    ) -> Result<AiPromptContext> {
        SiToAiTranspiler::serialize_task_to_prompt(task_id, domain, tensor_context, goal_description)
    }

    /// Extracts clean executable code from raw AI markdown responses
    pub fn ai_to_si_code(raw_ai_response: &str) -> Result<ExtractedCodePayload> {
        AiToSiTranspiler::extract_code(raw_ai_response)
    }

    /// Plans a quantization profile for a foundation model
    pub fn plan_quantization(model_name: &str, vram_gb: f32) -> Result<QuantizationType> {
        ModelConverter::plan_quantization(model_name, vram_gb)
    }

    /// Formulates an automated self-repair prompt when compilation errors occur
    pub fn formulate_repair(iteration: usize, code: &str, error: &str) -> Result<ReflectionFeedback> {
        let engine = ReflectionLoopEngine::default();
        engine.formulate_repair_prompt(iteration, code, error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transpiler_end_to_end() {
        // 1. SI -> AI Prompt
        let prompt = TranspilerEngine::si_to_ai_prompt("task_1", "Kernel", &[1.0, 0.0], "Write vector norm").unwrap();
        assert!(prompt.system_instruction.contains("Kernel"));

        // 2. AI -> SI Ingestion
        let ai_response = "Here is the code:\n```rust\npub fn norm(v: &[f32]) -> f32 { 1.0 }\n```";
        let extracted = TranspilerEngine::ai_to_si_code(ai_response).unwrap();
        assert_eq!(extracted.language, "rust");

        // 3. Model Quantization planning
        let quant = TranspilerEngine::plan_quantization("qwen2.5-7b", 12.0).unwrap();
        assert_eq!(quant, QuantizationType::Q6_K);
    }
}
