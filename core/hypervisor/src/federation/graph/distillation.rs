#[allow(unused_imports)]
use super::analyzer::{GGUFAnalyzer, ModelAnalysis};
use super::task_spec::{SovereignTaskSpec, sovereign_task_specs, spec_for};
use serde::{Deserialize, Serialize};
use std::io::Write;

// ── Training data generation ──────────────────────────────────────────────────

/// Result of a training data generation run for one sovereign.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationReport {
    pub sovereign: String,
    pub examples_generated: u32,
    pub examples_saved: u32,
    pub output_path: String,
    pub skipped_capabilities: Vec<String>,
    pub errors: Vec<String>,
    pub duration_secs: f64,
}

/// Generates synthetic training examples for a sovereign using the foundation model.
///
/// # Process
/// For each capability in the sovereign's task spec:
/// 1. Fill the `generation_prompt_template` with varied synthetic intents
/// 2. Call the foundation model (via `LLMClient`) with the sovereign's system prompt
/// 3. Store the result as a `TrainingExample`
///
/// Results are written as JSONL to `<training_data_dir>/<sovereign>-training.jsonl`.
/// Existing data is **appended to** (not overwritten) so repeated runs accumulate data.
///
/// # Arguments
/// * `sovereign_name` - Case-insensitive sovereign name (e.g. "Odin", "ariel")
/// * `count` - Total number of examples to generate across all capabilities
/// * `llm` - Foundation model client (should be backed by `foundation_v1.gguf`)
/// * `training_data_dir` - Directory to write the JSONL file to
pub async fn generate_training_examples(
    sovereign_name: &str,
    count: u32,
    llm: &crate::llm::LLMClient,
    training_data_dir: &std::path::Path,
) -> anyhow::Result<GenerationReport> {
    use anyhow::Context;
    let start = std::time::Instant::now();

    // Find the spec for this sovereign
    let spec = spec_for(sovereign_name)
        .ok_or_else(|| anyhow::anyhow!("No task spec found for sovereign '{}'", sovereign_name))?;

    let output_path = training_data_dir.join(format!(
        "{}-training.jsonl",
        spec.sovereign_name.to_lowercase()
    ));

    std::fs::create_dir_all(training_data_dir)
        .context("Failed to create training data directory")?;

    // Open in append mode — accumulate across runs
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .append(true)
        .open(&output_path)
        .context("Failed to open training data file")?;

    let mut report = GenerationReport {
        sovereign: spec.sovereign_name.clone(),
        examples_generated: 0,
        examples_saved: 0,
        output_path: output_path.to_string_lossy().to_string(),
        skipped_capabilities: vec![],
        errors: vec![],
        duration_secs: 0.0,
    };

    // Distribute `count` examples across capabilities proportionally
    let cap_count = spec.capabilities.len().max(1) as u32;
    let per_capability = (count / cap_count).max(1);

    for capability in &spec.capabilities {
        let cap_intents = synthetic_intents_for(&spec, &capability.id, per_capability);
        for (intent_idx, synthetic_intent) in cap_intents.iter().enumerate() {
            // Build system prompt from the sovereign's domain
            let system_prompt = format!(
                "You are {}, a sovereign specialist in the Aaroneous hive.\n\
                 Domain: {}\n\
                 Persona: {}\n\
                 Capability: {} — {}\n\
                 Respond precisely and in the expected output format.",
                spec.sovereign_name,
                spec.domain,
                spec.persona_summary,
                capability.id,
                capability.description,
            );

            // Build the user prompt: use the synthetic_intent directly as input.
            // The capability's `input_format` provides the framing so the model
            // knows what format to expect.  We do NOT use the generation_prompt_template
            // directly because it may contain domain-specific placeholders (e.g.
            // {{BIOMETRICS}}) that don't map to generic substitution logic.
            let user_prompt = format!("{}\n\nInput: {}", capability.description, synthetic_intent,);

            // Call the foundation model
            let response = match llm
                .generate_domain_response(&system_prompt, &user_prompt, &spec.domain_key())
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("cap={} idx={}: {}", capability.id, intent_idx, e);
                    report.errors.push(msg);
                    continue;
                }
            };

            // Skip obviously bad responses (too short)
            if response.len() < 20 {
                report.skipped_capabilities.push(capability.id.clone());
                continue;
            }

            // Quality score: 1.0 if response contains JSON, 0.6 otherwise
            let quality_score = if response.contains('{') || response.contains('[') {
                0.9
            } else {
                0.6
            };

            let example = TrainingExample {
                id: format!(
                    "{}-{}-{}",
                    spec.sovereign_name.to_lowercase(),
                    capability.id,
                    intent_idx
                ),
                sovereign: spec.sovereign_name.clone(),
                capability_id: capability.id.clone(),
                instruction: user_prompt,
                response,
                system_prompt: system_prompt.clone(),
                quality_score,
                generated_at: now_ms(),
            };

            report.examples_generated += 1;

            // Write as Alpaca JSON line
            let line = serde_json::to_string(&example.to_alpaca_json())?;
            writeln!(file, "{}", line)?;
            report.examples_saved += 1;
        }
    }

    report.duration_secs = start.elapsed().as_secs_f64();
    tracing::info!(
        "Training data generation complete: {} examples for {} in {:.1}s → {}",
        report.examples_saved,
        report.sovereign,
        report.duration_secs,
        report.output_path,
    );

    Ok(report)
}

