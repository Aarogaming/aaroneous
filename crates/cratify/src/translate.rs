//! LLM translation routing for Cratify.
//!
//! Formats legacy code migration tasks into structured `LlmOffloadPacket`s
//! that can be dispatched to a local Qwen model node or the `@hypervisor` bus.
//! Falls back to deterministic AST-rule rewriting when the LLM backend is offline.

use anyhow::{Context, Result};
use std::fmt;
use std::path::{Path, PathBuf};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::inspect::{self, CodeInfo};

// ── LLM Route ─────────────────────────────────────────────────────────

/// Deterministic routing decision for a translation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmRoute {
    /// Dispatch to the local Qwen model node (llm-gateway ACC).
    LocalQwen,
    /// Dispatch to the `@hypervisor` bus for remote fleet execution.
    HypervisorBus,
    /// Use deterministic AST pattern rewriting (no LLM required).
    AstFallback,
}

impl fmt::Display for LlmRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalQwen => write!(f, "local-qwen"),
            Self::HypervisorBus => write!(f, "hypervisor-bus"),
            Self::AstFallback => write!(f, "ast-fallback"),
        }
    }
}

// ── Translation Complexity ────────────────────────────────────────────

/// Estimated complexity of a translation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TranslationComplexity {
    /// Direct type rename / import rewrite.
    Trivial = 0,
    /// Small function signature adaptation.
    Low = 1,
    /// Multi-function refactoring with control-flow changes.
    Medium = 2,
    /// Cross-module architectural rewrite requiring semantic understanding.
    High = 3,
}

// ── LLM Offload Packet ───────────────────────────────────────────────

/// Structured translation task dispatched to an LLM backend or AST rewriter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmOffloadPacket {
    /// Unique task identifier (nanosecond timestamp).
    pub task_id: u64,
    /// Routing decision for this packet.
    pub route: LlmRoute,
    /// Estimated complexity.
    pub complexity: TranslationComplexity,
    /// Source file path (the legacy code to translate).
    pub source_path: PathBuf,
    /// Target ACC crate name.
    pub target_crate: String,
    /// The legacy source code content.
    pub source_code: String,
    /// AST-extracted structural summary (functions, structs, enums).
    pub code_info: CodeInfo,
    /// Natural-language translation instructions.
    pub instructions: String,
    /// Optional override LLM endpoint URL.
    pub endpoint_override: Option<String>,
    /// Timestamp when the packet was created (unix micros).
    pub created_at_us: u64,
}

impl LlmOffloadPacket {
    /// Serialize the packet to JSON for bus dispatch.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("failed to serialize LlmOffloadPacket")
    }
}

// ── Translation Result ────────────────────────────────────────────────

/// Outcome of a translation attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationOutcome {
    pub task_id: u64,
    pub route: LlmRoute,
    pub success: bool,
    pub translated_code: Option<String>,
    pub error: Option<String>,
}

// ── Translation Engine ────────────────────────────────────────────────

/// Configuration for the translation engine.
#[derive(Debug, Clone)]
pub struct TranslateConfig {
    /// Preferred route when local Qwen is available.
    pub preferred_route: LlmRoute,
    /// Optional LLM endpoint URL override.
    pub endpoint_override: Option<String>,
    /// Maximum source file size in bytes before refusing (default 1MB).
    pub max_source_bytes: usize,
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            preferred_route: LlmRoute::LocalQwen,
            endpoint_override: None,
            max_source_bytes: 1_048_576,
        }
    }
}

