/// DistillationPipeline — From sovereign task spec to LoRA training config.
///
/// The pipeline:
///   1. Read task spec (what the sovereign does, expected I/O)
///   2. Analyze the crystallized GGUF (what weights it has)
///   3. Generate synthetic training conversations using the foundation model
///   4. Output a LoRA training spec (compatible with unsloth/llama.cpp finetune)
///
/// This is the bridge between "we copied weight bytes" and "we have a model
/// that actually IS its sovereign persona."

use std::collections::HashMap;
use std::io::Write;
use serde::{Deserialize, Serialize};
use super::task_spec::{SovereignTaskSpec, sovereign_task_specs, spec_for};
#[allow(unused_imports)]
use super::analyzer::{GGUFAnalyzer, ModelAnalysis};

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

    let output_path = training_data_dir
        .join(format!("{}-training.jsonl", spec.sovereign_name.to_lowercase()));

    std::fs::create_dir_all(training_data_dir)
        .context("Failed to create training data directory")?;

    // Open in append mode — accumulate across runs
    let mut file = std::fs::OpenOptions::new()
        .create(true)
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

            // Fill the generation template
            let user_prompt = spec.training_data_spec.generation_prompt_template
                .replace("{{INTENT}}", synthetic_intent)
                .replace("{{N}}", "3")
                .replace("{{CONFLICT}}", synthetic_intent)
                .replace("{{TASK}}", synthetic_intent);

            // Call the foundation model
            let response = match llm.generate_domain_response(&system_prompt, &user_prompt, &spec.domain_key()).await {
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
            let quality_score = if response.contains('{') || response.contains('[') { 0.9 } else { 0.6 };

            let example = TrainingExample {
                id: format!("{}-{}-{}", spec.sovereign_name.to_lowercase(), capability.id, intent_idx),
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

/// Generate synthetic intents for a specific capability.
///
/// Returns varied natural-language instructions that a user might send to this
/// sovereign for this capability — the diversity drives generalization.
fn synthetic_intents_for(spec: &SovereignTaskSpec, capability_id: &str, count: u32) -> Vec<String> {
    // Start with the example from the capability if available
    let mut intents: Vec<String> = spec.capabilities.iter()
        .find(|c| c.id == capability_id)
        .and_then(|c| c.example.as_ref().map(|(input, _)| vec![input.clone()]))
        .unwrap_or_default();

    // Generic varied intent templates per capability type
    let templates: Vec<String> = match capability_id {
        "generate_design" => vec![
            "Design a {} dashboard for monitoring {}".into(),
            "Create a minimal {} interface for {}".into(),
            "Build a dark-mode {} UI for {}".into(),
            "Design a mobile-first {} for {}".into(),
        ],
        "decompose_intent" | "plan_tasks" => vec![
            "Plan how to {}".into(),
            "Break down the task: {}".into(),
            "Organize the following into subtasks: {}".into(),
            "Create a task graph for: {}".into(),
        ],
        "resolve_sync_conflict" => vec![
            r#"{"state_a":{"value":1,"ts":100},"state_b":{"value":2,"ts":90}}"#.into(),
            r#"{"state_a":{"config":"dark"},"state_b":{"config":"light"},"ts_a":200,"ts_b":150}"#.into(),
        ],
        "security_audit" | "threat_scan" => vec![
            "Audit this system: {}".into(),
            "Scan for vulnerabilities in: {}".into(),
            "Check security posture for: {}".into(),
        ],
        "archive_result" | "store_memory" => vec![
            "Archive the following execution result: {}".into(),
            "Store this decision for future recall: {}".into(),
        ],
        _ => vec![
            "Execute the following in your domain: {}".into(),
            "Handle this request: {}".into(),
            "Process: {}".into(),
        ],
    };

    // Fill templates with varied subjects drawn from a domain vocabulary
    let subjects = [
        "sovereign specialists", "AI agents", "neural networks",
        "Rust async systems", "Aaroneous hive", "GGUF models",
        "WebSocket connections", "SQLite databases", "federation events",
        "intent processing pipeline", "MaelstromUI panels", "O3DE game engine",
    ];

    let mut idx = 0usize;
    while intents.len() < count as usize {
        let template = &templates[idx % templates.len()];
        let subject = subjects[idx % subjects.len()];
        intents.push(template.replace("{}", subject));
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
            self.system_prompt,
            self.instruction,
            self.response,
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
    pub lora_rank: u32,        // r: typically 8-64. Higher = more capacity, more VRAM
    pub lora_alpha: u32,       // α: typically = rank. Scaling factor.
    pub lora_dropout: f32,     // regularization
    pub target_modules: Vec<String>,  // which weight matrices to adapt

    // Training hyperparameters
    pub learning_rate: f64,
    pub num_epochs: u32,
    pub batch_size: u32,
    pub gradient_accumulation_steps: u32,
    pub max_seq_length: u32,
    pub warmup_ratio: f32,

    // Data
    pub training_data_path: String,
    pub data_format: String,   // "alpaca", "chatml", "sharegpt"

    // Metadata
    pub estimated_training_hours: f32,
    pub estimated_vram_gb: f32,
    pub notes: String,
}

impl LoraTrainingSpec {
    /// Generate a LoRA spec for a sovereign based on their task spec and model analysis.
    pub fn from_spec_and_analysis(
        spec: &SovereignTaskSpec,
        analysis: Option<&ModelAnalysis>,
        models_dir: &std::path::Path,
        training_data_dir: &std::path::Path,
    ) -> Self {
        let base_path = models_dir
            .join(format!("{}-qwen2.5-7b.gguf", spec.sovereign_name.to_lowercase()))
            .to_string_lossy()
            .to_string();

        let adapter_path = models_dir
            .join(format!("{}-lora-adapter.bin", spec.sovereign_name.to_lowercase()))
            .to_string_lossy()
            .to_string();

        let merged_path = models_dir
            .join(format!("{}-distilled.gguf", spec.sovereign_name.to_lowercase()))
            .to_string_lossy()
            .to_string();

        let data_path = training_data_dir
            .join(format!("{}-training.jsonl", spec.sovereign_name.to_lowercase()))
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
            * spec.training_data_spec.min_examples as f32 / 1000.0;

        // VRAM: base model + LoRA gradients + optimizer states
        let base_vram_gb = spec.target_tier.vram_mb() as f32 / 1024.0;
        let lora_overhead_gb = (lora_rank as f32 * 2.0) / 1024.0;  // approximate
        let estimated_vram_gb = base_vram_gb + lora_overhead_gb + 2.0;  // +2GB buffer

        Self {
            sovereign_name: spec.sovereign_name.clone(),
            base_model_path: base_path,
            lora_adapter_output: adapter_path,
            merged_gguf_output: merged_path,
            lora_rank,
            lora_alpha: lora_rank,
            lora_dropout: 0.05,
            target_modules: vec![
                "q_proj".into(), "k_proj".into(), "v_proj".into(),
                "o_proj".into(), "gate_proj".into(), "up_proj".into(), "down_proj".into(),
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
                spec.distillation_notes.chars().take(120).collect::<String>(),
            ),
        }
    }

    /// Output as a Python script for unsloth (fastest LoRA training library).
    pub fn to_unsloth_script(&self) -> String {
        format!(r#"#!/usr/bin/env python3
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
    target_modules={target_modules},
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
            target_modules = format!("{:?}", self.target_modules),
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

    specs.iter()
        .filter(|s| {
            only.map_or(true, |names| {
                names.iter().any(|n| n.to_lowercase() == s.sovereign_name.to_lowercase())
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
    println!("{:<13} {:>5}  {:>5}  {:>6}  {:>8}  {:>5}",
        "Sovereign", "Rank", "VRAM", "Hours", "Examples", "Status");
    println!("{}", "-".repeat(60));

    for p in &plans {
        let model_exists = std::path::Path::new(&p.base_model_path).exists();
        let data_exists = std::path::Path::new(&p.training_data_path).exists();
        let merged_exists = std::path::Path::new(&p.merged_gguf_output).exists();

        let status = if merged_exists { "done" }
            else if data_exists { "ready" }
            else if model_exists { "no-data" }
            else { "no-model" };

        let spec = spec_for(&p.sovereign_name);
        let examples = spec.map(|s| s.training_data_spec.min_examples).unwrap_or(0);

        println!("{:<13} {:>5}  {:>4.1}G  {:>5.1}h  {:>8}  {:>7}",
            p.sovereign_name, p.lora_rank,
            p.estimated_vram_gb, p.estimated_training_hours,
            examples, status);
    }
    println!();
    println!("To generate training data: POST /distillation/generate-training-data");
    println!("To run fine-tuning:        aaroneous distill <sovereign>");
}
