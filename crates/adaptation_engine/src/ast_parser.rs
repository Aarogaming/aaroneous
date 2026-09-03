//! ast_parser.rs
//! Polyglot AST parsing, syntax inspection, and incremental structural diffing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Supported Polyglot Source Code Languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceLanguage {
    Rust,
    Python,
    TypeScript,
    Cpp,
    Unknown,
}

impl SourceLanguage {
    /// Detects language from file path extension
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let p = path.as_ref();
        match p.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => SourceLanguage::Rust,
            Some("py") | Some("pyi") => SourceLanguage::Python,
            Some("ts") | Some("tsx") | Some("js") | Some("jsx") => SourceLanguage::TypeScript,
            Some("c") | Some("cpp") | Some("cc") | Some("h") | Some("hpp") => SourceLanguage::Cpp,
            _ => SourceLanguage::Unknown,
        }
    }
}

/// Function signature extracted from source code AST
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionSignature {
    pub name: String,
    pub visibility: String,
    pub is_async: bool,
    pub line_number: usize,
    pub parameter_count: usize,
    pub return_type: Option<String>,
}

/// Structural observations extracted from an analyzed source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstObservation {
    pub file_path: String,
    pub language: SourceLanguage,
    pub line_count: usize,
    pub functions: Vec<FunctionSignature>,
    pub structs: Vec<String>,
    pub syntax_errors: Vec<String>,
    pub complexity_score: f32,
}

/// Incremental AST Difference Result between two code revisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AstDiffResult {
    pub file_path: String,
    pub added_functions: Vec<FunctionSignature>,
    pub removed_functions: Vec<FunctionSignature>,
    pub modified_functions: Vec<(FunctionSignature, FunctionSignature)>,
    pub added_structs: Vec<String>,
    pub removed_structs: Vec<String>,
    pub complexity_delta: f32,
}

/// Polyglot AST Parser engine
pub struct AstParser;

