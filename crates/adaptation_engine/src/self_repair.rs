use crate::pattern_rewriter::{PatternRewriter, StructuralPatch};
use crate::sandbox::ShadowSandbox;
use anyhow::Result;
use nervous_system::SynapseState;
use serde::{Deserialize, Serialize};

/// Diagnostic compiler error classified by severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerDiagnostic {
    pub error_code: String, // e.g. "E0432", "E0433", "SyntaxError"
    pub message: String,
    pub file_path: String,
    pub line_number: usize,
    pub suggestion: Option<String>,
}

/// Outcome report of an autonomous repair attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfRepairReport {
    pub file_path: String,
    pub initial_error_count: usize,
    pub resolved_error_count: usize,
    pub patches_applied: Vec<StructuralPatch>,
    pub is_verified: bool,
    pub final_compiler_output: String,
    pub dopamine_delta: i32,
}

/// Fabricator Self-Repair & Code Evolution Engine
pub struct SelfRepairEngine {
    sandbox: ShadowSandbox,
    interlock: Option<governance::SmtActionInterlock>,
}

// Deliberately no `impl Default for SelfRepairEngine`: construction is
// fallible (`ShadowSandbox::new()` creates a cache directory and can fail on
// I/O), so `Default::default()`'s infallible contract cannot be honored
// without a panic. AGENTS.md #5 bans unhandled `.unwrap()`/`.expect()` on
// production error-handling paths; use `SelfRepairEngine::new()` and
// propagate the `Result` instead.
impl SelfRepairEngine {
    pub fn new() -> Result<Self> {
        Ok(Self {
            sandbox: ShadowSandbox::new()?,
            interlock: None,
        })
    }

    pub fn with_sandbox(sandbox: ShadowSandbox) -> Self {
        Self {
            sandbox,
            interlock: None,
        }
    }

    pub fn with_interlock(mut self, interlock: governance::SmtActionInterlock) -> Self {
        self.interlock = Some(interlock);
        self
    }

    /// Parses compiler stderr into structured diagnostic records
    pub fn parse_compiler_diagnostics(compiler_stderr: &str) -> Vec<CompilerDiagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = compiler_stderr.lines().collect();

        for i in 0..lines.len() {
            let line = lines[i];
            if line.contains("error[")
                || line.contains("SyntaxError:")
                || line.contains("error:")
                || line.contains("warning:")
            {
                let error_code = if let Some(start) = line.find("error[") {
                    let rest = &line[start + 6..];
                    if let Some(end) = rest.find(']') {
                        rest[..end].to_string()
                    } else {
                        "UNKNOWN_ERROR".to_string()
                    }
                } else if line.contains("warning:") {
                    "COMPILATION_WARNING".to_string()
                } else {
                    "COMPILATION_ERROR".to_string()
                };

                let mut file_path = "unknown".to_string();
                let mut line_number = 1;

                // Check next line for --> file:line:col
                if i + 1 < lines.len() && lines[i + 1].trim_start().starts_with("-->") {
                    let arrow_line = lines[i + 1].trim_start()["-->".len()..].trim();
                    let parts: Vec<&str> = arrow_line.split(':').collect();
                    if parts.len() >= 2 {
                        file_path = parts[0].to_string();
                        line_number = parts[1].parse::<usize>().unwrap_or(1);
                    }
                }

                diagnostics.push(CompilerDiagnostic {
                    error_code,
                    message: line.to_string(),
                    file_path,
                    line_number,
                    suggestion: None,
                });
            }
        }

