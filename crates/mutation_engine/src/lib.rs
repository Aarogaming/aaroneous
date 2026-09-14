// Mutation Engine - Automated Code Remediation with AST Transformation

pub mod unwrap_replacer;
pub mod layout_normalizer;
pub mod panic_replacer;

use anyhow::Result;

/// Main entry point for source code remediation
/// 
/// Currently uses text-based transformations as placeholder for full AST approach.
/// Future: Chain syn::visit_mut visitors (UnwrapReplacer, LayoutNormalizer, PanicReplacer)
pub fn remediate_source(source_code: &str) -> Result<String> {
    let mut result = source_code.to_string();

    // Replace .unwrap() with ok_or pattern  
    result = result.replace(".unwrap()", ".ok_or(|| anyhow::Error::msg(\"unwrap failed\"))?");

    // Replace panic! with Err() - simple heuristic for Result-returning functions
    result = result.replace("panic!", "return Err(anyhow::anyhow!(\"error occurred\"))");

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remediate_unwrap() {
        let input = r#"fn test() -> Result<i32, anyhow::Error> {
    let x: Option<i32> = None;
    x.unwrap();
    Ok(42)
}"#;

        let output = remediate_source(input).unwrap();
        
        assert!(!output.contains(".unwrap()"));
        assert!(output.contains("ok_or") || output.contains("?"));
    }

    #[test]
    fn test_remediate_panic() {
        let input = r#"pub fn process() -> Result<u32, anyhow::Error> {
    panic!("No data");
    Ok(42)
}"#;

        let output = remediate_source(input).unwrap();
        
        assert!(output.contains("Err(anyhow"));
        assert!(!output.contains("panic!"));
    }
}
