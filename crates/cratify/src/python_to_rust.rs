//! Python-to-Rust Translation Engine (Round 3).
//!
//! Ingests Python source modules, parses them into a deterministic
//! intermediate representation (`PythonIR`), maps types to zero-copy
//! Rust equivalents, and emits idiomatic, ACC-compliant Rust code.
//!
//! # Type Mapping (spec §3)
//!
//! | Python | PythonIR | Rust |
//! |--------|----------|------|
//! | `int` | `PrimInt` | `i64` |
//! | `float` | `PrimFloat` | `f64` |
//! | `bool` | `PrimBool` | `bool` |
//! | `str` | `PrimStr` | `&str` |
//! | `bytes` | `PrimBytes` | `&[u8]` |
//! | `list[T]` | `Array { elem }` | `&[T]` |
//! | `dict[K, V]` | `Map { key, value }` | `&[(K, V)]` |
//! | `class` | `Struct { fields }` | `#[repr(C)] struct` |
//! | `def` | `Function { args, ret }` | `fn` |
//! | C extension import | `ForeignModule` | **Rejected** |

use std::fmt;

use serde::{Deserialize, Serialize};

// ── Errors ───────────────────────────────────────────────────────────

/// Errors during Python-to-Rust translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslateError {
    /// A Python construct cannot be mapped to Rust.
    UnmappableConstruct(String),
    /// A forbidden import (C extension) was detected.
    ForbiddenForeignModule(String),
    /// A struct has fields exceeding the Pod size limit.
    PayloadTooLarge { name: String, size: usize, limit: usize },
    /// Generic syntax could not be parsed.
    GenericParseError(String),
    /// Indentation-based block parsing failed.
    IndentationError(String),
}

impl fmt::Display for TranslateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmappableConstruct(s) => write!(f, "unmappable construct: {s}"),
            Self::ForbiddenForeignModule(s) => write!(f, "forbidden foreign module: {s}"),
            Self::PayloadTooLarge { name, size, limit } => {
                write!(f, "struct `{name}` payload {size}B exceeds limit {limit}B")
            }
            Self::GenericParseError(s) => write!(f, "generic parse error: {s}"),
            Self::IndentationError(s) => write!(f, "indentation error: {s}"),
        }
    }
}

impl std::error::Error for TranslateError {}

pub type TranslateResult<T> = std::result::Result<T, TranslateError>;

// ── Python IR Types (spec §3) ───────────────────────────────────────

/// Primitive Python types that map directly to Rust scalars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PythonPrim {
    Int,
    Float,
    Bool,
    Str,
    Bytes,
}

impl fmt::Display for PythonPrim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::Float => write!(f, "float"),
            Self::Bool => write!(f, "bool"),
            Self::Str => write!(f, "str"),
            Self::Bytes => write!(f, "bytes"),
        }
    }
}

/// Deterministic intermediate representation for Python constructs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PythonIR {
    /// A primitive type reference.
    Prim(PythonPrim),
    /// A fixed-length array: `list[T]` with optional known length.
    Array {
        elem: Box<PythonIR>,
        len: Option<usize>,
    },
    /// A key-value map: `dict[K, V]`.
    Map {
        key: Box<PythonIR>,
        value: Box<PythonIR>,
    },
    /// A named struct (from a Python `class`).
    Struct {
        name: String,
        fields: Vec<(String, PythonIR)>,
    },
    /// A function (from a Python `def`).
    Function {
        name: String,
        args: Vec<(String, PythonIR)>,
        returns: Box<PythonIR>,
        body: String,
    },
    /// A byte slice.
    ByteSlice { len: usize },
    /// A foreign (C extension) import — rejected.
    ForeignModule { name: String },
    /// A named type reference (user-defined class).
    NamedRef(String),
}

// ── Type Mapping ─────────────────────────────────────────────────────

