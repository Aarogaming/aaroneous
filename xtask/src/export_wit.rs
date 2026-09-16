//! Module for exporting WebAssembly Interface Type (WIT) declarations
//! from component traits defined in the capabilities crate.

#![allow(dead_code, unused_variables)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Configuration for WIT export
#[derive(Debug, Clone)]
pub struct WitExportConfig {
    /// Path to the capabilities crate source
    pub capabilities_path: PathBuf,
    /// Output directory for WIT files
    pub output_dir: PathBuf,
    /// Whether to include documentation comments
    pub include_docs: bool,
    /// Whether to generate verbose output
    pub verbose: bool,
}

impl Default for WitExportConfig {
    fn default() -> Self {
        Self {
            capabilities_path: PathBuf::from("crates/capabilities"),
            output_dir: PathBuf::from("wit"),
            include_docs: true,
            verbose: true,
        }
    }
}

/// Represents a parsed Rust trait
#[derive(Debug, Clone)]
pub struct RustTrait {
    pub name: String,
    pub visibility: Visibility,
    pub methods: Vec<Method>,
    pub associated_types: Vec<AssociatedType>,
}

/// Represents a method in a trait
#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub visibility: Visibility,
    pub arguments: Vec<Argument>,
    pub return_type: Option<ReturnType>,
    pub async_: bool,
}

/// Represents an argument in a method
#[derive(Debug, Clone)]
pub struct Argument {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
}

/// Represents a return type
#[derive(Debug, Clone)]
pub struct ReturnType {
    pub ty: Type,
}

/// Represents a type in the trait
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Primitive(PrimitiveType),
    Custom(String),
    Vec(String),
    Option(String),
    Result(String, String),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Primitive(p) => write!(f, "{:?}", p),
            Type::Custom(s) => write!(f, "{}", s),
            Type::Vec(s) => write!(f, "list<{}>", s),
            Type::Option(s) => write!(f, "option<{}>", s),
            Type::Result(ok, err) => write!(f, "result<{}, {}>", ok, err),
        }
    }
}

/// Primitive type variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Char,
    Str,
}

/// Represents an associated type in a trait
#[derive(Debug, Clone)]
pub struct AssociatedType {
    pub name: String,
    pub ty: Type,
}

/// Visibility modifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Crate,
}

impl From<&str> for Visibility {
    fn from(v: &str) -> Self {
        match v {
            "pub" | "pub(crate)" => Visibility::Public,
            _ => Visibility::Private,
        }
    }
}

impl From<Visibility> for String {
    fn from(v: Visibility) -> Self {
        match v {
            Visibility::Public => "pub".to_string(),
            Visibility::Private => "".to_string(),
            Visibility::Crate => "pub(crate)".to_string(),
        }
    }
}

/// Parse Rust source files and extract trait definitions
pub fn parse_rust_files(config: &WitExportConfig) -> Result<Vec<RustTrait>, String> {
    if !config.capabilities_path.exists() {
        return Err(format!(
            "Capabilities path does not exist: {:?}",
            config.capabilities_path
        ));
    }

    let mut traits = Vec::new();
    let rs_files = find_rust_files(&config.capabilities_path)?;

    for file_path in rs_files {
        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read file {:?}: {}", file_path, e))?;

        let traits_in_file = parse_trait_definitions(&content, &file_path);
        traits.extend(traits_in_file);
    }

    Ok(traits)
}

/// Find all .rs files recursively in a directory
fn find_rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_rust_files(&path)?);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }

    Ok(files)
}

/// Parse trait definitions from Rust source code
fn parse_trait_definitions(content: &str, _file_path: &Path) -> Vec<RustTrait> {
    let mut traits = Vec::new();

    let trait_pattern = match regex::Regex::new(r#"pub\s+trait\s+(\w+)"#) {
        Ok(re) => re,
        Err(_) => return traits,
    };

    for cap in trait_pattern.captures_iter(content) {
        let trait_name = cap.get(1).map_or("", |m| m.as_str()).to_string();
        let methods = parse_methods(content);
        let associated_types = parse_associated_types(content);

        traits.push(RustTrait {
            name: trait_name,
            visibility: Visibility::Public,
            methods,
            associated_types,
        });
    }

    traits
}

/// Parse method definitions from trait body
fn parse_methods(body: &str) -> Vec<Method> {
    let mut methods = Vec::new();

    let method_pattern =
        regex::Regex::new(r#"fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*(\w+))?\s*(?:async)?\s*\{"#)
            .expect("valid regex");

    for cap in method_pattern.captures_iter(body) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let args_str = cap.get(2).unwrap().as_str();
        let return_type = cap.get(3).map(|m| m.as_str().to_string());

        let arguments = parse_arguments(args_str);
        let async_ = cap.get(0).unwrap().as_str().contains("async");

        methods.push(Method {
            name,
            visibility: Visibility::Public,
            arguments,
            return_type: return_type.map(|t| ReturnType {
                ty: Type::Custom(t),
            }),
            async_,
        });
    }

    methods
}

