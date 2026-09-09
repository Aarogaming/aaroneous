//! Saturation harness — second-tier stress tests for Cratify.
//!
//! Covers complex generics, malformed manifests, translation edge cases,
//! workspace boundary conditions, and deep AST inspection scenarios.

use cratify::inspect;
use cratify::audit::{self, AuditConfig, AuditRule};
use cratify::translate::{self, TranslateConfig, LlmRoute, LlmOffloadPacket, TranslationComplexity};
use cratify::inspect::CodeInfo;
use tempfile::NamedTempFile;
use std::io::Write;
use std::fs;

// ══════════════════════════════════════════════════════════════════════
//  SECTION 1 — Complex Generics & Type Signatures
// ══════════════════════════════════════════════════════════════════════

#[test]
fn complex_generics_function_with_where_clause() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub fn process<T, U>(items: Vec<T>, mapper: U) -> Vec<T::Output>
        where
            T: std::ops::AddAssign + Clone + Send + Sync + 'static,
            U: Fn(T) -> T::Output,
            T::Output: Default,
        {{
            let mut results = Vec::new();
            for item in items {{
                results.push(mapper(item));
            }}
            results
        }}
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).expect("inspect");
    assert_eq!(info.functions.len(), 1);
    let func = &info.functions[0];
    assert_eq!(func.name, "process");
    // Inputs should contain complex generic types
    assert!(!func.inputs.is_empty());
    // Output should contain Vec<T::Output>
    let output = func.output.as_ref().expect("should have return type");
    assert!(output.contains("Vec"));
}

#[test]
fn complex_generics_struct_with_lifetimes() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub struct BorrowedBuffer<'a, T: 'a> {{
            pub data: &'a [T],
            pub metadata: &'a str,
            pub len: usize,
        }}

        pub struct OwnedCache<K, V>
        where
            K: Eq + std::hash::Hash,
            V: Clone,
        {{
            pub entries: std::collections::HashMap<K, V>,
            pub capacity: usize,
        }}
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).expect("inspect");
    assert_eq!(info.structs.len(), 2);
    let names: Vec<&str> = info.structs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"BorrowedBuffer"));
    assert!(names.contains(&"OwnedCache"));
}

#[test]
fn complex_generics_associated_types() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub trait Processor {{
            type Input;
            type Output;
            type Error;

            fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;
        }}

        pub struct ImageProcessor;

        impl Processor for ImageProcessor {{
            type Input = Vec<u8>;
            type Output = String;
            type Error = std::io::Error;

            fn process(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {{
                Ok(format!("processed {{}} bytes", input.len()))
            }}
        }}
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).expect("inspect");
    assert_eq!(info.traits.len(), 1);
    assert_eq!(info.traits[0].name, "Processor");
    assert_eq!(info.impls.len(), 1);
    assert_eq!(info.impls[0].self_type, "ImageProcessor");
    assert_eq!(
        info.impls[0].trait_name.as_deref(),
        Some("Processor")
    );
}

#[test]
fn nested_tuple_struct_fields_flagged() {
    // Struct with nested tuple types should NOT be flagged as complex
    // (tuples are scalar-like, no heap allocation)
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub struct Coordinate {{
            pub pos: (f64, f64),
            pub id: u64,
        }}
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).expect("inspect");
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::ZeroCopyStructs);
    // (f64, f64) is not String/Vec/Box/HashMap/Rc/Arc — should be flagged
    assert!(
        !violations.is_empty(),
        "nested tuple struct should be flagged for Pod derive"
    );
}

// ══════════════════════════════════════════════════════════════════════
//  SECTION 2 — Malformed Manifests & Boundary Conditions
// ══════════════════════════════════════════════════════════════════════

#[test]
fn inspect_empty_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "").unwrap();

    let info = inspect::inspect_code(file.path()).expect("inspect empty file");
    assert!(info.functions.is_empty());
    assert!(info.structs.is_empty());
    assert!(info.enums.is_empty());
    assert!(info.traits.is_empty());
}

#[test]
fn inspect_file_with_only_comments() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        "// This is a line comment\n/* Block comment */\n/// Doc comment\n"
    )
    .unwrap();

    // syn::parse_file requires at least one item — comment-only files fail to parse
    let result = inspect::inspect_code(file.path());
    assert!(result.is_err(), "comment-only file should fail to parse");
}

#[test]
fn inspect_invalid_rust_file_returns_error() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "this is not valid rust code {{{{").unwrap();

    let result = inspect::inspect_code(file.path());
    assert!(result.is_err(), "invalid Rust should return Err");
}