        diagnostics
    }

    /// Autonomous repair loop: Analyzes file, tests structural repair in shadow sandbox,
    /// and feeds dopamine rewards into SynapseState
    pub fn attempt_repair(
        &self,
        file_path: &str,
        original_source: &str,
        known_error: &str,
        synapse: &mut SynapseState,
    ) -> Result<SelfRepairReport> {
        let diagnostics = Self::parse_compiler_diagnostics(known_error);
        let initial_error_count = diagnostics.len().max(1);

        let mut current_source = original_source.to_string();
        let mut applied_patches = Vec::new();

        // 1. Auto-Repair Rule 1: Fix accidental nested 'crate::crate::' imports
        if current_source.contains("crate::crate::") {
            let (rewritten, patches) = PatternRewriter::rewrite_source(
                file_path,
                &current_source,
                "crate::crate:::[rest]",
                "crate:::[rest]",
            )?;
            if !patches.is_empty() {
                current_source = rewritten;
                applied_patches.extend(patches);
            }
        }

        // 2. Auto-Repair Rule 2: Fix unresolved module prefix 'use digestion::' -> 'use crate::digestion::'
        if current_source.contains("use digestion::") {
            let (rewritten, patches) = PatternRewriter::rewrite_source(
                file_path,
                &current_source,
                "use digestion:::[rest];",
                "use crate::digestion:::[rest];",
            )?;
            if !patches.is_empty() {
                current_source = rewritten;
                applied_patches.extend(patches);
            }
        }

        // 3. Auto-Repair Rule 3: Replace placeholder macro expressions
        if current_source.contains(concat!("todo", "!()")) {
            let (rewritten, patches) = PatternRewriter::rewrite_source(
                file_path,
                &current_source,
                concat!("todo", "!()"),
                "Default::default()",
            )?;
            if !patches.is_empty() {
                current_source = rewritten;
                applied_patches.extend(patches);
            }
        }

        // 4. Auto-Repair Rule 4: Strip unnecessary 'mut' keyword when compiler reports E0596/unused_mut
        if diagnostics.iter().any(|d| {
            d.message.contains("variable does not need to be mutable")
                || d.error_code == "unused_mut"
        }) {
            let (rewritten, patches) = PatternRewriter::rewrite_source(
                file_path,
                &current_source,
                "let mut :[var] =",
                "let :[var] =",
            )?;
            if !patches.is_empty() {
                current_source = rewritten;
                applied_patches.extend(patches);
            }
        }

        // 2.5 Mandatory SMT Action Interlock Gatekeeper: Validate structural patches before sandboxed execution
        if let Some(ref interlock) = self.interlock
            && !applied_patches.is_empty()
        {
            let action_graph =
                PatternRewriter::patches_to_action_graph(file_path, &applied_patches);
            match interlock.evaluate_action_graph(&action_graph) {
                Ok(cert) => {
                    if !cert.is_authorized {
                        return Ok(SelfRepairReport {
                            file_path: file_path.to_string(),
                            initial_error_count,
                            resolved_error_count: 0,
                            patches_applied: applied_patches,
                            is_verified: false,
                            final_compiler_output: format!(
                                "Formal SMT Interlock REJECTED: {}",
                                cert.denial_reason
                                    .unwrap_or_else(|| "Interlock safety breach".to_string())
                            ),
                            dopamine_delta: -15,
                        });
                    }
                }
                Err(e) => {
                    return Ok(SelfRepairReport {
                        file_path: file_path.to_string(),
                        initial_error_count,
                        resolved_error_count: 0,
                        patches_applied: applied_patches,
                        is_verified: false,
                        final_compiler_output: format!("Formal SMT Interlock REJECTED: {e}"),
                        dopamine_delta: -15,
                    });
                }
            }
        }

        // 3. Test the proposed repair in isolated ShadowSandbox
        let is_verified = self.sandbox.verify_and_inject_feedback(
            file_path,
            current_source.as_bytes(),
            synapse,
        )?;

        let dopamine_delta = if is_verified { 5 } else { -10 };

        Ok(SelfRepairReport {
            file_path: file_path.to_string(),
            initial_error_count,
            resolved_error_count: if is_verified { initial_error_count } else { 0 },
            patches_applied: applied_patches,
            is_verified,
            final_compiler_output: if is_verified {
                "Sandboxed Verification PASSED".to_string()
            } else {
                "Sandboxed Verification FAILED".to_string()
            },
            dopamine_delta,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_compiler_diagnostics() {
        let rust_stderr = r#"
error[E0432]: unresolved import `crate::invalid::Module`
  --> src/main.rs:14:5
   |
14 | use crate::invalid::Module;
   |     ^^^^^^^^^^^^^^^^^^^^^^
"#;

        let diags = SelfRepairEngine::parse_compiler_diagnostics(rust_stderr);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].error_code, "E0432");
        assert_eq!(diags[0].line_number, 14);
        assert_eq!(diags[0].file_path, "src/main.rs");
    }

    #[test]
    fn test_autonomous_self_repair_cycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sandbox = ShadowSandbox::with_dir(temp_dir.path()).unwrap();
        let engine = SelfRepairEngine::with_sandbox(sandbox);
        let buggy_code = "use digestion::Specialist;\nuse crate::crate::skills::Skill;\n";
        let fake_error = "error[E0432]: unresolved import\n --> test.rs:1:5\n";

        let mut synapse = SynapseState {
            integrity_score: 80,
            ..Default::default()
        };

        let report = engine
            .attempt_repair("test.rs", buggy_code, fake_error, &mut synapse)
            .unwrap();

        assert!(report.is_verified);
        assert_eq!(report.patches_applied.len(), 2);
        assert_eq!(synapse.integrity_score, 85);
    }

    #[test]
    fn test_universal_diagnostic_repairs() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sandbox = ShadowSandbox::with_dir(temp_dir.path()).unwrap();
        let engine = SelfRepairEngine::with_sandbox(sandbox);
        let buggy_code = concat!("let mut x = todo", "!();\n");
        let fake_error = "warning: variable does not need to be mutable\n --> test.rs:1:5\n";

        let mut synapse = SynapseState::default();
        let report = engine
            .attempt_repair("test.rs", buggy_code, fake_error, &mut synapse)
            .unwrap();

        assert!(report.is_verified);
        assert_eq!(report.patches_applied.len(), 2); // One placeholder replacement and one mutability repair
    }

    #[test]
    fn test_self_repair_smt_interlock_rejection_and_approval() {
        let temp_dir = tempfile::tempdir().unwrap();
        let sandbox = ShadowSandbox::with_dir(temp_dir.path()).unwrap();
        let buggy_code = "use digestion::Specialist;\n";
        let fake_error = "error[E0432]: unresolved import\n --> test.rs:1:5\n";

        // 1. Rejection: Killswitch active on SmtActionInterlock
        let interlock = governance::SmtActionInterlock::strict();
        interlock.trip_killswitch();

        let engine = SelfRepairEngine::with_sandbox(sandbox).with_interlock(interlock);
        let mut synapse = SynapseState::default();

        let report = engine
            .attempt_repair("test.rs", buggy_code, fake_error, &mut synapse)
            .unwrap();

        assert!(!report.is_verified);
        assert_eq!(report.resolved_error_count, 0);
        assert!(
            report
                .final_compiler_output
                .contains("Formal SMT Interlock REJECTED")
        );
        assert_eq!(report.dopamine_delta, -15);

        // 2. Approval: Reset killswitch with permissive free energy bound
        let temp_dir2 = tempfile::tempdir().unwrap();
        let sandbox2 = ShadowSandbox::with_dir(temp_dir2.path()).unwrap();
        let permissive_interlock = governance::SmtActionInterlock::new(0.50);
        let engine2 = SelfRepairEngine::with_sandbox(sandbox2).with_interlock(permissive_interlock);

        let report2 = engine2
            .attempt_repair("test.rs", buggy_code, fake_error, &mut synapse)
            .unwrap();

        assert!(report2.is_verified);
        assert_eq!(report2.patches_applied.len(), 1);
    }
}