/// Parse arguments from method signature
fn parse_arguments(args_str: &str) -> Vec<Argument> {
    let mut arguments = Vec::new();

    if args_str.trim().is_empty() {
        return arguments;
    }

    let mut current = String::new();
    let mut depth = 0;

    for ch in args_str.chars() {
        match ch {
            ',' if depth == 0 => {
                if !current.trim().is_empty() {
                    arguments.push(parse_argument(current.trim()));
                }
                current.clear();
            }
            '(' | '{' | '[' => depth += 1,
            ')' | '}' | ']' => depth -= 1,
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        arguments.push(parse_argument(current.trim()));
    }

    arguments
}

/// Parse a single argument
fn parse_argument(arg_str: &str) -> Argument {
    let trimmed = arg_str.trim();
    let name: String;
    let ty: String;
    let mut optional = false;

    if let Some(inner) = trimmed.strip_prefix("Option<") {
        optional = true;
        let last_bracket = inner.rfind('<').unwrap_or(0);
        ty = inner[last_bracket + 1..].trim().to_string();
        name = "arg".to_string();
    } else {
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            ty = parts[parts.len() - 1].to_string();
            name = parts[0].to_string();
        } else {
            ty = trimmed.to_string();
            name = "arg".to_string();
        }
    }

    Argument {
        name,
        ty: Type::Custom(ty),
        optional,
    }
}

/// Parse associated types from trait body
fn parse_associated_types(body: &str) -> Vec<AssociatedType> {
    let mut types = Vec::new();

    let type_pattern = regex::Regex::new(r#"type\s+(\w+)\s*=\s*(\w+)"#).expect("valid regex");

    for cap in type_pattern.captures_iter(body) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let ty = cap.get(2).unwrap().as_str().to_string();

        types.push(AssociatedType {
            name,
            ty: Type::Custom(ty),
        });
    }

    types
}

/// Generate WIT declarations from parsed traits
pub fn generate_wit(traits: &[RustTrait], config: &WitExportConfig) -> Result<String, String> {
    let mut wit_content = String::new();

    wit_content.push_str(&format!(
        ";; WIT file generated from Rust traits\n;; Source: {}\n\n",
        config.capabilities_path.display()
    ));

    wit_content.push_str("package capabilities:component;\n\n");

    for trait_def in traits {
        let wit_name = format!("{}-trait", trait_def.name);
        wit_content.push_str(&format!("interface {} {{\n", wit_name));

        for method in &trait_def.methods {
            let visibility = if method.visibility == Visibility::Public {
                ""
            } else {
                "private "
            };

            let async_marker = if method.async_ { "async " } else { "" };

            let args: String = method
                .arguments
                .iter()
                .map(|arg| {
                    let opt = if arg.optional { "?" } else { "" };
                    format!("{}{} {}", arg.ty, opt, arg.name)
                })
                .collect::<Vec<_>>()
                .join(", ");

            let return_type = method
                .return_type
                .as_ref()
                .map(|rt| format!(" -> {}", rt.ty))
                .unwrap_or_default();

            wit_content.push_str(&format!(
                "  {}{}fn {}({}){}",
                visibility, async_marker, method.name, args, return_type
            ));
            wit_content.push('\n');
        }

        for assoc_type in &trait_def.associated_types {
            wit_content.push_str(&format!(
                "  type {} = {};\n",
                assoc_type.name, assoc_type.ty
            ));
        }

        wit_content.push_str("}\n\n");
    }

    Ok(wit_content)
}

/// Export WIT files to the output directory
pub fn export_wit(config: &WitExportConfig) -> Result<(), String> {
    if config.verbose {
        println!(
            "Exporting WIT declarations from {:?}",
            config.capabilities_path
        );
    }

    if let Err(e) = fs::create_dir_all(&config.output_dir) {
        return Err(format!("Failed to create output directory: {}", e));
    }

    let traits = parse_rust_files(config)?;

    if traits.is_empty() {
        if config.verbose {
            println!("No traits found to export");
        }
        return Ok(());
    }

    let wit_content = generate_wit(&traits, config)?;

    let wit_path = config.output_dir.join("capabilities.wit");
    if let Err(e) = fs::write(&wit_path, &wit_content) {
        return Err(format!("Failed to write WIT file: {}", e));
    }

    if config.verbose {
        println!("WIT file exported to: {:?}", wit_path);
    }

    Ok(())
}

/// Validate WIT syntax using wit-parser if available
pub fn validate_wit(wit_path: &Path) -> Result<(), String> {
    if !wit_path.exists() {
        return Err(format!("WIT file does not exist: {:?}", wit_path));
    }

    let output = Command::new("wit").arg("parse").arg(wit_path).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(())
            } else {
                Err(String::from_utf8_lossy(&out.stderr).to_string())
            }
        }
        Err(e) => {
            if config_verbose() {
                println!("Warning: wit command not available for validation: {}", e);
            }
            Ok(())
        }
    }
}

/// Check if verbose mode is enabled
fn config_verbose() -> bool {
    std::env::var("WIT_EXPORT_VERBOSE").is_ok_and(|v| v == "1")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_conversion() {
        assert_eq!(Visibility::from("pub"), Visibility::Public);
        assert_eq!(Visibility::from("pub(crate)"), Visibility::Public);
        assert_eq!(Visibility::from("private"), Visibility::Private);
    }

    #[test]
    fn test_type_conversion() {
        let ty = Type::Custom("String".to_string());
        assert_eq!(ty, Type::Custom("String".to_string()));
    }
}
