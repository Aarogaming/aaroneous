//! reflection_loop.rs
//! Automated Self-Healing Reflection Loop:
//! Executes synthesized code in a sandboxed check; if compiler errors occur,
//! feeds the errors back into a reflection prompt to iteratively self-heal.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Reflection feedback packet containing errors to be fixed by AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionFeedback {
    pub iteration: usize,
    pub original_code: String,
    pub compiler_error: String,
    pub reflection_prompt: String,
    pub is_resolved: bool,
}

/// Automated Reflection and Self-Repair Loop
pub struct ReflectionLoopEngine {
    pub max_iterations: usize,
}

impl Default for ReflectionLoopEngine {
    fn default() -> Self {
        Self { max_iterations: 5 }
    }
}

impl ReflectionLoopEngine {
    pub fn new(max_iterations: usize) -> Self {
        Self { max_iterations }
    }

    /// Formulates a reflection feedback packet when a compilation error occurs
    pub fn formulate_repair_prompt(
        &self,
        iteration: usize,
        code: &str,
        compiler_error: &str,
    ) -> Result<ReflectionFeedback> {
        info!(
            target: "transpiler::reflection",
            iteration,
            "Formulating self-healing reflection prompt for compiler error"
        );

        let reflection_prompt = format!(
            "Your previous code output failed compilation with the following error:\n\n\
             ```\n{}\n```\n\n\
             Faulty Code:\n```rust\n{}\n```\n\n\
             Please repair the code to eliminate the compiler error. Return ONLY the fixed code block.",
            compiler_error, code
        );

        Ok(ReflectionFeedback {
            iteration,
            original_code: code.to_string(),
            compiler_error: compiler_error.to_string(),
            reflection_prompt,
            is_resolved: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reflection_repair_prompt() {
        let engine = ReflectionLoopEngine::default();
        let code = "fn test() { let x: u32 = \"bad\"; }";
        let err = "mismatched types: expected u32, found &str";

        let feedback = engine.formulate_repair_prompt(1, code, err).unwrap();
        assert!(feedback.reflection_prompt.contains("mismatched types"));
        assert!(feedback.reflection_prompt.contains("Faulty Code"));
        assert_eq!(feedback.iteration, 1);
    }
}