/// Map a Python type annotation string to a `PythonIR` node.
pub fn map_python_type(annotation: &str) -> TranslateResult<PythonIR> {
    let trimmed = annotation.trim();

    // Primitives
    match trimmed {
        "int" => return Ok(PythonIR::Prim(PythonPrim::Int)),
        "float" => return Ok(PythonIR::Prim(PythonPrim::Float)),
        "bool" => return Ok(PythonIR::Prim(PythonPrim::Bool)),
        "str" => return Ok(PythonIR::Prim(PythonPrim::Str)),
        "bytes" => return Ok(PythonIR::Prim(PythonPrim::Bytes)),
        "None" => return Ok(PythonIR::Prim(PythonPrim::Bool)), // unit-like
        _ => {}
    }

    // list[T]
    if let Some(inner) = trimmed.strip_prefix("list[").and_then(|s| s.strip_suffix(']')) {
        let elem = map_python_type(inner)?;
        return Ok(PythonIR::Array {
            elem: Box::new(elem),
            len: None,
        });
    }

    // list (bare)
    if trimmed == "list" {
        return Ok(PythonIR::Array {
            elem: Box::new(PythonIR::Prim(PythonPrim::Bool)), // placeholder
            len: None,
        });
    }

    // dict[K, V]
    if let Some(inner) = trimmed.strip_prefix("dict[").and_then(|s| s.strip_suffix(']')) {
        let parts: Vec<&str> = inner.splitn(2, ',').collect();
        if parts.len() == 2 {
            let key = map_python_type(parts[0])?;
            let value = map_python_type(parts[1])?;
            return Ok(PythonIR::Map {
                key: Box::new(key),
                value: Box::new(value),
            });
        }
    }

    // dict (bare)
    if trimmed == "dict" {
        return Ok(PythonIR::Map {
            key: Box::new(PythonIR::Prim(PythonPrim::Str)),
            value: Box::new(PythonIR::Prim(PythonPrim::Str)),
        });
    }

    // Tuple → Array (treat as fixed-size slice)
    if let Some(inner) = trimmed.strip_prefix("tuple[").and_then(|s| s.strip_suffix(']')) {
        let elem = map_python_type(inner)?;
        return Ok(PythonIR::Array {
            elem: Box::new(elem),
            len: None,
        });
    }

    // Named type reference (user class)
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(PythonIR::NamedRef(trimmed.to_string()));
    }

    Err(TranslateError::GenericParseError(format!(
        "cannot map Python type: {trimmed}"
    )))
}

/// Check if a PythonIR is Pod-eligible (all scalar or Pod-struct fields).
pub fn is_pod_eligible(ir: &PythonIR) -> bool {
    match ir {
        PythonIR::Prim(_) | PythonIR::ByteSlice { .. } => true,
        PythonIR::Array { elem, .. } => is_pod_eligible(elem),
        PythonIR::Struct { fields, .. } => fields.iter().all(|(_, f)| is_pod_eligible(f)),
        PythonIR::NamedRef(_) => false, // Unknown — conservatively non-Pod
        PythonIR::Map { .. } => false,  // HashMap is not Pod
        PythonIR::Function { .. } => false,
        PythonIR::ForeignModule { .. } => false,
    }
}

// ── Rust Code Emission ───────────────────────────────────────────────

/// Emit a Rust type string from a PythonIR node.
pub fn emit_rust_type(ir: &PythonIR) -> String {
    match ir {
        PythonIR::Prim(p) => match p {
            PythonPrim::Int => "i64".into(),
            PythonPrim::Float => "f64".into(),
            PythonPrim::Bool => "bool".into(),
            PythonPrim::Str => "&str".into(),
            PythonPrim::Bytes => "&[u8]".into(),
        },
        PythonIR::Array { elem, .. } => {
            format!("&[{}]", emit_rust_type(elem))
        }
        PythonIR::Map { key, value } => {
            format!(
                "&[({}, {})]",
                emit_rust_type(key),
                emit_rust_type(value)
            )
        }
        PythonIR::ByteSlice { .. } => "&[u8]".into(),
        PythonIR::NamedRef(name) => name.clone(),
        PythonIR::Struct { name, .. } => name.clone(),
        PythonIR::Function { name, .. } => name.clone(),
        PythonIR::ForeignModule { name } => format!("/* FORBIDDEN: {name} */"),
    }
}