#[test]
fn audit_empty_code_info_produces_no_violations() {
    let info = CodeInfo::default();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("empty.rs", &info, &config);
    assert_eq!(report.violations.len(), 0);
    assert_eq!(report.files_scanned, 1);
    assert_eq!(report.structs_scanned, 0);
    assert_eq!(report.functions_scanned, 0);
}

#[test]
fn audit_all_rules_skipped_produces_clean_report() {
    let info = CodeInfo {
        functions: vec![cratify::inspect::FunctionInfo {
            name: "do_unsafe_thing".into(),
            visibility: "pub".into(),
            inputs: vec![],
            output: None,
            body: "unsafe { }".into(),
        }],
        ..Default::default()
    };
    let config = AuditConfig {
        skip_rules: vec![
            AuditRule::NoUnsafe,
            AuditRule::NoUnwrap,
            AuditRule::NoPanic,
            AuditRule::NoPrintln,
            AuditRule::ZeroCopyStructs,
            AuditRule::ForbiddenDeps,
            AuditRule::ScalingLawBlastRadius,
        ],
        ..Default::default()
    };
    let report = audit::audit_code_info("bad.rs", &info, &config);
    assert_eq!(report.violations.len(), 0);
}

// ══════════════════════════════════════════════════════════════════════
//  SECTION 3 — Translation Router & Fallback Edge Cases
// ══════════════════════════════════════════════════════════════════════

#[test]
fn ast_fallback_preserves_string_with_unsafe_keyword() {
    let packet = LlmOffloadPacket {
        task_id: 100,
        route: LlmRoute::AstFallback,
        complexity: TranslationComplexity::Trivial,
        source_path: "test.rs".into(),
        target_crate: "test".into(),
        source_code: r#"fn check() {
    let msg = "unsafe blocks are forbidden";
    println!("{}", msg);
}"#
        .into(),
        code_info: CodeInfo::default(),
        instructions: String::new(),
        endpoint_override: None,
        created_at_us: 0,
    };

    let outcome = translate::execute_ast_fallback(&packet);
    assert!(outcome.success);
    let code = outcome.translated_code.unwrap();
    // The string literal must NOT be corrupted
    assert!(
        code.contains(r#""unsafe blocks are forbidden""#),
        "string literal was corrupted by unsafe replacement: {}",
        code
    );
    // println should still be replaced
    assert!(code.contains("tracing::info!"));
}

#[test]
fn ast_fallback_handles_deeply_nested_control_flow() {
    let source = r#"
fn deeply_nested() {
    for i in 0..10 {
        if i > 5 {
            match i {
                6 => println!("six"),
                7 => println!("seven"),
                _ => {
                    for j in 0..i {
                        if j % 2 == 0 {
                            println!("even: {}", j);
                        }
                    }
                }
            }
        }
    }
}
"#;
    let packet = LlmOffloadPacket {
        task_id: 101,
        route: LlmRoute::AstFallback,
        complexity: TranslationComplexity::Medium,
        source_path: "test.rs".into(),
        target_crate: "test".into(),
        source_code: source.into(),
        code_info: CodeInfo::default(),
        instructions: String::new(),
        endpoint_override: None,
        created_at_us: 0,
    };

    let outcome = translate::execute_ast_fallback(&packet);
    assert!(outcome.success);
    let code = outcome.translated_code.unwrap();
    assert!(!code.contains("println!"), "all println! should be replaced");
    assert!(code.contains("tracing::info!"));
}

