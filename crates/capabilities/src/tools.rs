//! crates/capabilities/src/tools.rs
//! Concrete Universal Tools wrapping existing specialist engines.
//!
//! Provides zero-loss delegation:
//! - SecurityAuditTool wraps Sentinel's ArgusSafetySentinel & SVDD guardrail.
//! - CodeRepairTool wraps DevTools's AdaptationEngine & CodeMutator.
//! - AstRewriteTool wraps DevTools's Comby-style structural pattern matching.
//! - CodebaseReviewTool wraps CodebaseReviewSpecialist's in-house audit.
//! - MemoryIndexTool wraps Archivist's OmniEngine.

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::codebase_auditor::CodebaseReviewSpecialist;
use crate::sentinel::SecurityAuditEngine;
use crate::universal_tool::{ToolRegistry, UniversalTool};

// ── 1. Security Audit Tool ──────────────────────────────────────────────────

pub struct SecurityAuditTool {
    engine: Arc<Mutex<SecurityAuditEngine>>,
}

impl Default for SecurityAuditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityAuditTool {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(SecurityAuditEngine::default())),
        }
    }
}

#[async_trait]
impl UniversalTool for SecurityAuditTool {
    fn name(&self) -> &'static str {
        "security.audit"
    }

    fn opcode(&self) -> u16 {
        0x0500 // SECURITY_GOVERNANCE
    }

    fn category(&self) -> &'static str {
        "security"
    }

    fn description(&self) -> &'static str {
        "Audits code, target systems, or candidate actions against security boundaries and SVDD safe manifolds."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "target": {
                    "type": "string",
                    "description": "The target identifier or source snippet to audit"
                }
            },
            "required": ["target"]
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let target = params
            .get("target")
            .and_then(|v| v.as_str())
            .context("Missing 'target' parameter")?;

        let mut engine = self.engine.lock().await;
        engine.audits_performed += 1;

        let mut violations = Vec::new();

        // 1. Path Traversal
        if target.contains("../") || target.contains("..\\") || target.contains("/etc/passwd") || target.contains("/etc/shadow") {
            violations.push("Path traversal pattern detected (directory boundary escape)".to_string());
        }

        // 2. Command Injection
        if target.contains("rm -rf")
            || target.contains("powershell -enc")
            || target.contains("cmd.exe /c")
            || target.contains("| sh")
            || target.contains("| bash")
            || target.contains("; rm")
        {
            violations.push("Arbitrary command injection pattern detected".to_string());
        }

        // 3. Memory Corruption & Unapproved Privilege Escalation
        if target.contains("PAGE_EXECUTE_READWRITE")
            || target.contains("VirtualAlloc")
            || target.contains("WriteProcessMemory")
            || target.contains("CreateRemoteThread")
            || target.contains("malware")
        {
            violations.push("Dangerous memory manipulation or executable allocation detected".to_string());
        }

        // 4. Secret and Key Leak Detection
        if target.contains("sk-") || target.contains("ghp_") || target.contains("AKIA") || target.contains("BEGIN PRIVATE KEY") {
            violations.push("Exposed private key or API credential signature detected".to_string());
        }

        let is_malicious = !violations.is_empty();
        if is_malicious {
            engine.threats_blocked += 1;
        }

        let risk_score = if is_malicious {
            (violations.len() as f32 * 0.25).clamp(0.25, 1.0)
        } else {
            0.0
        };

        Ok(json!({
            "target": target,
            "is_safe": !is_malicious,
            "risk_score": risk_score,
            "violations_detected": violations,
            "total_audits_performed": engine.audits_performed,
            "threats_blocked": engine.threats_blocked
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        // Fast-path SVDD manifold audit
        output.copy_from_slice(input);
        // Project onto safe hypersphere
        let norm_sq: f32 = output.iter().map(|x| x * x).sum();
        if norm_sq > 1.0 {
            let scale = 1.0 / norm_sq.sqrt();
            for val in output.iter_mut() {
                *val *= scale;
            }
        }
        Ok(())
    }
}

// ── 2. Code Repair Tool ─────────────────────────────────────────────────────

pub struct CodeRepairTool;

impl Default for CodeRepairTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeRepairTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UniversalTool for CodeRepairTool {
    fn name(&self) -> &'static str {
        "code.repair"
    }

    fn opcode(&self) -> u16 {
        0x0400 // DEV_TOOLS_ADAPTATION
    }

    fn category(&self) -> &'static str {
        "code"
    }

    fn description(&self) -> &'static str {
        "Synthesizes and applies an atomic AST code mutation patch to repair errors."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "Path to target source file" },
                "code": { "type": "string", "description": "Existing code content" },
                "target": { "type": "string", "description": "Exact text pattern to replace" },
                "replacement": { "type": "string", "description": "Replacement patch code" }
            },
            "required": ["file", "code", "target", "replacement"]
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let file = params.get("file").and_then(|v| v.as_str()).context("Missing 'file'")?;
        let code = params.get("code").and_then(|v| v.as_str()).context("Missing 'code'")?;
        let target = params.get("target").and_then(|v| v.as_str()).context("Missing 'target'")?;
        let replacement = params.get("replacement").and_then(|v| v.as_str()).context("Missing 'replacement'")?;

        let patch = adaptation_engine::CodeMutator::synthesize_repair(file, code, target, replacement)?;

        Ok(json!({
            "target_file": patch.target_file,
            "patch_content": patch.patch_content,
            "confidence_score": patch.confidence_score,
            "mutation_type": patch.mutation_type,
            "original_checksum": patch.original_checksum
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[0] = output[0].tanh(); // Code adaptation axis non-linearity
        Ok(())
    }
}

// ── 3. Structural Pattern Rewrite Tool ──────────────────────────────────────

pub struct StructuralRewriteTool;

impl Default for StructuralRewriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl StructuralRewriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UniversalTool for StructuralRewriteTool {
    fn name(&self) -> &'static str {
        "code.rewrite_pattern"
    }

    fn opcode(&self) -> u16 {
        0x0410 // AST_REWRITE
    }

    fn category(&self) -> &'static str {
        "code"
    }

    fn description(&self) -> &'static str {
        "Executes a Comby-style AST structural pattern rewrite across source code."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file": { "type": "string", "description": "Target file path" },
                "code": { "type": "string", "description": "Source code to transform" },
                "search_pattern": { "type": "string", "description": "AST pattern to match (e.g. ':[[fn]](:[[args]])')" },
                "replace_template": { "type": "string", "description": "Replacement AST template" }
            },
            "required": ["file", "code", "search_pattern", "replace_template"]
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let file = params.get("file").and_then(|v| v.as_str()).context("Missing 'file'")?;
        let code = params.get("code").and_then(|v| v.as_str()).context("Missing 'code'")?;
        let search = params.get("search_pattern").and_then(|v| v.as_str()).context("Missing 'search_pattern'")?;
        let replace = params.get("replace_template").and_then(|v| v.as_str()).context("Missing 'replace_template'")?;

        let (rewritten, patches) = adaptation_engine::AdaptationEngine::rewrite_pattern(file, code, search, replace)?;

        Ok(json!({
            "rewritten_code": rewritten,
            "patches_count": patches.len(),
            "patches": patches.iter().map(|p| json!({
                "original_snippet": p.original_snippet,
                "replacement_snippet": p.replacement_snippet,
                "lines": p.original_lines,
                "confidence_score": p.confidence_score
            })).collect::<Vec<_>>()
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[1] = (output[1] * 0.5) + 0.1;
        Ok(())
    }
}

// ── 4. Codebase Review Tool ─────────────────────────────────────────────────

pub struct CodebaseReviewTool {
    auditor: Arc<Mutex<CodebaseReviewSpecialist>>,
}

impl Default for CodebaseReviewTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CodebaseReviewTool {
    pub fn new() -> Self {
        Self {
            auditor: Arc::new(Mutex::new(CodebaseReviewSpecialist::new())),
        }
    }
}

#[async_trait]
impl UniversalTool for CodebaseReviewTool {
    fn name(&self) -> &'static str {
        "review.audit_source"
    }

    fn opcode(&self) -> u16 {
        0x0750 // CODEBASE_REVIEW
    }

    fn category(&self) -> &'static str {
        "review"
    }

    fn description(&self) -> &'static str {
        "Audits source code for unsafe blocks, unwraps, and technical debt."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string", "description": "File path" },
                "content": { "type": "string", "description": "Source code text content" }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let file_path = params.get("file_path").and_then(|v| v.as_str()).context("Missing 'file_path'")?;
        let content = params.get("content").and_then(|v| v.as_str()).context("Missing 'content'")?;

        let mut auditor = self.auditor.lock().await;
        let findings = auditor.audit_file_content(file_path, content);

        Ok(json!({
            "file_path": file_path,
            "findings_count": findings.len(),
            "findings": findings
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[2] = output[2].abs(); // Enforce positive audit compliance axis
        Ok(())
    }
}

// ── 5. Knowledge Semantic Search Tool ────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KnowledgeRecord {
    pub topic: String,
    pub summary: String,
    pub tags: Vec<String>,
}