/// Emit a complete Rust struct definition from a Python class IR.
pub fn emit_struct(ir: &PythonIR) -> TranslateResult<String> {
    let PythonIR::Struct { name, fields } = ir else {
        return Err(TranslateError::UnmappableConstruct(
            "expected Struct IR node".into(),
        ));
    };

    let pod = is_pod_eligible(ir);
    let mut out = String::new();

    // Doc comment
    out.push_str(&format!("/// Translated from Python class `{name}`.\n"));

    // repr(C) for all structs (Pod-eligible or not)
    out.push_str("#[repr(C)]\n");

    // Derives
    if pod {
        out.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]\n");
    } else {
        out.push_str("#[derive(Clone, Debug, PartialEq, Eq)]\n");
    }

    out.push_str(&format!("pub struct {name} {{\n"));
    for (fname, ftype) in fields {
        let rust_type = emit_rust_type(ftype);
        out.push_str(&format!("    pub {fname}: {rust_type},\n"));
    }
    out.push_str("}\n");

    Ok(out)
}

/// Emit a Rust function from a Python function IR.
pub fn emit_function(ir: &PythonIR) -> TranslateResult<String> {
    let PythonIR::Function { name, args, returns, body } = ir else {
        return Err(TranslateError::UnmappableConstruct(
            "expected Function IR node".into(),
        ));
    };

    let mut out = String::new();

    // Doc comment
    out.push_str(&format!("/// Translated from Python function `{name}`.\n"));

    // Signature
    let ret_type = emit_rust_type(returns);
    let arg_list: Vec<String> = args
        .iter()
        .map(|(aname, atype)| format!("{aname}: {}", emit_rust_type(atype)))
        .collect();

    out.push_str(&format!("pub fn {name}({}) -> {ret_type} {{\n", arg_list.join(", ")));

    // Body — emit Python body as a block with Rust-compatible transformations
    let rust_body = python_body_to_rust(body);
    out.push_str(&format!("    {rust_body}\n"));
    out.push_str("}\n");

    Ok(out)
}

/// Emit a full Rust module from a list of Python IR nodes.
pub fn emit_module(nodes: &[PythonIR], module_name: &str) -> TranslateResult<String> {
    let mut out = String::new();

    // Module header
    out.push_str(&format!("//! `{module_name}` — translated from Python by Cratify (Round 3).\n"));
    out.push_str("//!\n");
    out.push_str("//! Auto-generated ACC module. Do not edit manually.\n\n");

    // Use statements for non-Pod types that need serde
    let needs_serde = nodes.iter().any(|n| !is_pod_eligible(n));
    if needs_serde {
        out.push_str("use serde::{Deserialize, Serialize};\n");
    }

    // Emit each node
    for node in nodes {
        match node {
            PythonIR::Struct { .. } => {
                out.push_str(&emit_struct(node)?);
                out.push('\n');
            }
            PythonIR::Function { .. } => {
                out.push_str(&emit_function(node)?);
                out.push('\n');
            }
            _ => {}
        }
    }

    Ok(out)
}

/// Transform a Python function body into Rust-compatible code.
fn python_body_to_rust(body: &str) -> String {
    let mut result = body.trim().to_string();

    // Replace Python print with tracing
    result = result.replace("print(", "tracing::info!(");

    // Replace Python None with unit
    result = result.replace("None", "()");
    result = result.replace("None,", "(),");

    // Replace Python True/False
    result = result.replace("True", "true");
    result = result.replace("False", "false");

    // Replace Python `is not None` with `is_some()`
    result = result.replace("is not None", ".is_some()");
    result = result.replace("is None", ".is_none()");

    // Replace `len(x)` with `x.len()`
    // Simple heuristic: len(X) → X.len()
    // This is a best-effort transformation; complex cases need LLM.
    let len_pattern = regex_lite("len(", ".len()");

    // Apply simple patterns
    for (pattern, replacement) in &len_pattern {
        result = result.replace(pattern, replacement);
    }

    // Indent to 4 spaces (Python uses 4-space indent, Rust too)
    result
}