/// Generate concrete, fully-filled synthetic prompts for a sovereign capability.
///
/// All placeholders are replaced with real values — the model sees complete,
/// unambiguous inputs, not template markers like `{{BIOMETRICS}}`.
fn synthetic_intents_for(spec: &SovereignTaskSpec, capability_id: &str, count: u32) -> Vec<String> {
    // Start with the capability's own example input if available
    let mut intents: Vec<String> = spec
        .capabilities
        .iter()
        .find(|c| c.id == capability_id)
        .and_then(|c| c.example.as_ref().map(|(input, _)| vec![input.clone()]))
        .unwrap_or_default();

    // Sovereign-specific concrete prompt banks — fully rendered, no placeholders
    let bank: Vec<String> = match spec.sovereign_name.to_lowercase().as_str() {

        // ── Wen: biometric classification ──
        "wen" => vec![
            r#"{"bpm":95,"bpm_trend":"up","mem_pressure":0.7,"time_of_day":"late_night"}"#.into(),
            r#"{"bpm":62,"bpm_trend":"stable","mem_pressure":0.2,"time_of_day":"morning"}"#.into(),
            r#"{"bpm":110,"bpm_trend":"up","mem_pressure":0.9,"time_of_day":"afternoon"}"#.into(),
            r#"{"bpm":75,"bpm_trend":"down","mem_pressure":0.4,"time_of_day":"evening"}"#.into(),
            r#"{"bpm":55,"bpm_trend":"down","mem_pressure":0.1,"time_of_day":"morning"}"#.into(),
            r#"{"bpm":88,"bpm_trend":"stable","mem_pressure":0.6,"time_of_day":"midday"}"#.into(),
            r#"{"bpm":130,"bpm_trend":"up","mem_pressure":0.95,"time_of_day":"evening"}"#.into(),
            r#"{"bpm":70,"bpm_trend":"stable","mem_pressure":0.3,"time_of_day":"afternoon"}"#.into(),
        ],

        // ── Odin: intent decomposition ──
        "odin" => vec![
            "Build a real-time dashboard showing sovereign execution metrics".into(),
            "Research the latest Rust async patterns and write a summary".into(),
            "Audit the Aaroneous HTTP API for security vulnerabilities".into(),
            "Archive this session's results to the DNA Bank and generate insights".into(),
            "Design and implement a biometric-triggered notification system".into(),
            "Create a LoRA training pipeline for Wen using foundation_v1.gguf".into(),
            "Set up continuous monitoring for all sovereign confidence scores".into(),
            "Integrate egui HiveRepresentativeComponent with the genome display".into(),
        ],

        // ── Ariel: UI/UX design ──
        "ariel" => vec![
            "Design a dark-mode sovereign dashboard with confidence heat map".into(),
            "Create a mobile layout for submitting intents to the hive".into(),
            "Design the Guild coordination panel showing Odin's task graph".into(),
            "Build a minimal AR overlay for the egui spatial interface".into(),
            "Design a genomic visualization for sovereign weight profiles".into(),
            "Create an onboarding flow for the Maelstrom launcher".into(),
            "Design a settings panel for distillation job management".into(),
            "Build a dark-mode graph view for the sovereign RAG memory network".into(),
        ],

        // ── Hermes: sync conflict resolution ──
        "hermes" => vec![
            r#"{"state_a":{"intent":"build dashboard","ts":1000},"state_b":{"intent":"research async","ts":990},"ts_a":1000,"ts_b":990}"#.into(),
            r#"{"state_a":{"config":"dark","version":3},"state_b":{"config":"light","version":2},"ts_a":2000,"ts_b":1800}"#.into(),
            r#"{"state_a":{"sessions":["s1","s2"]},"state_b":{"sessions":["s1","s3"]},"ts_a":500,"ts_b":510}"#.into(),
            r#"{"state_a":{"confidence":0.8},"state_b":{"confidence":0.75},"ts_a":300,"ts_b":350}"#.into(),
        ],

        // ── Kami: AR/VR spatial ──
        "kami" => vec![
            "Place a holographic display panel 1.5m in front of the user at eye level".into(),
            "Anchor the sovereign status overlay to the left wall of the physical room".into(),
            "Create a spatial notification that appears at the user's peripheral vision".into(),
            "Map the federation execution graph onto the floor of the physical space".into(),
            "Design an AR interaction zone for intent submission at arm's reach".into(),
            "Place sovereign confidence indicators as floating orbs around the workspace".into(),
        ],

        // ── Dionysus: archival ──
        "dionysus" => vec![
            r#"{"session_id":"s-001","results":[{"specialist":"Odin","status":"Success","output":"Task graph created"}],"intent":"build dashboard"}"#.into(),
            r#"{"session_id":"s-002","results":[{"specialist":"Wen","status":"Success","output":"{\"state\":\"focused\"}"}],"intent":"biometric check"}"#.into(),
            r#"{"session_id":"s-003","results":[{"specialist":"Argus","status":"Success","output":"{\"risk\":\"Low\"}"}],"intent":"security audit"}"#.into(),
            r#"{"session_id":"s-004","results":[{"specialist":"Merlin","status":"Success","output":"Research complete: 3 findings"}],"intent":"research async"}"#.into(),
        ],

        // ── Merlin: research synthesis ──
        "merlin" => vec![
            "Synthesize current best practices for Rust async programming with tokio".into(),
            "Research LoRA fine-tuning techniques for small language models on CPU".into(),
            "Summarize recent developments in GGUF quantization formats".into(),
            "Research TF-IDF vs neural embedding approaches for semantic search".into(),
            "Compile findings on AR UI design patterns for industrial applications".into(),
            "Research biometric classification accuracy benchmarks for stress detection".into(),
            "Synthesize information on sovereign AI architectures and multi-agent systems".into(),
            "Research best practices for SQLite persistence in Rust async applications".into(),
        ],

        // ── Argus: security audit ──
        "argus" => vec![
            "Audit the Aaroneous HTTP API for authentication vulnerabilities".into(),
            "Scan the specialist registry JSON for misconfigured permissions".into(),
            "Check the federation session manager for data leakage risks".into(),
            "Audit the CORS configuration for the development server".into(),
            "Scan the GGUF model loading path for path traversal vulnerabilities".into(),
            "Check the SQLite persistence layer for injection risks".into(),
            "Audit the SSE streaming endpoints for information disclosure".into(),
            "Review the API key authentication middleware for bypass conditions".into(),
        ],

        // ── Hephaestus: construction/fabrication ──
        "hephaestus" => vec![
            "Plan the construction of a new LoRA fine-tuning pipeline for Wen".into(),
            "Design the build system for compiling native engine plugins".into(),
            "Plan the integration of the unsloth training scripts into the CI pipeline".into(),
            "Design the deployment architecture for Aaroneous as a Windows service".into(),
            "Plan the fabrication of sovereign-specific GGUF models from foundation_v1".into(),
            "Design the maintenance schedule for the DNA Bank archival system".into(),
            "Plan the expansion of the internal HiveLevel with sovereign spatial avatars".into(),
            "Build the automated testing harness for all 9 sovereign specialists".into(),
        ],

        // ── Fallback ──
        _ => {
            let subjects = [
                "sovereign specialists", "AI agents", "federation events",
                "Aaroneous hive", "GGUF models", "intent processing",
                "RAG memory retrieval", "LoRA fine-tuning",
            ];
            subjects.iter().map(|s| format!("Handle this {} request: {}", spec.domain, s)).collect()
        }
    };

    // Fill from the bank, cycling if needed
    let mut idx = intents.len();
    while intents.len() < count as usize {
        intents.push(bank[idx % bank.len()].clone());
        idx += 1;
    }

    intents.truncate(count as usize);
    intents
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Training example ──────────────────────────────────────────────────────────

/// A single training conversation (instruction-response pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub id: String,
    pub sovereign: String,
    pub capability_id: String,
    /// The instruction/input to the model
    pub instruction: String,
    /// The expected output
    pub response: String,
    /// System prompt to bake the persona
    pub system_prompt: String,
    /// Quality score [0,1] — set after generation + review
    pub quality_score: f32,
    pub generated_at: u64,
}

