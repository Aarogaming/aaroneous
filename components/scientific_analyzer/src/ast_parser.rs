// OBSERVE Phase: AST Parsing and Code Structure Analysis
// Parses Rust/Python code into dense structural representations

use std::path::Path;
use std::fs;
use serde::{Serialize, Deserialize};
use regex::Regex;

/// Observation of a code unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstObservation {
    pub file_path: String,
    pub language: Language,
    pub structures: Vec<CodeStructure>,
    pub complexity_metrics: ComplexityMetrics,
    pub raw_entropy: f64,
}

/// Programming language
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Language {
    Rust,
    Python,
    Unknown,
}

/// Code structure (function, struct, module, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeStructure {
    pub name: String,
    pub structure_type: StructureType,
    pub signature: FunctionSignature,
    pub line_range: (usize, usize),
    pub dependencies: Vec<String>,
    pub control_flow_complexity: u32,
}

/// Type of code structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StructureType {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Class,
    Method,
}

/// Function signature representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<String>,
    pub is_async: bool,
    pub visibility: Visibility,
}

/// Parameter in a function signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
}

/// Visibility level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Visibility {
    Public,
    Private,
    Restricted(String),
}

/// Complexity metrics for a code unit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: u32,
    pub lines_of_code: u32,
    pub nesting_depth: u32,
    pub branch_count: u32,
    pub call_count: u32,
}

impl AstObservation {
    /// Parse a Rust file into structural observations
    pub fn parse_rust_file(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let structures = parse_rust_ast(&content);
        let complexity = calculate_complexity(&content, &structures);
        let raw_entropy = compute_entropy(&content);
        
        Ok(Self {
            file_path: path.to_string_lossy().to_string(),
            language: Language::Rust,
            structures,
            complexity_metrics: complexity,
            raw_entropy,
        })
    }

    /// Parse a Python file into structural observations
    pub fn parse_python_file(path: &Path) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path)?;
        let structures = parse_python_ast(&content);
        let complexity = calculate_complexity(&content, &structures);
        let raw_entropy = compute_entropy(&content);
        
        Ok(Self {
            file_path: path.to_string_lossy().to_string(),
            language: Language::Python,
            structures,
            complexity_metrics: complexity,
            raw_entropy,
        })
    }
}

/// Parse Rust code using regex-based AST extraction
fn parse_rust_ast(content: &str) -> Vec<CodeStructure> {
    let mut structures = Vec::new();
    
    // Function regex
    let func_re = Regex::new(r#"(?m)^\s*(pub\s+)?(async\s+)?fn\s+(\w+)\s*\(([^)]*)\)\s*(->\s*([^{]+))?\s*\{"#).unwrap();
    
    for cap in func_re.captures_iter(content) {
        let visibility = if cap.get(1).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        
        let is_async = cap.get(2).is_some();
        let name = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
        let params_str = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();
        let return_type = cap.get(6).map(|m| m.as_str().trim().to_string());
        
        let parameters = parse_parameters(&params_str);
        
        // Find line range
        let start_line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        let end_line = find_closing_brace(content, cap.get(0).unwrap().end()).unwrap_or(start_line + 10);
        
        structures.push(CodeStructure {
            name: name.clone(),
            structure_type: StructureType::Function,
            signature: FunctionSignature {
                name,
                parameters,
                return_type,
                is_async,
                visibility,
            },
            line_range: (start_line, end_line),
            dependencies: Vec::new(), // Would extract from use statements
            control_flow_complexity: 1, // Base complexity
        });
    }
    
    structures
}

/// Parse Python code using regex-based AST extraction
fn parse_python_ast(content: &str) -> Vec<CodeStructure> {
    let mut structures = Vec::new();
    
    // Function regex
    let func_re = Regex::new(r#"(?m)^\s*(async\s+)?def\s+(\w+)\s*\(([^)]*)\)\s*(->\s*([^\n:]+))?\s*:"#).unwrap();
    
    for cap in func_re.captures_iter(content) {
        let is_async = cap.get(1).is_some();
        let name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        let params_str = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
        let return_type = cap.get(5).map(|m| m.as_str().trim().to_string());
        
        let parameters = parse_parameters(&params_str);
        
        let start_line = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        let end_line = find_python_block_end(content, cap.get(0).unwrap().end()).unwrap_or(start_line + 10);
        
        structures.push(CodeStructure {
            name: name.clone(),
            structure_type: StructureType::Function,
            signature: FunctionSignature {
                name,
                parameters,
                return_type,
                is_async,
                visibility: Visibility::Public, // Python defaults to public
            },
            line_range: (start_line, end_line),
            dependencies: Vec::new(),
            control_flow_complexity: 1,
        });
    }
    
    structures
}

/// Parse parameter string into Parameter structs
fn parse_parameters(params_str: &str) -> Vec<Parameter> {
    if params_str.trim().is_empty() || params_str.trim() == "self" || params_str.trim() == "&self" || params_str.trim() == "&mut self" {
        return Vec::new();
    }
    
    params_str
        .split(',')
        .filter_map(|p| {
            let p = p.trim();
            if p.is_empty() || p == "self" || p.starts_with('&') && p.contains("self") {
                return None;
            }
            
            let parts: Vec<&str> = p.split(':').collect();
            if parts.len() >= 2 {
                Some(Parameter {
                    name: parts[0].trim().to_string(),
                    param_type: parts[1].trim().to_string(),
                })
            } else {
                Some(Parameter {
                    name: p.to_string(),
                    param_type: "unknown".to_string(),
                })
            }
        })
        .collect()
}

/// Find closing brace for Rust function
fn find_closing_brace(content: &str, start: usize) -> Option<usize> {
    let mut depth = 1;
    let _pos = start;
    let bytes = &content.as_bytes()[start..];
    
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(content[..start + i].matches('\n').count() + 1);
                }
            }
            _ => {}
        }
    }
    
    None
}