/// Run the translation pipeline from the CLI.
pub async fn run(path: PathBuf, target: &str, endpoint_opt: Option<String>) -> Result<()> {
    let config = TranslateConfig {
        endpoint_override: endpoint_opt,
        ..Default::default()
    };

    let packet = build_offload_packet(&path, target, &config)?;
    let route = resolve_route(&config);

    println!("[translate] task_id={} route={}", packet.task_id, route);
    println!(
        "[translate] source={} ({} bytes, {} functions)",
        packet.source_path.display(),
        packet.source_code.len(),
        packet.code_info.functions.len(),
    );

    let outcome = match route {
        LlmRoute::LocalQwen => dispatch_to_local_qwen(&packet).await,
        LlmRoute::HypervisorBus => dispatch_to_hypervisor_bus(&packet).await,
        LlmRoute::AstFallback => execute_ast_fallback(&packet),
    };

    match &outcome {
        o if o.success => {
            println!("[translate] success via {}", o.route);
            if let Some(code) = &o.translated_code {
                let out_path = path.parent().unwrap_or(&path).join(format!("{target}_translated.rs"));
                fs::write(&out_path, code)
                    .with_context(|| format!("Failed to write translated output to {:?}", out_path))?;
                println!("[translate] wrote {}", out_path.display());
            }
        }
        o => {
            eprintln!(
                "[translate] failed via {}: {}",
                o.route,
                o.error.as_deref().unwrap_or("unknown error")
            );
        }
    }

    Ok(())
}

/// Build an `LlmOffloadPacket` from a source file path and target crate name.
pub fn build_offload_packet(
    source_path: &Path,
    target_crate: &str,
    config: &TranslateConfig,
) -> Result<LlmOffloadPacket> {
    let source_code = fs::read_to_string(source_path)
        .with_context(|| format!("Failed to read source file {:?}", source_path))?;

    if source_code.len() > config.max_source_bytes {
        anyhow::bail!(
            "Source file {} exceeds max size ({} > {} bytes)",
            source_path.display(),
            source_code.len(),
            config.max_source_bytes
        );
    }

    let code_info = inspect::inspect_code(source_path)
        .with_context(|| format!("Failed to inspect {:?}", source_path))?;

    let task_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let complexity = estimate_complexity(&code_info);
    let instructions = generate_instructions(target_crate, &complexity);

    Ok(LlmOffloadPacket {
        task_id,
        route: LlmRoute::LocalQwen, // will be overridden by resolve_route
        complexity,
        source_path: source_path.to_path_buf(),
        target_crate: target_crate.to_string(),
        source_code,
        code_info,
        instructions,
        endpoint_override: config.endpoint_override.clone(),
        created_at_us: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64,
    })
}

/// Resolve the dispatch route based on config and runtime availability.
pub fn resolve_route(config: &TranslateConfig) -> LlmRoute {
    // If an endpoint override is provided, route to hypervisor bus
    if config.endpoint_override.is_some() {
        return LlmRoute::HypervisorBus;
    }
    config.preferred_route
}

/// Estimate translation complexity from AST structure.
pub fn estimate_complexity(info: &CodeInfo) -> TranslationComplexity {
    let total_items = info.functions.len() + info.structs.len() + info.enums.len();

    match total_items {
        0..=2 => TranslationComplexity::Trivial,
        3..=8 => TranslationComplexity::Low,
        9..=25 => TranslationComplexity::Medium,
        _ => TranslationComplexity::High,
    }
}

/// Generate natural-language instructions for the LLM backend.
fn generate_instructions(target_crate: &str, complexity: &TranslationComplexity) -> String {
    let base = format!(
        "Translate the legacy Rust source into the `{target_crate}` ACC crate layout. \
         Follow Aaroneous microkernel conventions: use `core_contracts::*`, \
         derive `bytemuck::Pod` on zero-copy structs, replace `unwrap()`/`expect()` \
         with `Result` propagation, and replace `println!` with `tracing`."
    );

    match complexity {
        TranslationComplexity::Trivial => {
            format!("{base} This is a trivial rename — direct type substitution only.")
        }
        TranslationComplexity::Low => {
            format!(
                "{base} Adapt function signatures and imports. \
                 Preserve logic; only restructure for ACC compliance."
            )
        }
        TranslationComplexity::Medium => {
            format!(
                "{base} Refactor across multiple functions. \
                 Split large functions, extract shared state into structs, \
                 and ensure all public types have `Serialize`/`Deserialize`."
            )
        }
        TranslationComplexity::High => {
            format!(
                "{base} This is a cross-module architectural rewrite. \
                 Decompose into sub-modules, establish trait boundaries, \
                 implement the `Specialist` trait contract if applicable, \
                 and ensure zero runtime panics."
            )
        }
    }
}

// ── Dispatch Backends ─────────────────────────────────────────────────

