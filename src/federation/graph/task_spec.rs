/// SovereignTaskSpec — Precise definition of each sovereign's I/O contract.
///
/// This is the document that drives the distillation pipeline.
/// Before we can bake a persona into a model, we need to know exactly:
///   - What input does this sovereign receive? (format, typical length)
///   - What output does it produce? (format, typical length)
///   - What reasoning steps does it need? (determines required depth)
///   - What's the maximum acceptable latency? (determines target size)
///
/// These specs drive:
///   1. Training data generation (what conversations to synthesize)
///   2. Target model size selection (token budget → parameter count)
///   3. Layer selection for crystallization (which blocks of the 7B to use)
///   4. Evaluation criteria (what "correct" output looks like)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Structured JSON with a defined schema
    Json { schema_example: String },
    /// Plain prose/markdown
    Prose { max_tokens: u32 },
    /// Binary classification (yes/no, safe/unsafe, etc.)
    Classification { classes: Vec<String> },
    /// Numeric score in a range
    Score { min: f32, max: f32, description: String },
    /// Structured list of items
    List { item_format: String, max_items: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelTier {
    /// 50M-300M params: binary/classification tasks, single-step lookups
    Nano { target_params_m: u32 },
    /// 300M-1.5B params: structured reasoning, JSON generation, short synthesis
    Micro { target_params_m: u32 },
    /// 1.5B-3B params: multi-step reasoning, medium context, complex output
    Standard { target_params_m: u32 },
    /// 3B-7B params: deep research, long synthesis, complex planning
    Deep { target_params_m: u32 },
}

impl ModelTier {
    pub fn target_params(&self) -> u32 {
        match self {
            ModelTier::Nano { target_params_m } => *target_params_m,
            ModelTier::Micro { target_params_m } => *target_params_m,
            ModelTier::Standard { target_params_m } => *target_params_m,
            ModelTier::Deep { target_params_m } => *target_params_m,
        }
    }

    pub fn tier_name(&self) -> &'static str {
        match self {
            ModelTier::Nano { .. } => "nano",
            ModelTier::Micro { .. } => "micro",
            ModelTier::Standard { .. } => "standard",
            ModelTier::Deep { .. } => "deep",
        }
    }

    /// Approximate VRAM usage at Q4_K_M quantization
    pub fn vram_mb(&self) -> u32 {
        // ~0.5 bytes per parameter at Q4, plus context overhead
        let params = self.target_params() as u32;
        (params as f32 * 0.6) as u32 + 256  // +256MB overhead
    }
}

/// A specific capability (task type) this sovereign handles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCapability {
    /// Short identifier
    pub id: String,
    /// Human description
    pub description: String,
    /// Input format and typical size
    pub input_format: String,
    pub typical_input_tokens: u32,
    /// Expected output
    pub output_format: OutputFormat,
    pub typical_output_tokens: u32,
    /// Frequency: how often this task runs per hour in production
    pub calls_per_hour_estimate: f32,
    /// Max acceptable latency in milliseconds
    pub latency_budget_ms: u32,
    /// Example input/output pair for training data generation
    pub example: Option<(String, String)>,
}

/// Full specification for a sovereign's model requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignTaskSpec {
    pub sovereign_name: String,
    pub internal_id: String,       // matches SpecialistId name
    pub domain: String,
    pub persona_summary: String,
    pub target_tier: ModelTier,
    pub capabilities: Vec<TaskCapability>,
    /// Context window needed (longer = more expensive to run)
    pub context_window_tokens: u32,
    /// Whether this sovereign needs to be always-resident in VRAM
    pub always_resident: bool,
    /// Notes on the distillation approach
    pub distillation_notes: String,
    /// Which blocks of the 7B foundation to use as starting point
    pub crystallization_blocks: Vec<usize>,
    /// Training data characteristics needed
    pub training_data_spec: TrainingDataSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataSpec {
    pub min_examples: u32,
    pub quality_criteria: Vec<String>,
    pub generation_prompt_template: String,
    pub synthetic_generation_model: String,
}

// ── The Roster ────────────────────────────────────────────────────────────────