impl AstParser {
    /// Ingests source code and extracts structural observations across polyglot languages
    pub fn parse_source(file_path: &str, source_code: &str) -> Result<AstObservation> {
        let language = SourceLanguage::from_path(file_path);
        let lines: Vec<&str> = source_code.lines().collect();
        let line_count = lines.len();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let syntax_errors = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            match language {
                SourceLanguage::Rust => {
                    if trimmed.starts_with("pub fn ")
                        || trimmed.starts_with("fn ")
                        || trimmed.starts_with("async fn ")
                        || trimmed.starts_with("pub async fn ")
                    {
                        let is_async = trimmed.contains("async ");
                        let vis = if trimmed.starts_with("pub ") { "public" } else { "private" };
                        let fn_part = if let Some(idx) = trimmed.find("fn ") {
                            &trimmed[idx + 3..]
                        } else {
                            trimmed
                        };
                        let fn_name = fn_part.split('(').next().unwrap_or("unknown").trim().to_string();
                        let param_count = if let Some(params) = fn_part.split('(').nth(1) {
                            params.split(')').next().unwrap_or("").split(',').filter(|p| !p.trim().is_empty()).count()
                        } else {
                            0
                        };

                        functions.push(FunctionSignature {
                            name: fn_name,
                            visibility: vis.to_string(),
                            is_async,
                            line_number: i + 1,
                            parameter_count: param_count,
                            return_type: None,
                        });
                    } else if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                        let struct_name = trimmed
                            .split_whitespace()
                            .nth(if trimmed.starts_with("pub ") { 2 } else { 1 })
                            .unwrap_or("unknown")
                            .trim_end_matches('{')
                            .trim()
                            .to_string();
                        structs.push(struct_name);
                    }
                }
                SourceLanguage::Python => {
                    if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                        let is_async = trimmed.starts_with("async def ");
                        let fn_part = if is_async { &trimmed[10..] } else { &trimmed[4..] };
                        let fn_name = fn_part.split('(').next().unwrap_or("unknown").trim().to_string();
                        let vis = if fn_name.starts_with('_') { "private" } else { "public" };
                        let param_count = if let Some(params) = fn_part.split('(').nth(1) {
                            params.split(')').next().unwrap_or("").split(',').filter(|p| !p.trim().is_empty() && p.trim() != "self" && p.trim() != "cls").count()
                        } else {
                            0
                        };

                        functions.push(FunctionSignature {
                            name: fn_name,
                            visibility: vis.to_string(),
                            is_async,
                            line_number: i + 1,
                            parameter_count: param_count,
                            return_type: None,
                        });
                    } else if let Some(rest) = trimmed.strip_prefix("class ") {
                        let class_name = rest.split('(').next().unwrap_or("").split(':').next().unwrap_or("unknown").trim().to_string();
                        structs.push(class_name);
                    }
                }
                SourceLanguage::TypeScript => {
                    if trimmed.starts_with("function ")
                        || trimmed.starts_with("export function ")
                        || trimmed.starts_with("async function ")
                        || trimmed.starts_with("export async function ")
                    {
                        let is_async = trimmed.contains("async ");
                        let vis = if trimmed.starts_with("export ") { "public" } else { "private" };
                        let fn_part = if let Some(idx) = trimmed.find("function ") {
                            &trimmed[idx + 9..]
                        } else {
                            trimmed
                        };
                        let fn_name = fn_part.split('(').next().unwrap_or("unknown").trim().to_string();
                        let param_count = if let Some(params) = fn_part.split('(').nth(1) {
                            params.split(')').next().unwrap_or("").split(',').filter(|p| !p.trim().is_empty()).count()
                        } else {
                            0
                        };

                        functions.push(FunctionSignature {
                            name: fn_name,
                            visibility: vis.to_string(),
                            is_async,
                            line_number: i + 1,
                            parameter_count: param_count,
                            return_type: None,
                        });
                    } else if trimmed.starts_with("export class ") || trimmed.starts_with("class ") || trimmed.starts_with("export interface ") || trimmed.starts_with("interface ") {
                        let name = trimmed.split_whitespace().nth(if trimmed.starts_with("export ") { 2 } else { 1 }).unwrap_or("unknown").trim_end_matches('{').trim().to_string();
                        structs.push(name);
                    }
                }
                SourceLanguage::Cpp | SourceLanguage::Unknown => {
                    if (trimmed.starts_with("void ") || trimmed.starts_with("int ") || trimmed.starts_with("double ") || trimmed.starts_with("bool ") || trimmed.starts_with("extern \"C\"")) && trimmed.contains('(') {
                        let fn_name = trimmed.split('(').next().unwrap_or("unknown").split_whitespace().last().unwrap_or("unknown").to_string();
                        let param_count = if let Some(params) = trimmed.split('(').nth(1) {
                            params.split(')').next().unwrap_or("").split(',').filter(|p| !p.trim().is_empty() && p.trim() != "void").count()
                        } else {
                            0
                        };

                        functions.push(FunctionSignature {
                            name: fn_name,
                            visibility: "public".to_string(),
                            is_async: false,
                            line_number: i + 1,
                            parameter_count: param_count,
                            return_type: None,
                        });
                    } else if trimmed.starts_with("struct ") || trimmed.starts_with("class ") {
                        let name = trimmed.split_whitespace().nth(1).unwrap_or("unknown").trim_end_matches('{').trim().to_string();
                        structs.push(name);
                    }
                }
            }
        }

        let complexity = (functions.len() as f32 * 1.5) + (structs.len() as f32 * 2.0);

        Ok(AstObservation {
            file_path: file_path.to_string(),
            language,
            line_count,
            functions,
            structs,
            syntax_errors,
            complexity_score: complexity,
        })
    }

    /// Computes incremental AST differences between two versions of a source file
    pub fn compute_ast_diff(old_obs: &AstObservation, new_obs: &AstObservation) -> AstDiffResult {
        let mut added_functions = Vec::new();
        let mut removed_functions = Vec::new();
        let mut modified_functions = Vec::new();

        for new_fn in &new_obs.functions {
            if let Some(old_fn) = old_obs.functions.iter().find(|f| f.name == new_fn.name) {
                if old_fn != new_fn {
                    modified_functions.push((old_fn.clone(), new_fn.clone()));
                }
            } else {
                added_functions.push(new_fn.clone());
            }
        }

        for old_fn in &old_obs.functions {
            if !new_obs.functions.iter().any(|f| f.name == old_fn.name) {
                removed_functions.push(old_fn.clone());
            }
        }

        let added_structs = new_obs
            .structs
            .iter()
            .filter(|s| !old_obs.structs.contains(s))
            .cloned()
            .collect();

        let removed_structs = old_obs
            .structs
            .iter()
            .filter(|s| !new_obs.structs.contains(s))
            .cloned()
            .collect();

        AstDiffResult {
            file_path: new_obs.file_path.clone(),
            added_functions,
            removed_functions,
            modified_functions,
            added_structs,
            removed_structs,
            complexity_delta: new_obs.complexity_score - old_obs.complexity_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_ast_parsing() {
        let code = r#"
pub struct EngineConfig {
    pub port: u16,
}

pub async fn start_server(port: u16, host: String) -> bool {
    true
}

fn internal_helper() {
}
"#;

        let obs = AstParser::parse_source("src/server.rs", code).unwrap();
        assert_eq!(obs.language, SourceLanguage::Rust);
        assert_eq!(obs.structs.len(), 1);
        assert_eq!(obs.structs[0], "EngineConfig");
        assert_eq!(obs.functions.len(), 2);
        assert_eq!(obs.functions[0].name, "start_server");
        assert!(obs.functions[0].is_async);
        assert_eq!(obs.functions[0].parameter_count, 2);
    }

    #[test]
    fn test_python_ast_parsing() {
        let py_code = r#"
class DataProcessor:
    def __init__(self, name):
        pass

async def fetch_records(query, limit):
    pass
"#;

        let obs = AstParser::parse_source("processor.py", py_code).unwrap();
        assert_eq!(obs.language, SourceLanguage::Python);
        assert_eq!(obs.structs.len(), 1);
        assert_eq!(obs.structs[0], "DataProcessor");
        assert_eq!(obs.functions.len(), 2);
        assert_eq!(obs.functions[0].name, "__init__");
        assert_eq!(obs.functions[1].name, "fetch_records");
        assert!(obs.functions[1].is_async);
        assert_eq!(obs.functions[1].parameter_count, 2);
    }

    #[test]
    fn test_typescript_ast_parsing() {
        let ts_code = r#"
export interface UserPayload {
    id: string;
}

export async function authenticateUser(token: string, retries: number) {
    return true;
}
"#;

        let obs = AstParser::parse_source("auth.ts", ts_code).unwrap();
        assert_eq!(obs.language, SourceLanguage::TypeScript);
        assert_eq!(obs.structs.len(), 1);
        assert_eq!(obs.structs[0], "UserPayload");
        assert_eq!(obs.functions.len(), 1);
        assert_eq!(obs.functions[0].name, "authenticateUser");
        assert!(obs.functions[0].is_async);
        assert_eq!(obs.functions[0].parameter_count, 2);
    }

    #[test]
    fn test_ast_diffing() {
        let v1_code = r#"
fn calculate_sum(a: i32, b: i32) -> i32 { a + b }
fn remove_me() {}
"#;
        let v2_code = r#"
fn calculate_sum(a: i32, b: i32, c: i32) -> i32 { a + b + c }
fn new_feature() {}
"#;

        let obs1 = AstParser::parse_source("lib.rs", v1_code).unwrap();
        let obs2 = AstParser::parse_source("lib.rs", v2_code).unwrap();

        let diff = AstParser::compute_ast_diff(&obs1, &obs2);
        assert_eq!(diff.added_functions.len(), 1);
        assert_eq!(diff.added_functions[0].name, "new_feature");
        assert_eq!(diff.removed_functions.len(), 1);
        assert_eq!(diff.removed_functions[0].name, "remove_me");
        assert_eq!(diff.modified_functions.len(), 1);
        assert_eq!(diff.modified_functions[0].0.name, "calculate_sum");
    }
}