#[test]
fn ast_fallback_handles_escape_sequences_in_strings() {
    let source = r#"fn check() {
    let raw = "line1\nline2\ttab";
    let quoted = "say \"hello\"";
    println!("done");
}"#;
    let packet = LlmOffloadPacket {
        task_id: 102,
        route: LlmRoute::AstFallback,
        complexity: TranslationComplexity::Trivial,
        source_path: "test.rs".into(),
        target_crate: "test".into(),
        source_code: source.into(),
        code_info: CodeInfo::default(),
        instructions: String::new(),
        endpoint_override: None,
        created_at_us: 0,
    };

    let outcome = translate::execute_ast_fallback(&packet);
    assert!(outcome.success);
    let code = outcome.translated_code.unwrap();
    // Strings with escape sequences must survive intact
    assert!(code.contains(r#""line1\nline2\ttab""#));
    assert!(code.contains(r#""say \"hello\"""#));
}

#[test]
fn translation_complexity_estimation_with_new_item_types() {
    let info = CodeInfo {
        functions: vec![
            cratify::inspect::FunctionInfo {
                name: "f1".into(),
                visibility: "pub".into(),
                inputs: vec![],
                output: None,
                body: String::new(),
            },
            cratify::inspect::FunctionInfo {
                name: "f2".into(),
                visibility: "pub".into(),
                inputs: vec![],
                output: None,
                body: String::new(),
            },
        ],
        structs: vec![cratify::inspect::StructInfo {
            name: "S".into(),
            visibility: "pub".into(),
            fields: vec![],
        }],
        enums: vec![cratify::inspect::EnumInfo {
            name: "E".into(),
            visibility: "pub".into(),
            variants: vec![],
        }],
        traits: vec![cratify::inspect::TraitInfo {
            name: "T".into(),
            visibility: "pub".into(),
            supertraits: vec![],
            methods: vec![],
        }],
        impls: vec![],
        type_aliases: vec![],
        consts: vec![],
        statics: vec![],
    };
    // 2 functions + 1 struct + 1 enum + 1 trait = 5 items → Low
    let complexity = translate::estimate_complexity(&info);
    assert_eq!(complexity, TranslationComplexity::Low);
}

#[test]
fn build_offload_packet_from_complex_generic_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub fn convert<T: std::fmt::Display>(item: T) -> String {{
            format!("{{}}", item)
        }}
        pub struct Wrapper<'a, T: 'a> {{
            pub inner: &'a T,
        }}
        "#
    )
    .unwrap();

    let config = TranslateConfig::default();
    let packet = translate::build_offload_packet(file.path(), "complex_acc", &config).unwrap();
    assert_eq!(packet.target_crate, "complex_acc");
    assert_eq!(packet.code_info.functions.len(), 1);
    assert_eq!(packet.code_info.structs.len(), 1);
}

// ══════════════════════════════════════════════════════════════════════
//  SECTION 4 — Audit Engine Edge Cases
// ══════════════════════════════════════════════════════════════════════

#[test]
fn false_positive_unwrap_inside_raw_string_literal() {
    let file = NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        "pub fn safe_fn() {\n\
         \x20   let msg = r#\"this contains .unwrap() in a raw string\"#;\n\
         \x20   let x = 42;\n\
         }\n",
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoUnwrap);
    assert!(
        violations.is_empty(),
        "raw string containing .unwrap() must not trigger: {:?}",
        violations
    );
}

#[test]
fn false_positive_panic_inside_multi_hash_raw_string() {
    let file = NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        "pub fn safe_fn() {\n\
         \x20   let template = r##\"this has panic! inside a double-hash raw string\"##;\n\
         \x20   let x = 42;\n\
         }\n",
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoPanic);
    assert!(
        violations.is_empty(),
        "multi-hash raw string with panic! must not trigger: {:?}",
        violations
    );
}

#[test]
fn false_positive_println_inside_char_literal_context() {
    // println! inside a string that also has char literals
    let file = NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        "pub fn safe_fn() {\n\
         \x20   let c = 'a';\n\
         \x20   let msg = \"use println! carefully\";\n\
         \x20   let d = '\\n';\n\
         \x20   let x = 42;\n\
         }\n",
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoPrintln);
    assert!(
        violations.is_empty(),
        "println! inside string with char literals must not trigger: {:?}",
        violations
    );
}

#[test]
fn audit_detects_unwrap_in_actual_code_not_in_strings() {
    let file = NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        "pub fn risky() {\n\
         \x20   let msg = \".unwrap() is safe in a string\";\n\
         \x20   let val = \"42\".parse::<u32>().unwrap();\n\
         }\n",
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoUnwrap);
    assert!(
        !violations.is_empty(),
        "actual .unwrap() in code must be detected"
    );
}

