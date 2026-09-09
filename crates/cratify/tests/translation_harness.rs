//! Integration test harness for Cratify Python-to-Rust Translation Engine.
//!
//! Exercises the full pipeline: Python source parsing → PythonIR intermediate
//! representation → type mapping → Rust code emission. Covers primitive
//! types, generic containers, struct generation, function generation, C
//! extension rejection, and end-to-end module translation.

use cratify::python_to_rust::*;

// ══════════════════════════════════════════════════════════════════════
//  GROUP 1 — Primitive Type Mapping
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group1_map_int() {
    assert_eq!(
        map_python_type("int").unwrap(),
        PythonIR::Prim(PythonPrim::Int)
    );
}

#[test]
fn group1_map_float() {
    assert_eq!(
        map_python_type("float").unwrap(),
        PythonIR::Prim(PythonPrim::Float)
    );
}

#[test]
fn group1_map_bool() {
    assert_eq!(
        map_python_type("bool").unwrap(),
        PythonIR::Prim(PythonPrim::Bool)
    );
}

#[test]
fn group1_map_str() {
    assert_eq!(
        map_python_type("str").unwrap(),
        PythonIR::Prim(PythonPrim::Str)
    );
}

#[test]
fn group1_map_bytes() {
    assert_eq!(
        map_python_type("bytes").unwrap(),
        PythonIR::Prim(PythonPrim::Bytes)
    );
}

#[test]
fn group1_map_none_as_bool() {
    assert_eq!(
        map_python_type("None").unwrap(),
        PythonIR::Prim(PythonPrim::Bool)
    );
}

#[test]
fn group1_map_bare_list() {
    let ir = map_python_type("list").unwrap();
    match ir {
        PythonIR::Array { .. } => {}
        _ => panic!("expected Array for bare list"),
    }
}

#[test]
fn group1_map_bare_dict() {
    let ir = map_python_type("dict").unwrap();
    match ir {
        PythonIR::Map { key, value } => {
            assert_eq!(*key, PythonIR::Prim(PythonPrim::Str));
            assert_eq!(*value, PythonIR::Prim(PythonPrim::Str));
        }
        _ => panic!("expected Map for bare dict"),
    }
}

#[test]
fn group1_map_unknown_type_returns_error() {
    assert!(map_python_type("[invalid").is_err());
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 2 — Generic Container Mapping
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group2_list_of_int() {
    let ir = map_python_type("list[int]").unwrap();
    match ir {
        PythonIR::Array { elem, .. } => assert_eq!(*elem, PythonIR::Prim(PythonPrim::Int)),
        _ => panic!("expected Array"),
    }
}

#[test]
fn group2_list_of_str() {
    let ir = map_python_type("list[str]").unwrap();
    match ir {
        PythonIR::Array { elem, .. } => assert_eq!(*elem, PythonIR::Prim(PythonPrim::Str)),
        _ => panic!("expected Array"),
    }
}

#[test]
fn group2_list_of_float() {
    let ir = map_python_type("list[float]").unwrap();
    match ir {
        PythonIR::Array { elem, .. } => assert_eq!(*elem, PythonIR::Prim(PythonPrim::Float)),
        _ => panic!("expected Array"),
    }
}

#[test]
fn group2_dict_str_int() {
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
fn group2_dict_str_str() {
    let ir = map_python_type("dict[str, str]").unwrap();
    match ir {
        PythonIR::Map { key, value } => {
            assert_eq!(*key, PythonIR::Prim(PythonPrim::Str));
            assert_eq!(*value, PythonIR::Prim(PythonPrim::Str));
        }
        _ => panic!("expected Map"),
    }
}

#[test]
fn group2_dict_int_float() {
    let ir = map_python_type("dict[int, float]").unwrap();
    match ir {
        PythonIR::Map { key, value } => {
            assert_eq!(*key, PythonIR::Prim(PythonPrim::Int));
            assert_eq!(*value, PythonIR::Prim(PythonPrim::Float));
        }
        _ => panic!("expected Map"),
    }
}

#[test]
fn group2_nested_list() {
    // list[list[int]] should parse
    let ir = map_python_type("list[list[int]]");
    // Outer list wraps inner list
    match ir {
        Ok(PythonIR::Array { elem, .. }) => match *elem {
            PythonIR::Array { elem, .. } => assert_eq!(*elem, PythonIR::Prim(PythonPrim::Int)),
            _ => panic!("expected nested Array"),
        },
        _ => panic!("expected outer Array"),
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 3 — Pod Eligibility
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group3_all_primitives_are_pod() {
    assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Int)));
    assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Float)));
    assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Bool)));
    assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Str)));
    assert!(is_pod_eligible(&PythonIR::Prim(PythonPrim::Bytes)));
}

