//! crates/chimera/src/error_interceptor.rs
//! Live compiler, process, and execution error interceptor.
//! Translates process exit failures and stderr traces into continuous error tensors
//! to trigger instant in-place online error-steering updates within the active .si model container.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Summary of an intercepted execution failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptedProcessError {
    pub exit_code: i32,
    pub error_codes_detected: Vec<String>,
    pub panic_message: Option<String>,
    pub error_vector_norm: f32,
    pub is_syntax_error: bool,
    pub is_type_error: bool,
}

/// Process Error Interceptor Engine
pub struct ProcessErrorInterceptor;

impl ProcessErrorInterceptor {
    /// Parses stderr and exit code into an InterceptedProcessError summary
    pub fn parse_stderr(stderr: &str, exit_code: i32) -> InterceptedProcessError {
        let mut error_codes = Vec::new();
        let mut panic_msg = None;
        let mut is_syntax = false;
        let mut is_type = false;

        for line in stderr.lines() {
            if line.contains("error[E") {
                if let Some(start) = line.find("error[E") {
                    if let Some(end) = line[start..].find(']') {
                        let code = &line[start + 6..start + end];
                        error_codes.push(format!("E{}", code));
                    }
                }
            }

            if line.contains("panicked at") {
                panic_msg = Some(line.trim().to_string());
            }

            if line.contains("syntax error") || line.contains("expected ") || line.contains("unexpected token") {
                is_syntax = true;
            }

            if line.contains("mismatched types") || line.contains("type mismatch") || line.contains("cannot find type") {
                is_type = true;
            }
        }

        let norm = if exit_code != 0 {
            (1.0 + error_codes.len() as f32 * 0.5 + if panic_msg.is_some() { 2.0 } else { 0.0 }).min(10.0)
        } else {
            0.0
        };

        InterceptedProcessError {
            exit_code,
            error_codes_detected: error_codes,
            panic_message: panic_msg,
            error_vector_norm: norm,
            is_syntax_error: is_syntax,
            is_type_error: is_type,
        }
    }

    /// Converts an execution failure into a normalized continuous error tensor in R^dim
    pub fn extract_error_vector(stderr: &str, exit_code: i32, dim: usize) -> Vec<f32> {
        let summary = Self::parse_stderr(stderr, exit_code);
        let mut error_vec = vec![0.0f32; dim];

        if summary.exit_code == 0 && summary.error_vector_norm == 0.0 {
            return error_vec;
        }

        // 1. Channel 0: Exit code severity
        error_vec[0] = (summary.exit_code.abs() as f32).clamp(1.0, 5.0);

        // 2. Channel 1: Syntax error flag
        if summary.is_syntax_error {
            error_vec[1 % dim] = 2.0;
        }

        // 3. Channel 2: Type error flag
        if summary.is_type_error {
            error_vec[2 % dim] = 2.5;
        }

        // 4. Channel 3: Panic severity
        if summary.panic_message.is_some() {
            error_vec[3 % dim] = 4.0;
        }

        // 5. Distributed hash channels for detected compiler error codes
        for (i, code) in summary.error_codes_detected.iter().enumerate() {
            let hash_val: usize = code.bytes().map(|b| b as usize).sum();
            let target_idx = (4 + i * 7 + hash_val) % dim;
            error_vec[target_idx] += 1.5;
        }

        error_vec
    }

    /// Intercepts process error output and invokes in-place online steering on an active SI learner
    pub fn intercept_and_steer(
        learner: &mut compute::SiOnlineLearner,
        current_state: &[f32],
        stderr: &str,
        exit_code: i32,
        lr: f32,
    ) -> Result<compute::OnlineCorrectionReport> {
        let error_dim = learner.container.adaptation.out_dim;
        let error_vec = Self::extract_error_vector(stderr, exit_code, error_dim);
        
        let report = learner.on_runtime_error(current_state, &error_vec, lr);
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_error_interception_and_vector_extraction() {
        let stderr_sample = r#"
error[E0433]: cannot find type `PathBuf` in this scope
  --> src/main.rs:10:12
error[E0308]: mismatched types
thread 'main' panicked at 'assertion failed: false', src/main.rs:15:5
"#;

        let summary = ProcessErrorInterceptor::parse_stderr(stderr_sample, 101);
        assert_eq!(summary.exit_code, 101);
        assert!(summary.panic_message.is_some());
        assert!(summary.is_type_error);
        assert_eq!(summary.error_codes_detected.len(), 2);
        assert!(summary.error_vector_norm > 0.0);

        let error_vec = ProcessErrorInterceptor::extract_error_vector(stderr_sample, 101, 32);
        assert_eq!(error_vec.len(), 32);
        assert!(error_vec.iter().any(|&v| v > 0.0));
    }
}