#[test]
fn audit_detects_unsafe_in_actual_code_not_in_strings() {
    let file = NamedTempFile::new().unwrap();
    fs::write(
        file.path(),
        "pub fn do_stuff() {\n\
         \x20   let warning = \"unsafe blocks are bad\";\n\
         \x20   unsafe {\n\
         \x20       let p = 0x1 as *mut u32;\n\
         \x20   }\n\
         }\n",
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    let config = AuditConfig::default();
    let report = audit::audit_code_info("test.rs", &info, &config);
    let violations = report.violations_for_rule(AuditRule::NoUnsafe);
    assert!(
        !violations.is_empty(),
        "actual unsafe block must be detected"
    );
}

// ══════════════════════════════════════════════════════════════════════
//  SECTION 5 — Workspace & Generator Boundary Conditions
// ══════════════════════════════════════════════════════════════════════

#[test]
fn workspace_find_root_nonexistent_path_returns_error() {
    let result = cratify::workspace::find_workspace_root(
        std::path::Path::new("/nonexistent/path/that/does/not/exist"),
    );
    assert!(result.is_err());
}

#[test]
fn generator_rejects_empty_crate_name() {
    let result = cratify::generator::generate_crate("");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("empty"), "error should mention empty: {}", err);
}

#[test]
fn generator_rejects_crate_name_starting_with_number() {
    let result = cratify::generator::generate_crate("1invalid");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("letter"), "error should mention letter: {}", err);
}

#[test]
fn generator_rejects_crate_name_with_special_chars() {
    let result = cratify::generator::generate_crate("bad name!@#");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("invalid characters"),
        "error should mention invalid chars: {}",
        err
    );
}

#[test]
fn generator_rejects_crate_name_too_long() {
    let long_name = "a".repeat(65);
    let result = cratify::generator::generate_crate(&long_name);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("length"), "error should mention length: {}", err);
}

// ══════════════════════════════════════════════════════════════════════
//  SECTION 6 — Inspect: New Item Types
// ══════════════════════════════════════════════════════════════════════

#[test]
fn inspect_trait_definition() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub trait Drawable: std::fmt::Debug + Clone {{
            fn draw(&self);
            fn area(&self) -> f64;
        }}
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    assert_eq!(info.traits.len(), 1);
    let tr = &info.traits[0];
    assert_eq!(tr.name, "Drawable");
    assert_eq!(tr.visibility, "pub");
    assert_eq!(tr.methods.len(), 2);
    assert!(tr.supertraits.iter().any(|s| s.contains("Debug")));
}

#[test]
fn inspect_impl_block() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub struct Counter {{ value: u64 }}

        impl Counter {{
            pub fn new() -> Self {{ Self {{ value: 0 }} }}
            pub fn increment(&mut self) {{ self.value += 1; }}
        }}

        impl std::fmt::Display for Counter {{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                write!(f, "Counter({{}})", self.value)
            }}
        }}
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    assert_eq!(info.impls.len(), 2);
    // First impl: no trait (inherent)
    assert!(info.impls[0].trait_name.is_none());
    assert_eq!(info.impls[0].methods.len(), 2);
    // Second impl: Display trait
    assert_eq!(
        info.impls[1].trait_name.as_deref(),
        Some("Display")
    );
}

#[test]
fn inspect_type_alias() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
        type InternalMap = std::collections::HashMap<String, Vec<u8>>;
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    assert_eq!(info.type_aliases.len(), 2);
    assert_eq!(info.type_aliases[0].name, "Result");
    assert_eq!(info.type_aliases[0].visibility, "pub");
    assert!(info.type_aliases[0].ty.contains("Result"));
    assert_eq!(info.type_aliases[1].name, "InternalMap");
    assert_eq!(info.type_aliases[1].visibility, "private");
}

#[test]
fn inspect_const_and_static() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        pub const MAX_SIZE: usize = 1024;
        const VERSION: u32 = 1;
        static mut COUNTER: u64 = 0;
        static BUFFER: [u8; 256] = [0u8; 256];
        "#
    )
    .unwrap();

    let info = inspect::inspect_code(file.path()).unwrap();
    assert_eq!(info.consts.len(), 2);
    assert_eq!(info.consts[0].name, "MAX_SIZE");
    assert_eq!(info.consts[0].visibility, "pub");
    assert_eq!(info.consts[1].name, "VERSION");

    assert_eq!(info.statics.len(), 2);
    assert_eq!(info.statics[0].name, "COUNTER");
    assert!(info.statics[0].is_mut);
    assert_eq!(info.statics[1].name, "BUFFER");
    assert!(!info.statics[1].is_mut);
}

#[test]
fn inspect_code_info_summary_includes_all_fields() {
    let info = CodeInfo::default();
    let summary = info.summary();
    assert!(summary.contains("Functions: 0"));
    assert!(summary.contains("Traits: 0"));
    assert!(summary.contains("Impls: 0"));
    assert!(summary.contains("TypeAliases: 0"));
    assert!(summary.contains("Consts: 0"));
    assert!(summary.contains("Statics: 0"));
}