impl TrainingExample {
    /// Format as Alpaca-style JSON for training frameworks (unsloth, axolotl, etc.)
    pub fn to_alpaca_json(&self) -> serde_json::Value {
        serde_json::json!({
            "system": self.system_prompt,
            "instruction": self.instruction,
            "output": self.response,
            "input": "",
        })
    }

    /// Format as ChatML for llama.cpp fine-tuning
    pub fn to_chatml(&self) -> String {
        format!(
            "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n{}<|im_end|>",
            self.system_prompt, self.instruction, self.response,
        )
    }
}

// ── LoRA training spec ────────────────────────────────────────────────────────

/// Configuration for a LoRA fine-tuning run on a sovereign's crystallized model.
/// Compatible with unsloth (Python) and llama.cpp finetune (C++).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoraTrainingSpec {
    pub sovereign_name: String,
    /// Input GGUF to fine-tune from
    pub base_model_path: String,
    /// Output path for LoRA adapter
    pub lora_adapter_output: String,
    /// Output path for merged GGUF (base + adapter)
    pub merged_gguf_output: String,

    // LoRA hyperparameters
    pub lora_rank: u32,    // r: typically 8-64. Higher = more capacity, more VRAM
    pub lora_alpha: u32,   // α: typically = rank. Scaling factor.
    pub lora_dropout: f32, // regularization
    pub target_modules: Vec<String>, // which weight matrices to adapt

    // Training hyperparameters
    pub learning_rate: f64,
    pub num_epochs: u32,
    pub batch_size: u32,
    pub gradient_accumulation_steps: u32,
    pub max_seq_length: u32,
    pub warmup_ratio: f32,

    // Data
    pub training_data_path: String,
    pub data_format: String, // "alpaca", "chatml", "sharegpt"

    // Metadata
    pub estimated_training_hours: f32,
    pub estimated_vram_gb: f32,
    pub notes: String,
}