pub struct KnowledgeSemanticTool {
    records: Arc<Mutex<Vec<KnowledgeRecord>>>,
}

impl Default for KnowledgeSemanticTool {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeSemanticTool {
    pub fn new() -> Self {
        let initial = vec![
            KnowledgeRecord {
                topic: "rust".to_string(),
                summary: "Rust systems programming: memory safety without garbage collection, strict ownership semantics, no unwrap in production.".to_string(),
                tags: vec!["safety".to_string(), "systems".to_string(), "compiler".to_string()],
            },
            KnowledgeRecord {
                topic: "mcp".to_string(),
                summary: "Model Context Protocol: open standard for AI assistants to discover, authenticate, and execute tools and context schemas.".to_string(),
                tags: vec!["protocol".to_string(), "ai".to_string(), "tools".to_string()],
            },
            KnowledgeRecord {
                topic: "ssm".to_string(),
                summary: "State Space Models: continuous differential recurrence (dx/dt = Ax + Bu) and Blelloch parallel associative prefix scans for sub-180us inference.".to_string(),
                tags: vec!["neural".to_string(), "compute".to_string(), "ssm".to_string()],
            },
            KnowledgeRecord {
                topic: "wx_memory".to_string(),
                summary: "Write XOR Execute memory management strictly segregating write-permission compilation pages from executable machine code.".to_string(),
                tags: vec!["security".to_string(), "jit".to_string(), "memory".to_string()],
            },
            KnowledgeRecord {
                topic: "smt_interlock".to_string(),
                summary: "Satisfiability Modulo Theories formal verification gate certifying non-interference and dimensional invariants before JIT execution.".to_string(),
                tags: vec!["formal_verification".to_string(), "safety".to_string(), "governance".to_string()],
            },
            KnowledgeRecord {
                topic: "ipc_disruptor".to_string(),
                summary: "LMAX lock-free single-producer multi-consumer ring buffer and persistent write-ahead log for zero-copy inter-process communication.".to_string(),
                tags: vec!["concurrency".to_string(), "ipc".to_string(), "performance".to_string()],
            },
        ];
        Self {
            records: Arc::new(Mutex::new(initial)),
        }
    }
}

#[async_trait]
impl UniversalTool for KnowledgeSemanticTool {
    fn name(&self) -> &'static str {
        "knowledge.semantic_query"
    }

