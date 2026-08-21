use transpiler::{transpile, Transpiler, TranspilerConfig};

#[test]
fn test_transpile_rust() {
    let result = transpile("fn main() { println!(\"Hello\"); }", "rust");
    assert!(result.is_ok());
}

#[test]
fn test_transpile_python() {
    let result = transpile("print('hello')", "python");
    assert!(result.is_ok());
}

#[test]
fn test_transpile_with_config() {
    let config = TranspilerConfig {
        llm_model: "test-model".to_string(),
        max_iterations: 5,
        temperature: 0.5,
        hot_patch_enabled: false,
    };
    let mut t = Transpiler::new(config);
    let result = t.transpile("fn main() {}", "rust");
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.success);
}