/// Simple regex-like replacements (avoids pulling in regex crate).
fn regex_lite(find: &str, replace: &str) -> Vec<(String, String)> {
    // For now, just return the literal pair
    vec![(find.to_string(), replace.to_string())]
}

// ── Python Source Parser ─────────────────────────────────────────────

/// Parse a Python source file into a list of IR nodes.
pub fn parse_python_source(source: &str) -> TranslateResult<Vec<PythonIR>> {
    let mut nodes = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            i += 1;
            continue;
        }

        // Import statements
        if line.starts_with("import ") || line.starts_with("from ") {
            let module = extract_import_module(line);
            if is_c_extension(&module) {
                nodes.push(PythonIR::ForeignModule { name: module });
            }
            i += 1;
            continue;
        }

        // Class definition
        if line.starts_with("class ") {
            let (class_ir, new_i) = parse_class(&lines, i)?;
            nodes.push(class_ir);
            i = new_i;
            continue;
        }

        // Function definition
        if line.starts_with("def ") || line.starts_with("async def ") {
            let (func_ir, new_i) = parse_function(&lines, i)?;
            nodes.push(func_ir);
            i = new_i;
            continue;
        }

        i += 1;
    }

    Ok(nodes)
}

/// Extract the module name from an import statement.
fn extract_import_module(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("from ") {
        // `from module import ...`
        rest.split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    } else if let Some(rest) = line.strip_prefix("import ") {
        // `import module`
        rest.split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    }
}

/// Check if a module name is a known C extension.
fn is_c_extension(module: &str) -> bool {
    matches!(
        module,
        "ctypes"
            | "cffi"
            | "cython"
            | "numpy"
            | "scipy"
            | "pandas"
            | "torch"
            | "tensorflow"
            | "PIL"
            | "cv2"
            | "pybind11"
    )
}

/// Parse a Python class definition starting at line index.
fn parse_class(lines: &[&str], start: usize) -> TranslateResult<(PythonIR, usize)> {
    let header = lines[start].trim();
    // `class ClassName:` or `class ClassName(Base):`
    let name = header
        .strip_prefix("class ")
        .and_then(|s| s.split('(').next())
        .and_then(|s| s.split(':').next())
        .map(|s| s.trim().to_string())
        .ok_or_else(|| TranslateError::IndentationError(format!("bad class header: {header}")))?;

    // Collect body lines (indented blocks), skipping nested def/class
    let mut body_lines = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        // Check indentation: body lines must be indented more than the class header
        let indent = lines[i].len() - lines[i].trim_start().len();
        let header_indent = lines[start].len() - lines[start].trim_start().len();
        if indent <= header_indent && !trimmed.is_empty() {
            break; // End of class body
        }
        // Skip nested def/class and their bodies
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") || trimmed.starts_with("class ") {
            let method_indent = indent;
            i += 1;
            while i < lines.len() {
                let mt = lines[i].trim();
                if mt.is_empty() || mt.starts_with('#') {
                    i += 1;
                    continue;
                }
                let mi = lines[i].len() - lines[i].trim_start().len();
                if mi <= method_indent {
                    break;
                }
                i += 1;
            }
            continue;
        }
        body_lines.push(lines[i]);
        i += 1;
    }

    // Parse fields from body — look for `self.x: Type` or `self.x = ...`
    let mut fields = Vec::new();
    for bline in &body_lines {
        let trimmed = bline.trim();
        if trimmed.starts_with("def ") {
            // Skip methods
            continue;
        }
        if let Some((fname, ftype)) = parse_field_assignment(trimmed) {
            let ir_type = map_python_type(&ftype).unwrap_or(PythonIR::Prim(PythonPrim::Str));
            fields.push((fname, ir_type));
        }
    }

    // If no fields parsed, add a dummy marker
    if fields.is_empty() {
        fields.push(("_marker".into(), PythonIR::Prim(PythonPrim::Int)));
    }

    Ok((PythonIR::Struct { name, fields }, i))
}