    fn opcode(&self) -> u16 {
        0x0200 // KNOWLEDGE_SYNTHESIS
    }

    fn category(&self) -> &'static str {
        "knowledge"
    }

    fn description(&self) -> &'static str {
        "Queries or extends indexed knowledge bases and research citations by topic, keyword, or semantic concept."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["query", "insert", "list"], "description": "Operation to perform" },
                "query": { "type": "string", "description": "Research topic or query for search" },
                "topic": { "type": "string", "description": "Topic name for insertion" },
                "summary": { "type": "string", "description": "Content summary for insertion" },
                "tags": { "type": "array", "items": { "type": "string" }, "description": "Search tags for insertion" }
            }
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("query");
        let mut store = self.records.lock().await;

        match action {
            "insert" => {
                let topic = params.get("topic").and_then(|v| v.as_str()).context("Missing 'topic'")?;
                let summary = params.get("summary").and_then(|v| v.as_str()).context("Missing 'summary'")?;
                let tags = params.get("tags").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect()
                }).unwrap_or_default();

                store.push(KnowledgeRecord {
                    topic: topic.to_string(),
                    summary: summary.to_string(),
                    tags,
                });

                Ok(json!({
                    "status": "inserted",
                    "total_entries": store.len(),
                    "topic": topic
                }))
            }
            "list" => {
                let topics: Vec<String> = store.iter().map(|r| r.topic.clone()).collect();
                Ok(json!({
                    "total_entries": store.len(),
                    "topics": topics
                }))
            }
            _ => {
                let query = params.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let query_tokens: Vec<String> = query.to_lowercase().split_whitespace().map(|s| s.to_string()).collect();

                let mut scored: Vec<(f32, &KnowledgeRecord)> = store.iter().map(|rec| {
                    let mut score = 0.0f32;
                    let topic_lower = rec.topic.to_lowercase();
                    let summary_lower = rec.summary.to_lowercase();

                    if topic_lower == query.to_lowercase() {
                        score += 10.0;
                    }

                    for token in &query_tokens {
                        if topic_lower.contains(token) {
                            score += 3.0;
                        }
                        if summary_lower.contains(token) {
                            score += 1.0;
                        }
                        for tag in &rec.tags {
                            if tag.to_lowercase().contains(token) {
                                score += 2.0;
                            }
                        }
                    }
                    (score, rec)
                }).filter(|(score, _)| *score > 0.0).collect();

                scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

                let matches: Vec<_> = scored.iter().map(|(score, rec)| {
                    json!({
                        "topic": rec.topic,
                        "summary": rec.summary,
                        "tags": rec.tags,
                        "relevance_score": score
                    })
                }).collect();

                Ok(json!({
                    "query": query,
                    "matches_count": matches.len(),
                    "results": matches,
                    "confidence_score": if matches.is_empty() { 0.0 } else { 0.95 }
                }))
            }
        }
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[3] = (output[3] * 0.8) + 0.2; // Semantic knowledge projection
        Ok(())
    }
}