/// Returns the task specifications for all 9 sovereigns.
/// These drive training data generation and target model sizing.
pub fn sovereign_task_specs() -> Vec<SovereignTaskSpec> {
    vec![
        // ── Ariel (Visionary) ──────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Ariel".into(),
            internal_id: "Visionary".into(),
            domain: "UI/UX design generation, Maelstrom spatial visualization".into(),
            persona_summary: "Creative director. Generates design variants as structured JSON with colors, typography, layout, confidence. Aesthetic judgment is the core skill.".into(),
            target_tier: ModelTier::Standard { target_params_m: 1500 },
            context_window_tokens: 2048,
            always_resident: true,  // Ariel runs on every design intent
            distillation_notes: "Fine-tune on UI/UX design conversations. Training data: intent → structured JSON variants with hex colors, font stacks, layout names. The Coder 7B base is good — coding and design share structured output patterns.".into(),
            crystallization_blocks: (0..20).map(|i| i * 28 / 20).collect(),
            capabilities: vec![
                TaskCapability {
                    id: "generate_design".into(),
                    description: "Generate UI/UX design variants for an intent".into(),
                    input_format: "Text: user intent + style hints".into(),
                    typical_input_tokens: 200,
                    output_format: OutputFormat::Json {
                        schema_example: "{\"variants\":[{\"title\":\"...\",\"colors\":[\"hex\"],\"typography\":\"font-stack\",\"layout\":\"card-grid\",\"confidence\":0.8,\"reasoning\":\"...\"}]}".into(),
                    },
                    typical_output_tokens: 400,
                    calls_per_hour_estimate: 20.0,
                    latency_budget_ms: 3000,
                    example: Some((
                        "Design a dashboard for monitoring AI specialists".into(),
                        "{\"variants\":[{\"title\":\"Constellation View\",\"colors\":[\"0a1520\",\"00ffcc\",\"c084fc\"],\"typography\":\"Consolas, monospace\",\"layout\":\"radial-constellation\",\"confidence\":0.88,\"reasoning\":\"Dark theme with sovereign color coding\"}]}".into(),
                    )),
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 500,
                quality_criteria: vec![
                    "Valid JSON output".into(),
                    "Hex colors are valid CSS".into(),
                    "Layout names are specific (not generic)".into(),
                    "Reasoning references the intent".into(),
                ],
                generation_prompt_template: "You are Ariel, a UI/UX design specialist. For the intent: {{INTENT}}, generate {{N}} design variants as JSON.".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Hermes (Omnipresent) ───────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Hermes".into(),
            internal_id: "Omnipresent".into(),
            domain: "P2P mesh sync, state consistency, device coordination".into(),
            persona_summary: "Messenger. Routes messages, resolves sync conflicts using CRDT logic, reports mesh state. Structured output is critical — JSON sync operations.".into(),
            target_tier: ModelTier::Micro { target_params_m: 500 },
            context_window_tokens: 1024,
            always_resident: false,
            distillation_notes: "Small and fast — sync decisions are formulaic. Fine-tune on conflict resolution scenarios. Key skill: given two conflicting states, output the CRDT merge resolution.".into(),
            crystallization_blocks: (0..16).map(|i| i * 28 / 16).collect(),
            capabilities: vec![
                TaskCapability {
                    id: "resolve_sync_conflict".into(),
                    description: "Resolve a state conflict between two nodes".into(),
                    input_format: "JSON: {state_a, state_b, timestamp_a, timestamp_b}".into(),
                    typical_input_tokens: 150,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"resolved_state":{},"strategy":"last_write_wins","confidence":0.9}"#.into(),
                    },
                    typical_output_tokens: 150,
                    calls_per_hour_estimate: 5.0,
                    latency_budget_ms: 500,
                    example: None,
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 300,
                quality_criteria: vec!["Valid JSON".into(), "Strategy is named".into()],
                generation_prompt_template: "You are Hermes. Resolve this sync conflict: {{CONFLICT}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Wen (Symbiotic) ────────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Wen".into(),
            internal_id: "Symbiotic".into(),
            domain: "Biometric classification, human state adaptation".into(),
            persona_summary: "Warm and attuned. Reads biometric signals and classifies user state. Output is simple structured state + recommendation. Called every 5 seconds — must be fast.".into(),
            target_tier: ModelTier::Nano { target_params_m: 70 },
            context_window_tokens: 256,
            always_resident: true,  // Called on every sensor tick
            distillation_notes: "This is the smallest sovereign. 70M parameters is enough for classification + short recommendation. Could even be a fine-tuned BERT-class model rather than a generative one. Training data: BPM/stress readings → state classification.".into(),
            crystallization_blocks: (0..8).collect(),
            capabilities: vec![
                TaskCapability {
                    id: "classify_user_state".into(),
                    description: "Classify user state from biometric readings".into(),
                    input_format: "JSON: {bpm: N, bpm_trend: up/down/stable, mem_pressure: N, time_of_day: str}".into(),
                    typical_input_tokens: 80,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"state":"focused","stress":0.3,"fatigue":0.2,"recommendation":"continue","defer_interruptions":false}"#.into(),
                    },
                    typical_output_tokens: 60,
                    calls_per_hour_estimate: 720.0,  // every 5 seconds
                    latency_budget_ms: 100,  // must be under 100ms
                    example: Some((
                        r#"{"bpm":95,"bpm_trend":"up","mem_pressure":0.7,"time_of_day":"late_night"}"#.into(),
                        r#"{"state":"stressed","stress":0.75,"fatigue":0.6,"recommendation":"defer_tasks","defer_interruptions":true}"#.into(),
                    )),
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 2000,  // needs many examples across biometric states
                quality_criteria: vec![
                    "State is one of: focused/relaxed/stressed/fatigued/recovering".into(),
                    "All numeric values in [0,1]".into(),
                    "Recommendation matches state".into(),
                ],
                generation_prompt_template: "You are Wen. Classify the user state from these biometrics: {{BIOMETRICS}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Kami (Phygital) ────────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Kami".into(),
            internal_id: "Phygital".into(),
            domain: "AR/VR spatial reasoning, OpenXR anchor placement".into(),
            persona_summary: "Spatial intelligence. Converts design intent into 3D anchor manifests. Understands spatial relationships, scale, and physical constraints.".into(),
            target_tier: ModelTier::Micro { target_params_m: 500 },
            context_window_tokens: 1024,
            always_resident: false,
            distillation_notes: "Spatial reasoning is a mid-depth skill. 500M should handle anchor manifest generation. Training data: intent + space description → JSON anchor manifest with coordinates.".into(),
            crystallization_blocks: (0..14).map(|i| i * 28 / 14).collect(),
            capabilities: vec![
                TaskCapability {
                    id: "generate_spatial_anchors".into(),
                    description: "Generate 3D spatial anchor placements for a design".into(),
                    input_format: "Text: design intent + space description".into(),
                    typical_input_tokens: 200,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"anchors":[{"id":"a1","label":"Sovereign Orb","position":{"x":0,"y":2.5,"z":-3},"scale":0.5,"rotation_deg":0,"visibility_pct":100}]}"#.into(),
                    },
                    typical_output_tokens: 300,
                    calls_per_hour_estimate: 5.0,
                    latency_budget_ms: 2000,
                    example: None,
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 400,
                quality_criteria: vec!["Valid JSON".into(), "Positions are plausible (y>0 for floor)".into()],
                generation_prompt_template: "You are Kami. Place these elements in 3D space: {{ELEMENTS}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Dionysus (Archivist) ───────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Dionysus".into(),
            internal_id: "Archivist".into(),
            domain: "Pattern extraction, memory consolidation, session archival".into(),
            persona_summary: "Memory keeper. Extracts patterns from execution histories, writes memory summaries, identifies recurring themes across sessions.".into(),
            target_tier: ModelTier::Micro { target_params_m: 700 },
            context_window_tokens: 2048,
            always_resident: false,
            distillation_notes: "Pattern extraction needs medium depth. Training data: list of events → extracted patterns as structured JSON. Also: session transcript → compressed memory summary.".into(),
            crystallization_blocks: {
                let mid = 28 / 4;
                (mid..mid + 12).collect()
            },
            capabilities: vec![
                TaskCapability {
                    id: "extract_patterns".into(),
                    description: "Extract recurring patterns from execution history".into(),
                    input_format: "JSON: list of {sovereign, action, status, output_preview}".into(),
                    typical_input_tokens: 400,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"patterns":[{"type":"recurring_success","description":"Ariel succeeds on design tasks when intent mentions color","confidence":0.85,"evidence_count":12}]}"#.into(),
                    },
                    typical_output_tokens: 300,
                    calls_per_hour_estimate: 2.0,
                    latency_budget_ms: 5000,
                    example: None,
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 300,
                quality_criteria: vec!["Patterns have confidence scores".into(), "Evidence count > 0".into()],
                generation_prompt_template: "You are Dionysus. Extract patterns from these executions: {{EXECUTIONS}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Merlin ─────────────────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Merlin".into(),
            internal_id: "Merlin".into(),
            domain: "Research synthesis, external knowledge, outbound queries".into(),
            persona_summary: "Knowledge synthesizer. Decomposes research questions, synthesizes multiple sources, returns structured intelligence reports.".into(),
            target_tier: ModelTier::Deep { target_params_m: 3000 },
            context_window_tokens: 8192,
            always_resident: false,  // Only loaded for research tasks
            distillation_notes: "Merlin needs the most depth — knowledge synthesis requires broad world knowledge. Use the full 24-block extraction. Training data: research question → structured synthesis with citations and confidence ratings.".into(),
            crystallization_blocks: (0..24).map(|i| i * 28 / 24).collect(),
            capabilities: vec![
                TaskCapability {
                    id: "synthesize_research".into(),
                    description: "Synthesize a research question into a structured intelligence report".into(),
                    input_format: "Text: research question + context".into(),
                    typical_input_tokens: 400,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"summary":"...","key_findings":[...],"confidence":0.8,"sources":["..."],"gaps":["..."]}"#.into(),
                    },
                    typical_output_tokens: 800,
                    calls_per_hour_estimate: 3.0,
                    latency_budget_ms: 15000,
                    example: None,
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 800,
                quality_criteria: vec!["Summary is specific".into(), "Confidence score present".into(), "Gaps identified".into()],
                generation_prompt_template: "You are Merlin. Research and synthesize: {{QUESTION}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Odin ───────────────────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Odin".into(),
            internal_id: "Odin".into(),
            domain: "Guild coordination, task decomposition, intent routing".into(),
            persona_summary: "Guild coordinator. Receives user intents and decomposes them into a dependency graph of subtasks, each assigned to the right sovereign.".into(),
            target_tier: ModelTier::Standard { target_params_m: 1500 },
            context_window_tokens: 4096,
            always_resident: true,  // Odin intercepts every intent
            distillation_notes: "The critical skill is structured decomposition — input is free text, output must be valid JSON with task graph. Upper-layer blocks for planning. Training data is the highest priority: hundreds of intent → task_dag examples across all domains.".into(),
            crystallization_blocks: (8..28).collect(),  // upper layers
            capabilities: vec![
                TaskCapability {
                    id: "decompose_intent".into(),
                    description: "Decompose a user intent into a dependency graph of sovereign tasks".into(),
                    input_format: "Text: user intent".into(),
                    typical_input_tokens: 100,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"tasks":[{"id":"t1","content":"...","assign_to":"Merlin","priority":"High","deps":[]},{"id":"t2","content":"...","assign_to":"Ariel","deps":["t1"]}]}"#.into(),
                    },
                    typical_output_tokens: 400,
                    calls_per_hour_estimate: 20.0,
                    latency_budget_ms: 2000,
                    example: Some((
                        "research the latest Rust async patterns and build a summary dashboard".into(),
                        r#"{"tasks":[{"id":"t1","content":"research Rust async runtime patterns 2025","assign_to":"Merlin","priority":"High","deps":[]},{"id":"t2","content":"synthesize findings into structured report","assign_to":"Merlin","priority":"Normal","deps":["t1"]},{"id":"t3","content":"design summary dashboard layout for the report","assign_to":"Ariel","priority":"Normal","deps":["t2"]},{"id":"t4","content":"archive research session and patterns","assign_to":"Dionysus","priority":"Low","deps":["t2"]}]}"#.into(),
                    )),
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 1000,  // Odin's training data is the foundation of everything
                quality_criteria: vec![
                    "Valid JSON with tasks array".into(),
                    "All assign_to values are sovereign names".into(),
                    "Dependencies reference task ids that exist".into(),
                    "No circular dependencies".into(),
                    "Priorities are: Critical/High/Normal/Low/Background".into(),
                ],
                generation_prompt_template: "You are Odin, the Guild coordinator. Decompose this intent into tasks for your sovereigns: {{INTENT}}. Return only JSON.".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Argus ──────────────────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Argus".into(),
            internal_id: "Argus".into(),
            domain: "Security audit, vulnerability scanning, secrets management".into(),
            persona_summary: "Security warden. Scans code/configs for vulnerabilities, classifies severity, reports actionable findings. Adversarial mindset.".into(),
            target_tier: ModelTier::Standard { target_params_m: 1500 },
            context_window_tokens: 4096,
            always_resident: false,
            distillation_notes: "Security pattern recognition spans early layers (syntax) and late layers (semantic intent). Lower+upper block selection from crystallization is correct. Training data: code/config → vulnerability report with CVSS-style severity.".into(),
            crystallization_blocks: {
                let mut blocks: Vec<usize> = (0..8).collect();
                blocks.extend(20..28);
                blocks
            },
            capabilities: vec![
                TaskCapability {
                    id: "audit_code".into(),
                    description: "Audit code for security vulnerabilities".into(),
                    input_format: "Text: code snippet or config".into(),
                    typical_input_tokens: 600,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"findings":[{"severity":"High","cve_pattern":"SQL_INJECTION","location":"line 42","description":"...","remediation":"..."}],"overall_risk":"High"}"#.into(),
                    },
                    typical_output_tokens: 400,
                    calls_per_hour_estimate: 5.0,
                    latency_budget_ms: 5000,
                    example: None,
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 600,
                quality_criteria: vec!["Severity is Critical/High/Medium/Low/Info".into(), "Remediation is specific".into()],
                generation_prompt_template: "You are Argus. Audit this for security issues: {{CODE}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },

        // ── Hephaestus ─────────────────────────────────────────────────────
        SovereignTaskSpec {
            sovereign_name: "Hephaestus".into(),
            internal_id: "Hephaestus".into(),
            domain: "Fabrication, build automation, maintenance, infrastructure expansion".into(),
            persona_summary: "Master craftsman. Writes build scripts, repairs broken systems, expands infrastructure. Code generation and system design are the core skills.".into(),
            target_tier: ModelTier::Standard { target_params_m: 2000 },
            context_window_tokens: 4096,
            always_resident: false,
            distillation_notes: "Coder 7B base is ideal — Hephaestus is the most code-centric sovereign. MLP-heavy selection is correct (FFN layers encode code logic). Training data: task description → implementation plan + code snippets.".into(),
            crystallization_blocks: (0..22).map(|i| i * 28 / 22).collect(),
            capabilities: vec![
                TaskCapability {
                    id: "plan_build".into(),
                    description: "Create a build/implementation plan for a task".into(),
                    input_format: "Text: task description + current system state".into(),
                    typical_input_tokens: 300,
                    output_format: OutputFormat::Json {
                        schema_example: r#"{"plan":{"steps":[{"id":"s1","action":"...","command":"...","expected_outcome":"..."}],"estimated_minutes":15,"risk_level":"Low"}}"#.into(),
                    },
                    typical_output_tokens: 500,
                    calls_per_hour_estimate: 3.0,
                    latency_budget_ms: 5000,
                    example: None,
                },
            ],
            training_data_spec: TrainingDataSpec {
                min_examples: 500,
                quality_criteria: vec!["Steps have actionable commands".into(), "Expected outcomes are verifiable".into()],
                generation_prompt_template: "You are Hephaestus. Plan how to build: {{TASK}}".into(),
                synthetic_generation_model: "foundation_v1.gguf".into(),
            },
        },
    ]
}

impl SovereignTaskSpec {
    /// Short domain key for LLM routing (matches GenericSpecialist domain labels).
    pub fn domain_key(&self) -> String {
        match self.sovereign_name.to_lowercase().as_str() {
            "ariel"      => "ui_design".into(),
            "hermes"     => "p2p_sync".into(),
            "wen"        => "biometric".into(),
            "kami"       => "phygital".into(),
            "dionysus"   => "archival".into(),
            "merlin"     => "research".into(),
            "odin"       => "task_orchestration".into(),
            "argus"      => "security_audit".into(),
            "hephaestus" => "construction".into(),
            other        => other.replace(' ', "_"),
        }
    }
}

/// Return the task spec for a specific sovereign by name.
pub fn spec_for(sovereign_name: &str) -> Option<SovereignTaskSpec> {
    sovereign_task_specs()
        .into_iter()
        .find(|s| s.sovereign_name.to_lowercase() == sovereign_name.to_lowercase())
}

/// Print a human-readable summary of all sovereign task specs.
pub fn print_roster_summary() {
    let specs = sovereign_task_specs();
    println!("{:<13} {:>8}  {:>7}  {:>8}  {}",
        "Sovereign", "Tier", "Params", "Latency", "Key capability");
    println!("{}", "-".repeat(75));
    for s in &specs {
        println!("{:<13} {:>8}  {:>5}M  {:>6}ms  {}",
            s.sovereign_name,
            s.target_tier.tier_name(),
            s.target_tier.target_params(),
            s.capabilities.first().map(|c| c.latency_budget_ms).unwrap_or(0),
            s.capabilities.first().map(|c| c.description.as_str()).unwrap_or(""),
        );
    }
    println!();
    let total_resident_vram: u32 = specs.iter()
        .filter(|s| s.always_resident)
        .map(|s| s.target_tier.vram_mb())
        .sum();
    let total_all_vram: u32 = specs.iter().map(|s| s.target_tier.vram_mb()).sum();
    println!("Always-resident VRAM: ~{}MB", total_resident_vram);
    println!("Full roster VRAM:     ~{}MB (not all loaded simultaneously)", total_all_vram);
}