impl LoraTrainingSpec {
    /// Generate a LoRA spec for a sovereign based on their task spec and model analysis.
    pub fn from_spec_and_analysis(
        spec: &SovereignTaskSpec,
        _analysis: Option<&ModelAnalysis>,
        models_dir: &std::path::Path,
        training_data_dir: &std::path::Path,
    ) -> Self {
        let base_path = models_dir
            .join(format!(
                "{}-qwen2.5-7b.gguf",
                spec.sovereign_name.to_lowercase()
            ))
            .to_string_lossy()
            .to_string();

        let adapter_path = models_dir
            .join(format!(
                "{}-lora-adapter.bin",
                spec.sovereign_name.to_lowercase()
            ))
            .to_string_lossy()
            .to_string();

        let merged_path = models_dir
            .join(format!(
                "{}-distilled.gguf",
                spec.sovereign_name.to_lowercase()
            ))
            .to_string_lossy()
            .to_string();

        let data_path = training_data_dir
            .join(format!(
                "{}-training.jsonl",
                spec.sovereign_name.to_lowercase()
            ))
            .to_string_lossy()
            .to_string();

        // LoRA rank based on model tier — smaller models need proportionally smaller rank
        let lora_rank = match spec.target_tier.target_params() {
            0..=200 => 8,
            201..=700 => 16,
            701..=2000 => 32,
            _ => 64,
        };

        // Estimate training cost
        let examples = spec.training_data_spec.min_examples;
        let params_m = spec.target_tier.target_params() as f32;
        // Rough estimate: 1M params × 1K examples × 1 epoch ≈ 1 minute on RTX 5070 Ti
        let estimated_hours = (params_m * examples as f32 / 1_000_000.0 / 60.0)
            * spec.training_data_spec.min_examples as f32
            / 1000.0;

        // VRAM: base model + LoRA gradients + optimizer states
        let base_vram_gb = spec.target_tier.vram_mb() as f32 / 1024.0;
        let lora_overhead_gb = (lora_rank as f32 * 2.0) / 1024.0; // approximate
        let estimated_vram_gb = base_vram_gb + lora_overhead_gb + 2.0; // +2GB buffer

        Self {
            sovereign_name: spec.sovereign_name.clone(),
            base_model_path: base_path,
            lora_adapter_output: adapter_path,
            merged_gguf_output: merged_path,
            lora_rank,
            lora_alpha: lora_rank,
            lora_dropout: 0.05,
            target_modules: vec![
                "q_proj".into(),
                "k_proj".into(),
                "v_proj".into(),
                "o_proj".into(),
                "gate_proj".into(),
                "up_proj".into(),
                "down_proj".into(),
            ],
            learning_rate: 2e-4,
            num_epochs: 3,
            batch_size: 4,
            gradient_accumulation_steps: 4,
            max_seq_length: spec.context_window_tokens,
            warmup_ratio: 0.05,
            training_data_path: data_path,
            data_format: "chatml".into(),
            estimated_training_hours: estimated_hours,
            estimated_vram_gb,
            notes: format!(
                "Fine-tune {} from {} blocks of foundation_v1. {} training examples. {}",
                spec.sovereign_name,
                spec.crystallization_blocks.len(),
                spec.training_data_spec.min_examples,
                spec.distillation_notes
                    .chars()
                    .take(120)
                    .collect::<String>(),
            ),
        }
    }