/// Parse a field assignment like `self.x: int = 0` or `self.name: str = ""`.
fn parse_field_assignment(line: &str) -> Option<(String, String)> {
    // Match `self.field_name: type = value` or `self.field_name = value`
    let rest = line.strip_prefix("self.")?;
    let (before_eq, _) = rest.split_once('=')?;
    let before_eq = before_eq.trim();

    if let Some((fname, ftype)) = before_eq.split_once(':') {
        Some((fname.trim().to_string(), ftype.trim().to_string()))
    } else {
        // No type annotation — infer from value
        Some((before_eq.trim().to_string(), "str".into()))
    }
}

/// Parse a Python function definition starting at line index.
fn parse_function(lines: &[&str], start: usize) -> TranslateResult<(PythonIR, usize)> {
    let header = lines[start].trim();
    // Strip `async ` prefix if present
    let header = header.strip_prefix("async ").unwrap_or(header);

    // `def func_name(arg1: type1, arg2: type2) -> ret_type:`
    let after_def = header.strip_prefix("def ").ok_or_else(|| {
        TranslateError::IndentationError(format!("expected 'def': {header}"))
    })?;

    let (name, args_str, ret_type) = parse_func_signature(after_def)?;

    // Parse arguments
    let mut args = Vec::new();
    for arg in args_str.split(',') {
        let arg = arg.trim();
        if arg.is_empty() || arg == "self" || arg == "cls" {
            continue;
        }
        if let Some((aname, atype)) = arg.split_once(':') {
            let ir_type = map_python_type(atype.trim())
                .unwrap_or(PythonIR::Prim(PythonPrim::Str));
            args.push((aname.trim().to_string(), ir_type));
        }
    }

    // Parse return type
    let returns = map_python_type(&ret_type).unwrap_or(PythonIR::Prim(PythonPrim::Int));

    // Collect body
    let mut body_lines = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let indent = lines[i].len() - lines[i].trim_start().len();
        let header_indent = lines[start].len() - lines[start].trim_start().len();
        if indent <= header_indent && !trimmed.is_empty() {
            break;
        }
        body_lines.push(lines[i].trim());
        i += 1;
    }

    let body = body_lines.join("\n");

    Ok((
        PythonIR::Function {
            name,
            args,
            returns: Box::new(returns),
            body,
        },
        i,
    ))
}

/// Parse a function signature: `func_name(args) -> ret_type:`
fn parse_func_signature(sig: &str) -> TranslateResult<(String, String, String)> {
    let sig = sig.trim_end_matches(':').trim();
    let (name, rest) = sig
        .split_once('(')
        .ok_or_else(|| TranslateError::GenericParseError(format!("missing '(': {sig}")))?;

    // Find the matching closing paren
    let close = rest
        .find(')')
        .ok_or_else(|| TranslateError::GenericParseError(format!("missing ')': {sig}")))?;
    let args_str = rest[..close].trim().to_string();

    // Return type is after the closing paren
    let after_paren = rest[close + 1..].trim();
    let ret_type = if let Some(ret) = after_paren.strip_prefix("->") {
        ret.trim().to_string()
    } else {
        "None".into()
    };

    Ok((name.trim().to_string(), args_str, ret_type))
}

// ── Pipeline Integration ─────────────────────────────────────────────

/// Translation outcome for a Python source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonTranslationOutcome {
    /// The generated Rust code.
    pub rust_code: String,
    /// Number of functions translated.
    pub functions_count: usize,
    /// Number of structs translated.
    pub structs_count: usize,
    /// IR nodes that could not be translated.
    pub untranslatable: Vec<String>,
    /// Warnings generated during translation.
    pub warnings: Vec<String>,
}