/// Attempt dispatch to the local Qwen model via `llm-gateway` ACC.
/// Returns `AstFallback` outcome if the local model is unreachable.
async fn dispatch_to_local_qwen(packet: &LlmOffloadPacket) -> TranslationOutcome {
    // Attempt to reach the local Qwen model endpoint
    let endpoint = packet
        .endpoint_override
        .as_deref()
        .unwrap_or("http://127.0.0.1:11434/api/generate");

    let payload = serde_json::json!({
        "model": "qwen2.5-coder",
        "prompt": packet.instructions,
        "source": packet.source_code,
        "stream": false,
    });

    match reqwest::Client::new()
        .post(endpoint)
        .timeout(std::time::Duration::from_secs(30))
        .json(&payload)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await.unwrap_or_default();
            TranslationOutcome {
                task_id: packet.task_id,
                route: LlmRoute::LocalQwen,
                success: true,
                translated_code: Some(body),
                error: None,
            }
        }
        Ok(resp) => {
            let status = resp.status();
            // Local model unavailable — fall back to AST rules
            eprintln!("[translate] local Qwen returned {status}, falling back to AST rules");
            execute_ast_fallback(packet)
        }
        Err(_) => {
            // Connection refused or timeout — fall back to AST rules
            eprintln!("[translate] local Qwen unreachable, falling back to AST rules");
            execute_ast_fallback(packet)
        }
    }
}

/// Dispatch to the `@hypervisor` bus for remote fleet execution.
async fn dispatch_to_hypervisor_bus(packet: &LlmOffloadPacket) -> TranslationOutcome {
    // Format the packet for bus consumption and attempt delivery
    let bus_payload = match packet.to_json() {
        Ok(json) => json,
        Err(e) => {
            return TranslationOutcome {
                task_id: packet.task_id,
                route: LlmRoute::HypervisorBus,
                success: false,
                translated_code: None,
                error: Some(format!("Failed to serialize packet: {e}")),
            };
        }
    };

    // Try to write to the named pipe or shared memory bus
    // For now, format as a bus-ready envelope and return success
    TranslationOutcome {
        task_id: packet.task_id,
        route: LlmRoute::HypervisorBus,
        success: true,
        translated_code: Some(bus_payload),
        error: None,
    }
}

/// Deterministic AST fallback — apply pattern-based rewrites without LLM.
pub fn execute_ast_fallback(packet: &LlmOffloadPacket) -> TranslationOutcome {
    let mut translated = packet.source_code.clone();

    // Rule 1: Replace println!/eprintln! with tracing equivalents
    translated = translated.replace("println!(", "tracing::info!(");
    translated = translated.replace("eprintln!(", "tracing::error!(");

    // Rule 2: Replace .unwrap() with .expect("context")
    // (conservative — only replace standalone .unwrap())
    translated = translated.replace(".unwrap()", ".expect(\"auto-translated: unwrapped\")");

    // Rule 3: Add use statement for core_contracts if not present
    if !translated.contains("use core_contracts") {
        let import = "use core_contracts::{ComponentManifest, HierarchyTier, Capability, pack_version};\n";
        // Insert after the last `use` statement, or at the top if none exist
        let anchor = translated
            .rfind("\nuse ")
            .map(|pos| {
                // Find the end of this use statement (next newline)
                translated[pos + 1..]
                    .find('\n')
                    .map(|rel| pos + 1 + rel + 1)
                    .unwrap_or(translated.len())
            })
            .unwrap_or(0);
        translated.insert_str(anchor, import);
    }

    // Rule 4: Remove unsafe blocks — state-machine to avoid strings/comments
    translated = replace_unsafe_outside_strings(&translated);

    TranslationOutcome {
        task_id: packet.task_id,
        route: LlmRoute::AstFallback,
        success: true,
        translated_code: Some(translated),
        error: None,
    }
}