    /// Output as a Python script for unsloth (fastest LoRA training library).
    pub fn to_unsloth_script(&self) -> String {
        format!(
            r#"#!/usr/bin/env python3
"""
LoRA fine-tuning for {sovereign} using Unsloth.
Generated by Aaroneous Distillation Pipeline.
Run: pip install unsloth && python {sovereign_lower}_train.py
"""

from unsloth import FastLanguageModel
import torch
from datasets import load_dataset
from trl import SFTTrainer
from transformers import TrainingArguments

# Load the crystallized sovereign model
model, tokenizer = FastLanguageModel.from_pretrained(
    model_name=r"{base_model}",
    max_seq_length={max_seq_len},
    load_in_4bit=True,
)

# Apply LoRA
model = FastLanguageModel.get_peft_model(
    model,
    r={rank},
    target_modules={target_modules:?},
    lora_alpha={alpha},
    lora_dropout={dropout},
    bias="none",
    use_gradient_checkpointing=True,
)

# Load training data
dataset = load_dataset("json", data_files=r"{data_path}", split="train")

# Format as ChatML
def format_example(example):
    return {{
        "text": f"<|im_start|>system\n{{example['system']}}<|im_end|>\n"
                f"<|im_start|>user\n{{example['instruction']}}<|im_end|>\n"
                f"<|im_start|>assistant\n{{example['output']}}<|im_end|>"
    }}
dataset = dataset.map(format_example)

# Train
trainer = SFTTrainer(
    model=model,
    tokenizer=tokenizer,
    train_dataset=dataset,
    dataset_text_field="text",
    max_seq_length={max_seq_len},
    args=TrainingArguments(
        per_device_train_batch_size={batch_size},
        gradient_accumulation_steps={grad_accum},
        warmup_ratio={warmup},
        num_train_epochs={epochs},
        learning_rate={lr},
        fp16=not torch.cuda.is_bf16_supported(),
        bf16=torch.cuda.is_bf16_supported(),
        logging_steps=10,
        output_dir="./lora_output_{sovereign_lower}",
    ),
)
trainer.train()

# Save LoRA adapter
model.save_pretrained(r"{adapter_output}")
print(f"LoRA adapter saved: {adapter_output}")

# Optional: merge into GGUF
# model.save_pretrained_merged(r"{merged_output}", tokenizer, save_method="merged_4bit")
"#,
            sovereign = self.sovereign_name,
            sovereign_lower = self.sovereign_name.to_lowercase(),
            base_model = self.base_model_path,
            max_seq_len = self.max_seq_length,
            rank = self.lora_rank,
            target_modules = &self.target_modules,
            alpha = self.lora_alpha,
            dropout = self.lora_dropout,
            data_path = self.training_data_path,
            batch_size = self.batch_size,
            grad_accum = self.gradient_accumulation_steps,
            warmup = self.warmup_ratio,
            epochs = self.num_epochs,
            lr = self.learning_rate,
            adapter_output = self.lora_adapter_output,
            merged_output = self.merged_gguf_output,
        )
    }
}