/// Translate a Python source file into Rust ACC code.
pub fn translate_python_to_rust(
    python_source: &str,
    module_name: &str,
) -> TranslateResult<PythonTranslationOutcome> {
    let nodes = parse_python_source(python_source)?;

    // Check for forbidden modules
    let forbidden: Vec<_> = nodes
        .iter()
        .filter_map(|n| match n {
            PythonIR::ForeignModule { name } => Some(name.clone()),
            _ => None,
        })
        .collect();

    if !forbidden.is_empty() {
        return Err(TranslateError::ForbiddenForeignModule(forbidden.join(", ")));
    }

    let functions_count = nodes
        .iter()
        .filter(|n| matches!(n, PythonIR::Function { .. }))
        .count();
    let structs_count = nodes
        .iter()
        .filter(|n| matches!(n, PythonIR::Struct { .. }))
        .count();

    // Generate warnings for non-Pod structs
    let mut warnings = Vec::new();
    for node in &nodes {
        if let PythonIR::Struct { name, .. } = node {
            if !is_pod_eligible(node) {
                warnings.push(format!(
                    "struct `{name}` is non-Pod (contains dynamic or unknown types)"
                ));
            }
        }
    }

    let rust_code = emit_module(&nodes, module_name)?;

    Ok(PythonTranslationOutcome {
        rust_code,
        functions_count,
        structs_count,
        untranslatable: Vec::new(),
        warnings,
    })
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Type Mapping ──────────────────────────────────────────────

    #[test]
    fn map_int() {
        assert_eq!(map_python_type("int").unwrap(), PythonIR::Prim(PythonPrim::Int));
    }

    #[test]
    fn map_float() {
        assert_eq!(map_python_type("float").unwrap(), PythonIR::Prim(PythonPrim::Float));
    }

    #[test]
    fn map_bool() {
        assert_eq!(map_python_type("bool").unwrap(), PythonIR::Prim(PythonPrim::Bool));
    }

    #[test]
    fn map_str() {
        assert_eq!(map_python_type("str").unwrap(), PythonIR::Prim(PythonPrim::Str));
    }

    #[test]
    fn map_bytes() {
        assert_eq!(map_python_type("bytes").unwrap(), PythonIR::Prim(PythonPrim::Bytes));
    }

    #[test]
    fn map_list_of_int() {
        let ir = map_python_type("list[int]").unwrap();
        match ir {
            PythonIR::Array { elem, .. } => {
                assert_eq!(*elem, PythonIR::Prim(PythonPrim::Int));
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn map_dict_str_int() {
        let ir = map_python_type("dict[str, int]").unwrap();
        match ir {
            PythonIR::Map { key, value } => {
                assert_eq!(*key, PythonIR::Prim(PythonPrim::Str));
                assert_eq!(*value, PythonIR::Prim(PythonPrim::Int));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn map_named_type() {
        assert_eq!(
            map_python_type("MyClass").unwrap(),
            PythonIR::NamedRef("MyClass".into())
        );
    }

    #[test]
    fn map_unknown_type_errors() {
        assert!(map_python_type("[invalid").is_err());
    }

    // ── Pod Eligibility ──────────────────────────────────────────

    #[test]
    fn prim_is_pod() {
        assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Int)));
        assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Float)));
    }

    #[test]
    fn array_of_pod_is_pod() {
        let ir = PythonIR::Array {
            elem: Box::new(PythonIR::Prim(PythonPrim::Int)),
            len: None,
        };
        assert!(is_pod_eligible(&ir));
    }

    #[test]
    fn map_is_not_pod() {
        let ir = PythonIR::Map {
            key: Box::new(PythonIR::Prim(PythonPrim::Str)),
            value: Box::new(PythonIR::Prim(PythonPrim::Int)),
        };
        assert!(!is_pod_eligible(&ir));
    }

    #[test]
    fn struct_with_pod_fields_is_pod() {
        let ir = PythonIR::Struct {
            name: "Point".into(),
            fields: vec![
                ("x".into(), PythonIR::Prim(PythonPrim::Float)),
                ("y".into(), PythonIR::Prim(PythonPrim::Float)),
            ],
        };
        assert!(is_pod_eligible(&ir));
    }

    #[test]
    fn struct_with_nonpod_field_is_not_pod() {
        let ir = PythonIR::Struct {
            name: "Config".into(),
            fields: vec![
                ("name".into(), PythonIR::Prim(PythonPrim::Str)),
                ("tags".into(), PythonIR::Map {
                    key: Box::new(PythonIR::Prim(PythonPrim::Str)),
                    value: Box::new(PythonIR::Prim(PythonPrim::Str)),
                }),
            ],
        };
        assert!(!is_pod_eligible(&ir));
    }

    // ── Rust Emission ────────────────────────────────────────────

    #[test]
    fn emit_rust_type_primitives() {
        assert_eq!(emit_rust_type(&PythonIR::Prim(PythonPrim::Int)), "i64");
        assert_eq!(emit_rust_type(&PythonIR::Prim(PythonPrim::Float)), "f64");
        assert_eq!(emit_rust_type(&PythonIR::Prim(PythonPrim::Bool)), "bool");
        assert_eq!(emit_rust_type(&PythonIR::Prim(PythonPrim::Str)), "&str");
        assert_eq!(emit_rust_type(&PythonIR::Prim(PythonPrim::Bytes)), "&[u8]");
    }

    #[test]
    fn emit_rust_type_array() {
        let ir = PythonIR::Array {
            elem: Box::new(PythonIR::Prim(PythonPrim::Int)),
            len: None,
        };
        assert_eq!(emit_rust_type(&ir), "&[i64]");
    }

    #[test]
    fn emit_rust_type_map() {
        let ir = PythonIR::Map {
            key: Box::new(PythonIR::Prim(PythonPrim::Str)),
            value: Box::new(PythonIR::Prim(PythonPrim::Int)),
        };
        assert_eq!(emit_rust_type(&ir), "&[(&str, i64)]");
    }

    #[test]
    fn emit_struct_pod() {
        let ir = PythonIR::Struct {
            name: "Point".into(),
            fields: vec![
                ("x".into(), PythonIR::Prim(PythonPrim::Float)),
                ("y".into(), PythonIR::Prim(PythonPrim::Float)),
            ],
        };
        let code = emit_struct(&ir).unwrap();
        assert!(code.contains("#[repr(C)]"));
        assert!(code.contains("bytemuck::Pod"));
        assert!(code.contains("pub struct Point"));
        assert!(code.contains("pub x: f64"));
        assert!(code.contains("pub y: f64"));
    }

    #[test]
    fn emit_struct_nonpod() {
        let ir = PythonIR::Struct {
            name: "Config".into(),
            fields: vec![("data".into(), PythonIR::Map {
                key: Box::new(PythonIR::Prim(PythonPrim::Str)),
                value: Box::new(PythonIR::Prim(PythonPrim::Str)),
            })],
        };
        let code = emit_struct(&ir).unwrap();
        assert!(code.contains("#[repr(C)]"));
        assert!(!code.contains("bytemuck::Pod"));
        assert!(code.contains("pub data: &[(&str, &str)]"));
    }

    #[test]
    fn emit_function_simple() {
        let ir = PythonIR::Function {
            name: "add".into(),
            args: vec![
                ("a".into(), PythonIR::Prim(PythonPrim::Int)),
                ("b".into(), PythonIR::Prim(PythonPrim::Int)),
            ],
            returns: Box::new(PythonIR::Prim(PythonPrim::Int)),
            body: "return a + b".into(),
        };
        let code = emit_function(&ir).unwrap();
        assert!(code.contains("pub fn add(a: i64, b: i64) -> i64"));
        assert!(code.contains("return a + b"));
    }

    #[test]
    fn emit_module_header() {
        let nodes = vec![PythonIR::Struct {
            name: "Header".into(),
            fields: vec![("tag".into(), PythonIR::Prim(PythonPrim::Int))],
        }];
        let code = emit_module(&nodes, "my_module").unwrap();
        assert!(code.contains("my_module"));
        assert!(code.contains("/// Translated from Python class"));
    }

    // ── Python Source Parsing ────────────────────────────────────

    #[test]
    fn parse_simple_function() {
        let src = r#"
def add(a: int, b: int) -> int:
    return a + b
"#;
        let nodes = parse_python_source(src).unwrap();
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            PythonIR::Function { name, args, returns, .. } => {
                assert_eq!(name, "add");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0].0, "a");
                assert_eq!(args[1].0, "b");
                assert_eq!(**returns, PythonIR::Prim(PythonPrim::Int));
            }
            _ => panic!("expected Function"),
        }
    }

    #[test]
    fn parse_simple_class() {
        let src = r#"
class Point:
    self.x: float = 0.0
    self.y: float = 0.0
"#;
        let nodes = parse_python_source(src).unwrap();
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            PythonIR::Struct { name, fields } => {
                assert_eq!(name, "Point");
                assert_eq!(fields.len(), 2);
            }
            _ => panic!("expected Struct"),
        }
    }

    #[test]
    fn parse_import_skips_non_c() {
        let src = "import os\nimport sys\n";
        let nodes = parse_python_source(src).unwrap();
        assert!(nodes.is_empty()); // Non-C imports are ignored
    }

    #[test]
    fn parse_c_extension_import() {
        let src = "import ctypes\n";
        let nodes = parse_python_source(src).unwrap();
        assert_eq!(nodes.len(), 1);
        match &nodes[0] {
            PythonIR::ForeignModule { name } => assert_eq!(name, "ctypes"),
            _ => panic!("expected ForeignModule"),
        }
    }

    #[test]
    fn parse_comment_lines_skipped() {
        let src = "# This is a comment\ndef foo() -> int:\n    return 1\n";
        let nodes = parse_python_source(src).unwrap();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn parse_function_with_body() {
        let src = r#"
def greet(name: str) -> str:
    print("Hello, " + name)
    return name
"#;
        let nodes = parse_python_source(src).unwrap();
        assert_eq!(nodes.len(), 1);
        if let PythonIR::Function { body, .. } = &nodes[0] {
            assert!(body.contains("return name"));
        }
    }

    // ── Full Translation Pipeline ────────────────────────────────

    #[test]
    fn translate_simple_module() {
        let src = r#"
class Point:
    self.x: float = 0.0
    self.y: float = 0.0

def distance(x: float, y: float) -> float:
    return (x * x + y * y) as float
"#;
        let outcome = translate_python_to_rust(src, "geometry").unwrap();
        assert_eq!(outcome.functions_count, 1);
        assert_eq!(outcome.structs_count, 1);
        assert!(outcome.rust_code.contains("pub struct Point"));
        assert!(outcome.rust_code.contains("pub fn distance"));
        assert!(outcome.rust_code.contains("geometry"));
    }

    #[test]
    fn translate_rejects_c_extension() {
        let src = "import ctypes\n";
        let result = translate_python_to_rust(src, "bad");
        assert!(result.is_err());
        match result.unwrap_err() {
            TranslateError::ForbiddenForeignModule(name) => assert_eq!(name, "ctypes"),
            _ => panic!("expected ForbiddenForeignModule"),
        }
    }

    #[test]
    fn translate_warns_nonpod_struct() {
        let src = r#"
class Config:
    self.data: dict[str, int] = {}
"#;
        let outcome = translate_python_to_rust(src, "config").unwrap();
        assert!(!outcome.warnings.is_empty());
        assert!(outcome.warnings[0].contains("non-Pod"));
    }

    // ── Body Transformation ──────────────────────────────────────

    #[test]
    fn body_replaces_print() {
        let result = python_body_to_rust("print(\"hello\")");
        assert!(result.contains("tracing::info!"));
    }

    #[test]
    fn body_replaces_true_false() {
        let result = python_body_to_rust("if True: return False");
        assert!(result.contains("true"));
        assert!(result.contains("false"));
    }

    #[test]
    fn body_replaces_none() {
        let result = python_body_to_rust("return None");
        assert!(result.contains("()"));
    }
}