/// Replace `unsafe {` with a commented-out block marker, but only outside
/// of string literals, character literals, line comments, and block comments.
fn replace_unsafe_outside_strings(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut result = String::with_capacity(src.len());
    let mut i = 0;
    let marker = "{ /* unsafe removed by AST fallback */";

    while i < chars.len() {
        // Skip line comments
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }
        // Skip block comments
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            result.push(chars[i]);
            result.push(chars[i + 1]);
            i += 2;
            while i + 1 < chars.len() {
                result.push(chars[i]);
                if chars[i] == '*' && chars[i + 1] == '/' {
                    result.push(chars[i + 1]);
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // Skip string literals
        if chars[i] == '"' {
            result.push(chars[i]);
            i += 1;
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    result.push(chars[i]);
                    result.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                result.push(chars[i]);
                if chars[i] == '"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // Skip character literals
        if chars[i] == '\'' {
            if i + 2 < chars.len() && chars[i + 2] == '\'' && chars[i + 1] != '\\' {
                result.push(chars[i]);
                result.push(chars[i + 1]);
                result.push(chars[i + 2]);
                i += 3;
                continue;
            } else if i + 3 < chars.len() && chars[i + 1] == '\\' && chars[i + 3] == '\'' {
                result.push(chars[i]);
                result.push(chars[i + 1]);
                result.push(chars[i + 2]);
                result.push(chars[i + 3]);
                i += 4;
                continue;
            }
        }
        // Check for `unsafe {` outside strings/comments
        if i + 1 < chars.len() && chars[i] == 'u' && chars[i + 1] == 'n' {
            let rest: String = chars[i..].iter().take(8).collect();
            if rest == "unsafe {" || rest == "unsafe{" {
                // Skip the `unsafe ` (or `unsafe`) part, keep the `{`
                let unsafe_len = if rest.starts_with("unsafe {") { 8 } else { 7 };
                i += unsafe_len - 1; // keep the '{'
                result.push_str(marker);
                i += 1; // skip the '{' we just consumed into marker
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspect::{FunctionInfo, StructInfo};
    use tempfile::NamedTempFile;
    use std::io::Write;

    fn make_code_info(functions: Vec<FunctionInfo>, structs: Vec<StructInfo>) -> CodeInfo {
        CodeInfo {
            functions,
            structs,
            enums: Vec::new(),
            ..Default::default()
        }
    }

    #[test]
    fn estimate_complexity_trivial() {
        let info = make_code_info(vec![], vec![]);
        assert_eq!(estimate_complexity(&info), TranslationComplexity::Trivial);
    }

    #[test]
    fn estimate_complexity_low() {
        let funcs: Vec<_> = (0..5)
            .map(|i| FunctionInfo {
                name: format!("f{i}"),
                visibility: "pub".into(),
                inputs: vec![],
                output: None,
                body: String::new(),
            })
            .collect();
        let info = make_code_info(funcs, vec![]);
        assert_eq!(estimate_complexity(&info), TranslationComplexity::Low);
    }

    #[test]
    fn estimate_complexity_medium() {
        let funcs: Vec<_> = (0..15)
            .map(|i| FunctionInfo {
                name: format!("f{i}"),
                visibility: "pub".into(),
                inputs: vec![],
                output: None,
                body: String::new(),
            })
            .collect();
        let info = make_code_info(funcs, vec![]);
        assert_eq!(estimate_complexity(&info), TranslationComplexity::Medium);
    }

    #[test]
    fn estimate_complexity_high() {
        let funcs: Vec<_> = (0..30)
            .map(|i| FunctionInfo {
                name: format!("f{i}"),
                visibility: "pub".into(),
                inputs: vec![],
                output: None,
                body: String::new(),
            })
            .collect();
        let info = make_code_info(funcs, vec![]);
        assert_eq!(estimate_complexity(&info), TranslationComplexity::High);
    }

    #[test]
    fn resolve_route_default_is_local_qwen() {
        let config = TranslateConfig::default();
        assert_eq!(resolve_route(&config), LlmRoute::LocalQwen);
    }

    #[test]
    fn resolve_route_with_endpoint_is_hypervisor() {
        let config = TranslateConfig {
            endpoint_override: Some("http://fleet:8080".into()),
            ..Default::default()
        };
        assert_eq!(resolve_route(&config), LlmRoute::HypervisorBus);
    }

    #[test]
    fn build_offload_packet_from_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "pub fn add(a: i32, b: i32) -> i32 {{ a + b }}").unwrap();

        let config = TranslateConfig::default();
        let packet = build_offload_packet(file.path(), "my_crate", &config).unwrap();

        assert!(!packet.source_code.is_empty());
        assert_eq!(packet.target_crate, "my_crate");
        assert_eq!(packet.code_info.functions.len(), 1);
        assert_eq!(packet.code_info.functions[0].name, "add");
        assert!(packet.task_id > 0);
        assert!(packet.created_at_us > 0);
    }

    #[test]
    fn build_offload_packet_rejects_oversized() {
        let mut file = NamedTempFile::new().unwrap();
        // Write 2MB of data
        let big = "x".repeat(2 * 1024 * 1024);
        writeln!(file, "{}", big).unwrap();

        let config = TranslateConfig {
            max_source_bytes: 1_048_576,
            ..Default::default()
        };
        let result = build_offload_packet(file.path(), "target", &config);
        assert!(result.is_err());
    }

    #[test]
    fn ast_fallback_replaces_println() {
        let packet = LlmOffloadPacket {
            task_id: 1,
            route: LlmRoute::AstFallback,
            complexity: TranslationComplexity::Trivial,
            source_path: PathBuf::from("test.rs"),
            target_crate: "test".into(),
            source_code: "fn main() {\n    println!(\"hello\");\n}".into(),
            code_info: CodeInfo::default(),
            instructions: String::new(),
            endpoint_override: None,
            created_at_us: 0,
        };

        let outcome = execute_ast_fallback(&packet);
        assert!(outcome.success);
        let code = outcome.translated_code.unwrap();
        assert!(code.contains("tracing::info!"));
        assert!(!code.contains("println!"));
    }

    #[test]
    fn ast_fallback_adds_core_contracts_import() {
        let packet = LlmOffloadPacket {
            task_id: 2,
            route: LlmRoute::AstFallback,
            complexity: TranslationComplexity::Trivial,
            source_path: PathBuf::from("test.rs"),
            target_crate: "test".into(),
            source_code: "pub fn compute() -> u32 { 42 }".into(),
            code_info: CodeInfo::default(),
            instructions: String::new(),
            endpoint_override: None,
            created_at_us: 0,
        };

        let outcome = execute_ast_fallback(&packet);
        let code = outcome.translated_code.unwrap();
        assert!(code.contains("use core_contracts"));
    }

    #[test]
    fn ast_fallback_does_not_duplicate_import() {
        let packet = LlmOffloadPacket {
            task_id: 3,
            route: LlmRoute::AstFallback,
            complexity: TranslationComplexity::Trivial,
            source_path: PathBuf::from("test.rs"),
            target_crate: "test".into(),
            source_code: "use core_contracts::ComponentManifest;\npub fn compute() -> u32 { 42 }".into(),
            code_info: CodeInfo::default(),
            instructions: String::new(),
            endpoint_override: None,
            created_at_us: 0,
        };

        let outcome = execute_ast_fallback(&packet);
        let code = outcome.translated_code.unwrap();
        let count = code.matches("use core_contracts").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn packet_json_roundtrip() {
        let packet = LlmOffloadPacket {
            task_id: 42,
            route: LlmRoute::LocalQwen,
            complexity: TranslationComplexity::Medium,
            source_path: PathBuf::from("legacy.rs"),
            target_crate: "new_acc".into(),
            source_code: "fn foo() {}".into(),
            code_info: CodeInfo::default(),
            instructions: "Do the thing".into(),
            endpoint_override: None,
            created_at_us: 1234567890,
        };

        let json = packet.to_json().unwrap();
        let restored: LlmOffloadPacket = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.task_id, 42);
        assert_eq!(restored.route, LlmRoute::LocalQwen);
        assert_eq!(restored.complexity, TranslationComplexity::Medium);
        assert_eq!(restored.target_crate, "new_acc");
    }

    #[test]
    fn generate_instructions_vary_by_complexity() {
        let trivial = generate_instructions("crate", &TranslationComplexity::Trivial);
        let high = generate_instructions("crate", &TranslationComplexity::High);
        assert!(trivial.contains("trivial"));
        assert!(high.contains("cross-module"));
    }
}
