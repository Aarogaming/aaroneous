// Mutation Engine - Automated Code Remediation

use anyhow::Result;

/// Main entry point for source code remediation
pub fn remediate_source(source_code: &str) -> Result<String> {
    let mut result = source_code.to_string();
    
    // Replace .unwrap() with ok_or_else pattern (simple string replacement)
    result = result.replace(".unwrap()", ".ok_or(|| anyhow::Error::msg(\"unwrap failed\"))?");

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remediate_unwrap() {
        let input = r#"fn test() {
    let x: Option<i32> = None;
    x.unwrap();
}"#;

        let output = remediate_source(input).unwrap();
        
        assert!(!output.contains(".unwrap()"));
        assert!(output.contains("ok_or"));
    }
}
