//! ast_parser.rs
//! AST parsing, syntax inspection, and symbol extraction.

use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    pub line_count: usize,
    pub functions: Vec<FunctionSignature>,
    pub structs: Vec<String>,
    pub syntax_errors: Vec<String>,
    pub complexity_score: f32,
}

/// AST Parser engine
pub struct AstParser;

impl AstParser {
    /// Ingests source code and extracts structural observations
    pub fn parse_source(file_path: &str, source_code: &str) -> Result<AstObservation> {
        let lines: Vec<&str> = source_code.lines().collect();
        let line_count = lines.len();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let syntax_errors = Vec::new();

        // Basic AST heuristic parsing (and Tree-Sitter when enabled)
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") || trimmed.starts_with("async fn ") || trimmed.starts_with("pub async fn ") {
                let is_async = trimmed.contains("async ");
                let vis = if trimmed.starts_with("pub ") { "public" } else { "private" };
                
                // Extract function name
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
                let struct_name = trimmed.split_whitespace().nth(if trimmed.starts_with("pub ") { 2 } else { 1 }).unwrap_or("unknown").trim_end_matches('{').trim().to_string();
                structs.push(struct_name);
            }
        }

        let complexity = (functions.len() as f32 * 1.5) + (structs.len() as f32 * 2.0);

        Ok(AstObservation {
            file_path: file_path.to_string(),
            line_count,
            functions,
            structs,
            syntax_errors,
            complexity_score: complexity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_parser_extraction() {
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
        assert_eq!(obs.line_count, 11);
        assert_eq!(obs.structs.len(), 1);
        assert_eq!(obs.structs[0], "EngineConfig");
        assert_eq!(obs.functions.len(), 2);
        assert_eq!(obs.functions[0].name, "start_server");
        assert!(obs.functions[0].is_async);
        assert_eq!(obs.functions[0].parameter_count, 2);
    }
}