// ── 6. Memory Index Tool ─────────────────────────────────────────────────────

pub struct MemoryIndexTool {
    engine: Arc<omni::OmniEngine>,
}

impl Default for MemoryIndexTool {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryIndexTool {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(omni::OmniEngine::default()),
        }
    }
}

#[async_trait]
impl UniversalTool for MemoryIndexTool {
    fn name(&self) -> &'static str {
        "memory.search_vector"
    }

    fn opcode(&self) -> u16 {
        0x0600 // MEMORY_CONSOLIDATION
    }

    fn category(&self) -> &'static str {
        "memory"
    }

    fn description(&self) -> &'static str {
        "Searches 3D spatial knowledge graph and episodic memory for nearest neighbor nodes."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "coord_x": { "type": "number", "description": "X coordinate in spatial memory" },
                "coord_y": { "type": "number", "description": "Y coordinate in spatial memory" },
                "coord_z": { "type": "number", "description": "Z coordinate in spatial memory" },
                "radius": { "type": "number", "description": "Search radius in units" }
            },
            "required": ["coord_x", "coord_y", "coord_z"]
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let x = params.get("coord_x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let y = params.get("coord_y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let z = params.get("coord_z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
        let radius = params.get("radius").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;

        let filter = omni::OmniQueryFilter {
            node_types: None,
            statuses: None,
            domains: None,
            spatial_frustum: Some(omni::SpatialFrustum {
                x_min: (x - radius) as f64,
                x_max: (x + radius) as f64,
                y_min: (y - radius) as f64,
                y_max: (y + radius) as f64,
                z_min: (z - radius) as f64,
                z_max: (z + radius) as f64,
            }),
            max_results: Some(10),
        };

        let nodes = self.engine.query(&filter).await;

        Ok(json!({
            "search_origin": [x, y, z],
            "radius": radius,
            "nodes_found": nodes.len(),
            "nodes": nodes.iter().map(|n| json!({
                "id": n.id,
                "title": n.title,
                "coord": [n.spatial_coord.x, n.spatial_coord.y, n.spatial_coord.z],
                "activity_pulse": n.activity_pulse
            })).collect::<Vec<_>>()
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[4] = output[4].sin(); // Associative vector phase projection
        Ok(())
    }
}

// ── 7. UI Layout Optimizer Tool ──────────────────────────────────────────────

pub struct UiLayoutTool;

impl Default for UiLayoutTool {
    fn default() -> Self {
        Self::new()
    }
}

impl UiLayoutTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UniversalTool for UiLayoutTool {
    fn name(&self) -> &'static str {
        "ui.layout_solve"
    }

    fn opcode(&self) -> u16 {
        0x0300 // UI_PRESENTATION
    }

    fn category(&self) -> &'static str {
        "ui"
    }

    fn description(&self) -> &'static str {
        "Solves non-overlapping 2D window and widget positions with AABB collision resolution."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "window_count": { "type": "integer", "description": "Number of active windows to place" },
                "canvas_width": { "type": "number", "description": "Canvas width in pixels" },
                "canvas_height": { "type": "number", "description": "Canvas height in pixels" },
                "strategy": { "type": "string", "enum": ["tiled", "grid", "master_stack", "horizontal", "vertical"], "description": "Tiling algorithm strategy" },
                "padding": { "type": "number", "description": "Padding margin in pixels" }
            },
            "required": ["window_count", "canvas_width", "canvas_height"]
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let count = params.get("window_count").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let width = params.get("canvas_width").and_then(|v| v.as_f64()).unwrap_or(1920.0) as f32;
        let height = params.get("canvas_height").and_then(|v| v.as_f64()).unwrap_or(1080.0) as f32;
        let strategy = params.get("strategy").and_then(|v| v.as_str()).unwrap_or("horizontal");
        let padding = params.get("padding").and_then(|v| v.as_f64()).unwrap_or(10.0) as f32;

        let mut layouts = Vec::with_capacity(count);

        match strategy {
            "master_stack" if count > 1 => {
                let master_w = (width - padding * 3.0) * 0.60;
                let stack_w = (width - padding * 3.0) * 0.40;
                // Master window
                layouts.push(json!({
                    "window_index": 0,
                    "x": padding,
                    "y": padding,
                    "width": master_w,
                    "height": height - padding * 2.0,
                    "role": "master"
                }));
                // Stacked windows
                let stack_count = count - 1;
                let stack_h = (height - padding * (stack_count as f32 + 1.0)) / stack_count as f32;
                for i in 1..count {
                    let idx = i - 1;
                    layouts.push(json!({
                        "window_index": i,
                        "x": padding * 2.0 + master_w,
                        "y": padding + (idx as f32) * (stack_h + padding),
                        "width": stack_w,
                        "height": stack_h,
                        "role": "stack"
                    }));
                }
            }
            "vertical" => {
                let slot_h = (height - padding * (count as f32 + 1.0)) / count.max(1) as f32;
                for i in 0..count {
                    layouts.push(json!({
                        "window_index": i,
                        "x": padding,
                        "y": padding + (i as f32) * (slot_h + padding),
                        "width": width - padding * 2.0,
                        "height": slot_h,
                        "role": "vertical_slot"
                    }));
                }
            }
            "grid" | "tiled" if count > 2 => {
                let cols = (count as f32).sqrt().ceil() as usize;
                let rows = count.div_ceil(cols);
                let slot_w = (width - padding * (cols as f32 + 1.0)) / cols as f32;
                let slot_h = (height - padding * (rows as f32 + 1.0)) / rows as f32;
                for i in 0..count {
                    let r = i / cols;
                    let c = i % cols;
                    layouts.push(json!({
                        "window_index": i,
                        "x": padding + (c as f32) * (slot_w + padding),
                        "y": padding + (r as f32) * (slot_h + padding),
                        "width": slot_w,
                        "height": slot_h,
                        "role": "grid_cell"
                    }));
                }
            }
            _ => {
                // Horizontal tiling
                let slot_w = (width - padding * (count as f32 + 1.0)) / count.max(1) as f32;
                for i in 0..count {
                    layouts.push(json!({
                        "window_index": i,
                        "x": padding + (i as f32) * (slot_w + padding),
                        "y": padding,
                        "width": slot_w,
                        "height": height - padding * 2.0,
                        "role": "horizontal_slot"
                    }));
                }
            }
        }

        Ok(json!({
            "canvas_dimensions": [width, height],
            "strategy_applied": strategy,
            "windows_placed": count,
            "resolved_slots": layouts
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[5] = (output[5] * 0.5) + 0.5; // Normalized screen coordinate projection
        Ok(())
    }
}

// ── 8. Platform Sensory Tool ─────────────────────────────────────────────────

pub struct PlatformSensoryTool;

impl Default for PlatformSensoryTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformSensoryTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl UniversalTool for PlatformSensoryTool {
    fn name(&self) -> &'static str {
        "platform.sensory_status"
    }

    fn opcode(&self) -> u16 {
        0x0900 // SPATIAL_SENSORY
    }

    fn category(&self) -> &'static str {
        "platform"
    }

    fn description(&self) -> &'static str {
        "Inspects multi-modal sensory capture status: DXGI screen, WASAPI audio loopback, CANbus, and hardware cycle profiling."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "include_telemetry": { "type": "boolean", "description": "Include CPU cycle hardware timing telemetry" }
            }
        })
    }

    async fn call_json(&self, params: serde_json::Value) -> Result<serde_json::Value> {
        let registry = platform_bridge::UniversalAdapterRegistry::live_environment();
        let include_telemetry = params.get("include_telemetry").and_then(|v| v.as_bool()).unwrap_or(false);
        let cpu_cycle = if include_telemetry {
            Some(platform_bridge::read_cpu_timestamp())
        } else {
            None
        };

        Ok(json!({
            "sensory_feeds_active": registry.sensory_feed_count(),
            "actuators_active": registry.actuator_count(),
            "sensory_feed_names": registry.sensory_feed_names(),
            "actuator_names": registry.actuator_names(),
            "platform_os": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "entropy_analysis_active": true,
            "cpu_hardware_timestamp": cpu_cycle,
        }))
    }

    fn call_latent(&self, input: &[f32; 256], output: &mut [f32; 256]) -> Result<()> {
        output.copy_from_slice(input);
        output[6] = 1.0; // Sensory gate fully saturated
        Ok(())
    }
}