/// Find end of Python block by indentation
fn find_python_block_end(content: &str, start: usize) -> Option<usize> {
    let lines: Vec<&str> = content[start..].lines().collect();
    let mut end_line = start;
    
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue; // Skip the def line
        }
        if line.trim().is_empty() || line.starts_with("def ") || line.starts_with("class ") {
            end_line = start + i;
            break;
        }
        end_line = start + i;
    }
    
    Some(content[..end_line].matches('\n').count() + 1)
}

/// Calculate complexity metrics
fn calculate_complexity(content: &str, _structures: &[CodeStructure]) -> ComplexityMetrics {
    let lines = content.lines().count() as u32;
    
    // Cyclomatic complexity approximation
    let branches = content.matches("if ").count() 
        + content.matches("else ").count() 
        + content.matches("match ").count()
        + content.matches("for ").count()
        + content.matches("while ").count()
        + content.matches("elif ").count();
    
    let calls = content.matches("::").count() + content.matches(".").count();
    
    ComplexityMetrics {
        cyclomatic_complexity: (1 + branches) as u32,
        lines_of_code: lines,
        nesting_depth: 2, // Simplified
        branch_count: branches as u32,
        call_count: calls as u32,
    }
}

/// Compute Shannon entropy of code content
fn compute_entropy(content: &str) -> f64 {
    let mut freq = std::collections::HashMap::new();
    let total = content.len() as f64;
    
    for byte in content.bytes() {
        *freq.entry(byte).or_insert(0) += 1;
    }
    
    let mut entropy = 0.0;
    for &count in freq.values() {
        let p = count as f64 / total;
        if p > 0.0 {
            entropy -= p * p.log2();
        }
    }
    
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_rust_function() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "pub fn test_function(x: i32, y: String) -> Result<(), Error> {{").unwrap();
        writeln!(file, "    // body").unwrap();
        writeln!(file, "}}").unwrap();
        
        let observation = AstObservation::parse_rust_file(file.path()).unwrap();
        assert_eq!(observation.structures.len(), 1);
        assert_eq!(observation.structures[0].name, "test_function");
        assert_eq!(observation.structures[0].signature.parameters.len(), 2);
    }

    #[test]
    fn test_parse_python_function() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "def test_function(x: int, y: str) -> bool:").unwrap();
        writeln!(file, "    # body").unwrap();
        writeln!(file, "    return True").unwrap();
        
        let observation = AstObservation::parse_python_file(file.path()).unwrap();
        assert_eq!(observation.structures.len(), 1);
        assert_eq!(observation.structures[0].name, "test_function");
    }

    #[test]
    fn test_complexity_calculation() {
        let content = "fn test() {\n    if x > 0 {\n        for i in 0..10 {\n            // code\n        }\n    }\n}";
        let complexity = calculate_complexity(content, &[]);
        assert!(complexity.cyclomatic_complexity >= 3);
        assert!(complexity.lines_of_code > 0);
    }

    #[test]
    fn test_entropy_calculation() {
        let content = "fn test() { let x = 1; }";
        let entropy = compute_entropy(content);
        assert!(entropy > 0.0 && entropy < 8.0);
    }
}