#[test]
fn group3_pod_array_of_primitives() {
    let ir = PythonIR::Array {
        elem: Box::new(PythonIR::Prim(PythonPrim::Int)),
        len: None,
    };
    assert!(is_pod_eligible(&ir));
}

#[test]
fn group3_pod_array_of_nonpod() {
    let ir = PythonIR::Array {
        elem: Box::new(PythonIR::Map {
            key: Box::new(PythonIR::Prim(PythonPrim::Str)),
            value: Box::new(PythonIR::Prim(PythonPrim::Int)),
        }),
        len: None,
    };
    assert!(!is_pod_eligible(&ir));
}

#[test]
fn group3_map_is_never_pod() {
    let ir = PythonIR::Map {
        key: Box::new(PythonIR::Prim(PythonPrim::Str)),
        value: Box::new(PythonIR::Prim(PythonPrim::Int)),
    };
    assert!(!is_pod_eligible(&ir));
}

#[test]
fn group3_struct_all_pod_fields_is_pod() {
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
fn group3_struct_with_map_field_is_not_pod() {
    let ir = PythonIR::Struct {
        name: "Config".into(),
        fields: vec![(
            "tags".into(),
            PythonIR::Map {
                key: Box::new(PythonIR::Prim(PythonPrim::Str)),
                value: Box::new(PythonIR::Prim(PythonPrim::Str)),
            },
        )],
    };
    assert!(!is_pod_eligible(&ir));
}

#[test]
fn group3_named_ref_is_not_pod() {
    let ir = PythonIR::NamedRef("MyClass".into());
    assert!(!is_pod_eligible(&ir));
}

#[test]
fn group3_function_is_not_pod() {
    let ir = PythonIR::Function {
        name: "foo".into(),
        args: vec![],
        returns: Box::new(PythonIR::Prim(PythonPrim::Int)),
        body: "return 0".into(),
    };
    assert!(!is_pod_eligible(&ir));
}

#[test]
fn group3_byteslice_is_pod() {
    let ir = PythonIR::ByteSlice { len: 32 };
    assert!(is_pod_eligible(&ir));
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 4 — Rust Code Emission: Types
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group4_emit_type_int() {
    assert_eq!(
        emit_rust_type(&PythonIR::Prim(PythonPrim::Int)),
        "i64"
    );
}

#[test]
fn group4_emit_type_float() {
    assert_eq!(
        emit_rust_type(&PythonIR::Prim(PythonPrim::Float)),
        "f64"
    );
}

#[test]
fn group4_emit_type_bool() {
    assert_eq!(
        emit_rust_type(&PythonIR::Prim(PythonPrim::Bool)),
        "bool"
    );
}

#[test]
fn group4_emit_type_str() {
    assert_eq!(
        emit_rust_type(&PythonIR::Prim(PythonPrim::Str)),
        "&str"
    );
}

#[test]
fn group4_emit_type_bytes() {
    assert_eq!(
        emit_rust_type(&PythonIR::Prim(PythonPrim::Bytes)),
        "&[u8]"
    );
}

#[test]
fn group4_emit_type_array() {
    let ir = PythonIR::Array {
        elem: Box::new(PythonIR::Prim(PythonPrim::Int)),
        len: None,
    };
    assert_eq!(emit_rust_type(&ir), "&[i64]");
}

#[test]
fn group4_emit_type_map() {
    let ir = PythonIR::Map {
        key: Box::new(PythonIR::Prim(PythonPrim::Str)),
        value: Box::new(PythonIR::Prim(PythonPrim::Int)),
    };
    assert_eq!(emit_rust_type(&ir), "&[(&str, i64)]");
}

#[test]
fn group4_emit_type_named_ref() {
    let ir = PythonIR::NamedRef("MyStruct".into());
    assert_eq!(emit_rust_type(&ir), "MyStruct");
}

#[test]
fn group4_emit_type_byteslice() {
    let ir = PythonIR::ByteSlice { len: 16 };
    assert_eq!(emit_rust_type(&ir), "&[u8]");
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 5 — Rust Code Emission: Structs
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group5_struct_pod_derives() {
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
    assert!(code.contains("bytemuck::Zeroable"));
    assert!(code.contains("pub struct Point"));
}

#[test]
fn group5_struct_nonpod_no_derives() {
    let ir = PythonIR::Struct {
        name: "Config".into(),
        fields: vec![(
            "data".into(),
            PythonIR::Map {
                key: Box::new(PythonIR::Prim(PythonPrim::Str)),
                value: Box::new(PythonIR::Prim(PythonPrim::Str)),
            },
        )],
    };
    let code = emit_struct(&ir).unwrap();
    assert!(code.contains("#[repr(C)]"));
    assert!(!code.contains("bytemuck::Pod"));
}

#[test]
fn group5_struct_fields() {
    let ir = PythonIR::Struct {
        name: "Vertex".into(),
        fields: vec![
            ("position".into(), PythonIR::Prim(PythonPrim::Float)),
            ("normal".into(), PythonIR::Prim(PythonPrim::Float)),
            ("uv".into(), PythonIR::Prim(PythonPrim::Float)),
        ],
    };
    let code = emit_struct(&ir).unwrap();
    assert!(code.contains("pub position: f64"));
    assert!(code.contains("pub normal: f64"));
    assert!(code.contains("pub uv: f64"));
}

#[test]
fn group5_struct_doc_comment() {
    let ir = PythonIR::Struct {
        name: "Header".into(),
        fields: vec![("tag".into(), PythonIR::Prim(PythonPrim::Int))],
    };
    let code = emit_struct(&ir).unwrap();
    assert!(code.contains("/// Translated from Python class `Header`"));
}

#[test]
fn group5_struct_empty_fields() {
    let ir = PythonIR::Struct {
        name: "Empty".into(),
        fields: vec![],
    };
    let code = emit_struct(&ir).unwrap();
    assert!(code.contains("pub struct Empty"));
    assert!(code.contains("}"));
}

#[test]
fn group5_emit_struct_rejects_non_struct() {
    let ir = PythonIR::Prim(PythonPrim::Int);
    assert!(emit_struct(&ir).is_err());
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 6 — Rust Code Emission: Functions
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group6_function_signature() {
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
}

#[test]
fn group6_function_body() {
    let ir = PythonIR::Function {
        name: "double".into(),
        args: vec![("x".into(), PythonIR::Prim(PythonPrim::Int))],
        returns: Box::new(PythonIR::Prim(PythonPrim::Int)),
        body: "return x * 2".into(),
    };
    let code = emit_function(&ir).unwrap();
    assert!(code.contains("return x * 2"));
}

#[test]
fn group6_function_no_args() {
    let ir = PythonIR::Function {
        name: "get_zero".into(),
        args: vec![],
        returns: Box::new(PythonIR::Prim(PythonPrim::Int)),
        body: "return 0".into(),
    };
    let code = emit_function(&ir).unwrap();
    assert!(code.contains("pub fn get_zero() -> i64"));
}

#[test]
fn group6_function_doc_comment() {
    let ir = PythonIR::Function {
        name: "compute".into(),
        args: vec![],
        returns: Box::new(PythonIR::Prim(PythonPrim::Int)),
        body: "return 42".into(),
    };
    let code = emit_function(&ir).unwrap();
    assert!(code.contains("/// Translated from Python function `compute`"));
}

#[test]
fn group6_function_complex_args() {
    let ir = PythonIR::Function {
        name: "process".into(),
        args: vec![
            ("data".into(), PythonIR::Prim(PythonPrim::Bytes)),
            ("flag".into(), PythonIR::Prim(PythonPrim::Bool)),
            ("label".into(), PythonIR::Prim(PythonPrim::Str)),
        ],
        returns: Box::new(PythonIR::Prim(PythonPrim::Int)),
        body: "return 0".into(),
    };
    let code = emit_function(&ir).unwrap();
    assert!(code.contains("pub fn process(data: &[u8], flag: bool, label: &str) -> i64"));
}

#[test]
fn group6_emit_function_rejects_non_function() {
    let ir = PythonIR::Prim(PythonPrim::Int);
    assert!(emit_function(&ir).is_err());
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 7 — Python Source Parsing
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group7_parse_single_function() {
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
            assert_eq!(args[0].1, PythonIR::Prim(PythonPrim::Int));
            assert_eq!(args[1].0, "b");
            assert_eq!(**returns, PythonIR::Prim(PythonPrim::Int));
        }
        _ => panic!("expected Function"),
    }
}

#[test]
fn group7_parse_single_class() {
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
            assert_eq!(fields[0].0, "x");
            assert_eq!(fields[1].0, "y");
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn group7_parse_empty_source() {
    let nodes = parse_python_source("").unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn group7_parse_comments_only() {
    let src = "# comment 1\n# comment 2\n";
    let nodes = parse_python_source(src).unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn group7_parse_blank_lines_only() {
    let src = "\n\n\n";
    let nodes = parse_python_source(src).unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn group7_parse_multiple_functions() {
    let src = r#"
def add(a: int, b: int) -> int:
    return a + b

def subtract(a: int, b: int) -> int:
    return a - b

def multiply(a: int, b: int) -> int:
    return a * b
"#;
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 3);
    let names: Vec<_> = nodes
        .iter()
        .filter_map(|n| match n {
            PythonIR::Function { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(names, vec!["add", "subtract", "multiply"]);
}

#[test]
fn group7_parse_function_without_return_type() {
    let src = "def noop() -> None:\n    pass\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        PythonIR::Function { returns, .. } => {
            // None maps to bool
            assert_eq!(**returns, PythonIR::Prim(PythonPrim::Bool));
        }
        _ => panic!("expected Function"),
    }
}

#[test]
fn group7_parse_class_with_methods_skipped() {
    let src = r#"
class Counter:
    self.count: int = 0
    def increment(self) -> None:
        self.count += 1
"#;
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        PythonIR::Struct { fields, .. } => {
            // Only the field should be parsed, method skipped
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].0, "count");
        }
        _ => panic!("expected Struct"),
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 8 — Import Handling & C Extension Rejection
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group8_non_c_imports_ignored() {
    let src = "import os\nimport sys\nfrom pathlib import Path\n";
    let nodes = parse_python_source(src).unwrap();
    assert!(nodes.is_empty());
}

#[test]
fn group8_ctypes_rejected() {
    let src = "import ctypes\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(&nodes[0], PythonIR::ForeignModule { name } if name == "ctypes"));
}

#[test]
fn group8_numpy_rejected() {
    let src = "import numpy\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(&nodes[0], PythonIR::ForeignModule { name } if name == "numpy"));
}

#[test]
fn group8_torch_rejected() {
    let src = "import torch\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(&nodes[0], PythonIR::ForeignModule { name } if name == "torch"));
}

#[test]
fn group8_pil_rejected() {
    let src = "from PIL import Image\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(&nodes[0], PythonIR::ForeignModule { name } if name == "PIL"));
}

#[test]
fn group8_cffi_rejected() {
    let src = "import cffi\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(&nodes[0], PythonIR::ForeignModule { name } if name == "cffi"));
}

#[test]
fn group8_cython_rejected() {
    let src = "import cython\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    assert!(matches!(&nodes[0], PythonIR::ForeignModule { name } if name == "cython"));
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 9 — Full Pipeline Translation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group9_translate_simple_module() {
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
fn group9_translate_empty_module() {
    let outcome = translate_python_to_rust("# nothing", "empty").unwrap();
    assert_eq!(outcome.functions_count, 0);
    assert_eq!(outcome.structs_count, 0);
    assert!(outcome.untranslatable.is_empty());
}

#[test]
fn group9_translate_rejects_ctypes() {
    let src = "import ctypes\n";
    let result = translate_python_to_rust(src, "bad");
    assert!(result.is_err());
    match result.unwrap_err() {
        TranslateError::ForbiddenForeignModule(name) => assert_eq!(name, "ctypes"),
        _ => panic!("expected ForbiddenForeignModule"),
    }
}

#[test]
fn group9_translate_rejects_numpy() {
    let src = "import numpy\n";
    let result = translate_python_to_rust(src, "bad");
    assert!(result.is_err());
    match result.unwrap_err() {
        TranslateError::ForbiddenForeignModule(name) => assert_eq!(name, "numpy"),
        _ => panic!("expected ForbiddenForeignModule"),
    }
}

#[test]
fn group9_translate_warns_nonpod_struct() {
    let src = r#"
class Config:
    self.data: dict[str, int] = {}
"#;
    let outcome = translate_python_to_rust(src, "config").unwrap();
    assert!(!outcome.warnings.is_empty());
    assert!(outcome.warnings[0].contains("non-Pod"));
}

#[test]
fn group9_translate_pod_struct_no_warnings() {
    let src = r#"
class Vec3:
    self.x: float = 0.0
    self.y: float = 0.0
    self.z: float = 0.0
"#;
    let outcome = translate_python_to_rust(src, "math3d").unwrap();
    assert!(outcome.warnings.is_empty());
}

#[test]
fn group9_translate_preserves_module_name() {
    let src = "def foo() -> int:\n    return 1\n";
    let outcome = translate_python_to_rust(src, "my_module_name").unwrap();
    assert!(outcome.rust_code.contains("my_module_name"));
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 10 — Module Emission
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group10_module_header() {
    let nodes = vec![PythonIR::Struct {
        name: "Header".into(),
        fields: vec![("tag".into(), PythonIR::Prim(PythonPrim::Int))],
    }];
    let code = emit_module(&nodes, "header").unwrap();
    assert!(code.contains("header"));
    assert!(code.contains("/// Translated from Python class"));
}

#[test]
fn group10_module_serde_when_nonpod() {
    let nodes = vec![PythonIR::Struct {
        name: "Cfg".into(),
        fields: vec![(
            "data".into(),
            PythonIR::Map {
                key: Box::new(PythonIR::Prim(PythonPrim::Str)),
                value: Box::new(PythonIR::Prim(PythonPrim::Str)),
            },
        )],
    }];
    let code = emit_module(&nodes, "cfg").unwrap();
    assert!(code.contains("use serde"));
}

#[test]
fn group10_module_no_serde_when_all_pod() {
    let nodes = vec![PythonIR::Struct {
        name: "Pt".into(),
        fields: vec![("x".into(), PythonIR::Prim(PythonPrim::Float))],
    }];
    let code = emit_module(&nodes, "pt").unwrap();
    assert!(!code.contains("use serde"));
}

#[test]
fn group10_module_mixed_structs_and_functions() {
    let nodes = vec![
        PythonIR::Struct {
            name: "Point".into(),
            fields: vec![("x".into(), PythonIR::Prim(PythonPrim::Float))],
        },
        PythonIR::Function {
            name: "distance".into(),
            args: vec![
                ("a".into(), PythonIR::Prim(PythonPrim::Float)),
                ("b".into(), PythonIR::Prim(PythonPrim::Float)),
            ],
            returns: Box::new(PythonIR::Prim(PythonPrim::Float)),
            body: "return a + b".into(),
        },
    ];
    let code = emit_module(&nodes, "geom").unwrap();
    assert!(code.contains("pub struct Point"));
    assert!(code.contains("pub fn distance"));
}

#[test]
fn group10_module_empty() {
    let code = emit_module(&[], "empty").unwrap();
    assert!(code.contains("empty"));
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 11 — Error Types
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group11_error_display_unmappable() {
    let e = TranslateError::UnmappableConstruct("decorator".into());
    assert!(e.to_string().contains("unmappable"));
    assert!(e.to_string().contains("decorator"));
}

#[test]
fn group11_error_display_forbidden() {
    let e = TranslateError::ForbiddenForeignModule("ctypes".into());
    assert!(e.to_string().contains("forbidden"));
    assert!(e.to_string().contains("ctypes"));
}

#[test]
fn group11_error_display_payload_too_large() {
    let e = TranslateError::PayloadTooLarge {
        name: "BigStruct".into(),
        size: 2048,
        limit: 1024,
    };
    assert!(e.to_string().contains("BigStruct"));
    assert!(e.to_string().contains("2048"));
}

#[test]
fn group11_error_display_generic_parse() {
    let e = TranslateError::GenericParseError("bad syntax".into());
    assert!(e.to_string().contains("generic parse"));
}

#[test]
fn group11_error_display_indentation() {
    let e = TranslateError::IndentationError("mixed tabs/spaces".into());
    assert!(e.to_string().contains("indentation"));
}

#[test]
fn group11_error_is_std_error() {
    let e: Box<dyn std::error::Error> =
        Box::new(TranslateError::UnmappableConstruct("test".into()));
    assert!(!e.to_string().is_empty());
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 12 — Edge Cases & Complex Inputs
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group12_function_with_multiline_body() {
    let src = r#"
def complex(x: int) -> int:
    a = x + 1
    b = a * 2
    return b - x
"#;
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    if let PythonIR::Function { body, .. } = &nodes[0] {
        assert!(body.contains("return b - x"));
        assert!(body.contains("a = x + 1"));
    }
}

#[test]
fn group12_class_with_no_fields() {
    let src = r#"
class Empty:
    pass
"#;
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        PythonIR::Struct { fields, .. } => {
            // No fields parsed → at least one marker
            assert!(!fields.is_empty());
        }
        _ => panic!("expected Struct"),
    }
}

#[test]
fn group12_function_with_long_name() {
    let name = "a".repeat(100);
    let src = format!("def {name}() -> int:\n    return 0\n");
    let nodes = parse_python_source(&src).unwrap();
    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        PythonIR::Function { name: n, .. } => assert_eq!(n.len(), 100),
        _ => panic!("expected Function"),
    }
}

#[test]
fn group12_import_with_multiple_modules() {
    let src = "import os, sys, json\n";
    let nodes = parse_python_source(src).unwrap();
    assert!(nodes.is_empty()); // None are C extensions
}

#[test]
fn group12_function_returning_str() {
    let src = "def greet() -> str:\n    return \"hello\"\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    if let PythonIR::Function { returns, .. } = &nodes[0] {
        assert_eq!(**returns, PythonIR::Prim(PythonPrim::Str));
    }
}

#[test]
fn group12_function_with_list_arg() {
    let src = "def sum_list(items: list[int]) -> int:\n    return 0\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    if let PythonIR::Function { args, .. } = &nodes[0] {
        assert_eq!(args.len(), 1);
        match &args[0].1 {
            PythonIR::Array { elem, .. } => assert_eq!(**elem, PythonIR::Prim(PythonPrim::Int)),
            _ => panic!("expected Array arg"),
        }
    }
}

#[test]
fn group12_translate_multiple_items() {
    let src = r#"
class Config:
    self.name: str = ""
    self.value: int = 0

def get_name(c: str) -> str:
    return c

def process(v: int) -> int:
    return v * 2
"#;
    let outcome = translate_python_to_rust(src, "mixed").unwrap();
    assert_eq!(outcome.structs_count, 1);
    assert_eq!(outcome.functions_count, 2);
    assert!(outcome.rust_code.contains("pub struct Config"));
    assert!(outcome.rust_code.contains("pub fn get_name"));
    assert!(outcome.rust_code.contains("pub fn process"));
}

#[test]
fn group12_async_def_parsing() {
    let src = "async def fetch() -> int:\n    return 0\n";
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    match &nodes[0] {
        PythonIR::Function { name, .. } => assert_eq!(name, "fetch"),
        _ => panic!("expected Function"),
    }
}

#[test]
fn group12_nested_comment_lines_skipped() {
    let src = r#"
# Top level comment

def foo() -> int:

    # Inside function comment
    return 1
"#;
    let nodes = parse_python_source(src).unwrap();
    assert_eq!(nodes.len(), 1);
    if let PythonIR::Function { body, .. } = &nodes[0] {
        assert!(body.contains("return 1"));
    }
}

// ══════════════════════════════════════════════════════════════════════
//  GROUP 13 — Serde Round-Trip (IR Serialization)
// ══════════════════════════════════════════════════════════════════════

#[test]
fn group13_ir_serializes_to_json() {
    let ir = PythonIR::Prim(PythonPrim::Int);
    let json = serde_json::to_string(&ir).unwrap();
    assert!(json.contains("Prim"));
    assert!(json.contains("Int"));
}

#[test]
fn group13_ir_deserializes_from_json() {
    let ir = PythonIR::Array {
        elem: Box::new(PythonIR::Prim(PythonPrim::Float)),
        len: None,
    };
    let json = serde_json::to_string(&ir).unwrap();
    let restored: PythonIR = serde_json::from_str(&json).unwrap();
    assert_eq!(ir, restored);
}

#[test]
fn group13_struct_roundtrip() {
    let ir = PythonIR::Struct {
        name: "TestStruct".into(),
        fields: vec![
            ("a".into(), PythonIR::Prim(PythonPrim::Int)),
            ("b".into(), PythonIR::Prim(PythonPrim::Str)),
        ],
    };
    let json = serde_json::to_string(&ir).unwrap();
    let restored: PythonIR = serde_json::from_str(&json).unwrap();
    assert_eq!(ir, restored);
}

#[test]
fn group13_function_roundtrip() {
    let ir = PythonIR::Function {
        name: "test_fn".into(),
        args: vec![("x".into(), PythonIR::Prim(PythonPrim::Int))],
        returns: Box::new(PythonIR::Prim(PythonPrim::Bool)),
        body: "return x > 0".into(),
    };
    let json = serde_json::to_string(&ir).unwrap();
    let restored: PythonIR = serde_json::from_str(&json).unwrap();
    assert_eq!(ir, restored);
}

#[test]
fn group13_outcome_serializes() {
    let outcome = PythonTranslationOutcome {
        rust_code: "pub fn foo() -> i64 { 0 }".into(),
        functions_count: 1,
        structs_count: 0,
        untranslatable: vec![],
        warnings: vec!["test warning".into()],
    };
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains("functions_count"));
    assert!(json.contains("test warning"));
}