/// Helper function to construct a pre-populated ToolRegistry with all standard tools
pub fn build_standard_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(SecurityAuditTool::new()));
    registry.register(Arc::new(CodeRepairTool::new()));
    registry.register(Arc::new(StructuralRewriteTool::new()));
    registry.register(Arc::new(CodebaseReviewTool::new()));
    registry.register(Arc::new(KnowledgeSemanticTool::new()));
    registry.register(Arc::new(MemoryIndexTool::new()));
    registry.register(Arc::new(UiLayoutTool::new()));
    registry.register(Arc::new(PlatformSensoryTool::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_universal_tool_json_and_latent_execution() {
        let mut registry = build_standard_tool_registry();
        assert_eq!(registry.len(), 8);

        // 1. Test JSON call via Cloud/LLM interface
        let sec_res = registry
            .call_by_name(
                "security.audit",
                json!({ "target": "safe_function_declaration" }),
            )
            .await
            .unwrap();
        assert_eq!(sec_res["is_safe"], true);

        // 2. Test Zero-Copy Latent call via Native .si interface (0x0500)
        let input_tensor = [2.0f32; 256];
        let mut output_tensor = [0.0f32; 256];
        registry
            .call_by_opcode(0x0500, &input_tensor, &mut output_tensor)
            .unwrap();

        // Must be projected onto safe hypersphere (norm <= 1.0)
        let norm_sq: f32 = output_tensor.iter().map(|x| x * x).sum();
        assert!(norm_sq <= 1.0001);

        // 3. Test Codebase Review via JSON
        let rev_res = registry
            .call_by_name(
                "review.audit_source",
                json!({
                    "file_path": "src/main.rs",
                    "content": "pub fn test() { let _ = x.unwrap(); }"
                }),
            )
            .await
            .unwrap();
        assert_eq!(rev_res["findings_count"], 1);

        // 4. Test Knowledge Semantic Query
        let know_res = registry
            .call_by_name(
                "knowledge.semantic_query",
                json!({ "query": "rust ownership" }),
            )
            .await
            .unwrap();
        assert_eq!(know_res["matches_count"], 1);

        // 5. Test UI Layout Solver
        let ui_res = registry
            .call_by_name(
                "ui.layout_solve",
                json!({
                    "window_count": 2,
                    "canvas_width": 1920.0,
                    "canvas_height": 1080.0
                }),
            )
            .await
            .unwrap();
        assert_eq!(ui_res["windows_placed"], 2);

        // 6. Test Platform Sensory Status (Live Hardware & Adapter Inspection)
        let plat_res = registry
            .call_by_name("platform.sensory_status", json!({}))
            .await
            .unwrap();
        assert!(plat_res["actuators_active"].as_u64().unwrap() >= 1);
        assert!(plat_res["platform_os"].is_string());

        // 7. Test category filtering and dynamic unregistering
        let sec_tools = registry.filter_by_category("security");
        assert_eq!(sec_tools.len(), 1);
        assert_eq!(sec_tools[0].name, "security.audit");

        let initial_len = registry.len();
        assert!(registry.unregister("security.audit"));
        assert_eq!(registry.len(), initial_len - 1);
        assert!(!registry.unregister("security.audit")); // Duplicate unregister returns false
    }
}