// ── Distillation pipeline ─────────────────────────────────────────────────────

/// Generate LoRA training specs for the full sovereign roster.
pub fn generate_distillation_plan(
    models_dir: &std::path::Path,
    training_data_dir: &std::path::Path,
    only: Option<&[&str]>,
) -> Vec<LoraTrainingSpec> {
    let specs = sovereign_task_specs();

    specs
        .iter()
        .filter(|s| {
            only.is_none_or(|names| {
                names
                    .iter()
                    .any(|n| n.to_lowercase() == s.sovereign_name.to_lowercase())
            })
        })
        .map(|spec| {
            // Use task spec only — no weight analysis here (fast path).
            // For weight-derived hyperparameters, use /distillation/analyze/:sovereign.
            LoraTrainingSpec::from_spec_and_analysis(spec, None, models_dir, training_data_dir)
        })
        .collect()
}

/// Print the full distillation plan as a summary table.
pub fn print_distillation_plan(models_dir: &std::path::Path, training_data_dir: &std::path::Path) {
    let plans = generate_distillation_plan(models_dir, training_data_dir, None);
    println!(
        "{:<13} {:>5}  {:>5}  {:>6}  {:>8}  {:>5}",
        "Sovereign", "Rank", "VRAM", "Hours", "Examples", "Status"
    );
    println!("{}", "-".repeat(60));

    for p in &plans {
        let model_exists = std::path::Path::new(&p.base_model_path).exists();
        let data_exists = std::path::Path::new(&p.training_data_path).exists();
        let merged_exists = std::path::Path::new(&p.merged_gguf_output).exists();

        let status = if merged_exists {
            "done"
        } else if data_exists {
            "ready"
        } else if model_exists {
            "no-data"
        } else {
            "no-model"
        };

        let spec = spec_for(&p.sovereign_name);
        let examples = spec.map(|s| s.training_data_spec.min_examples).unwrap_or(0);

        println!(
            "{:<13} {:>5}  {:>4.1}G  {:>5.1}h  {:>8}  {:>7}",
            p.sovereign_name,
            p.lora_rank,
            p.estimated_vram_gb,
            p.estimated_training_hours,
            examples,
            status
        );
    }
    println!();
    println!("To generate training data: POST /distillation/generate-training-data");
    println!("To run fine-tuning:        aaroneous distill <sovereign>");
}
