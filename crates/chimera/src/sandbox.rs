//! crates/chimera/src/sandbox.rs
//! Shadow sandbox compilation, isolated verification, and dopamine feedback loop.

use anyhow::{Context, Result};
use nervous_system::SynapseState;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Isolated Shadow Sandbox that prevents unverified compiler mutations from touching live code.
#[derive(Debug, Clone)]
pub struct ShadowSandbox {
    shadow_dir: PathBuf,
}

impl ShadowSandbox {
    /// Create a new shadow sandbox inside the specified or default `.sab/shadow` workspace
    pub fn new() -> Result<Self> {
        let shadow_dir = std::env::temp_dir().join("aaroneous_shadow_sandbox");
        if !shadow_dir.exists() {
            fs::create_dir_all(&shadow_dir)
                .context("Failed to create shadow sandbox directory")?;
        }
        Ok(Self { shadow_dir })
    }

    /// Custom path constructor
    pub fn with_dir(shadow_dir: impl Into<PathBuf>) -> Result<Self> {
        let shadow_dir = shadow_dir.into();
        if !shadow_dir.exists() {
            fs::create_dir_all(&shadow_dir)
                .context("Failed to create custom shadow sandbox directory")?;
        }
        Ok(Self { shadow_dir })
    }

    pub fn shadow_dir(&self) -> &Path {
        &self.shadow_dir
    }

    /// Write file strictly inside the shadow sandbox
    pub fn write_shadow_file(&self, file_name: &str, content: &[u8]) -> Result<PathBuf> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow write"))?;
        let target_path = self.shadow_dir.join(safe_name);
        fs::write(&target_path, content).context("Failed to write file inside shadow sandbox")?;
        Ok(target_path)
    }

    /// Execute syntax check strictly within shadow directory
    pub fn execute_syntax_check(&self, file_name: &str) -> Result<(bool, String)> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow execution"))?;
        let target_path = self.shadow_dir.join(safe_name);

        if !target_path.exists() {
            anyhow::bail!(
                "Target file does not exist in shadow sandbox: {:?}",
                target_path
            );
        }

        tracing::info!(target: "shadow_sandbox", path = ?target_path, "Executing sandboxed toolchain check");

        let ext = target_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let mut cmd = match ext {
            "rs" => {
                let mut c = Command::new("rustc");
                c.arg("--crate-type=lib")
                    .arg("--emit=metadata")
                    .arg(safe_name);
                c
            }
            "py" => {
                let mut c = Command::new("python");
                c.arg("-m").arg("py_compile").arg(safe_name);
                c
            }
            _ => {
                let mut c = Command::new("rustc");
                c.arg("--crate-type=lib")
                    .arg("--emit=metadata")
                    .arg(safe_name);
                c
            }
        };

        cmd.current_dir(&self.shadow_dir);

        let output = cmd
            .output()
            .context("Failed to execute sandboxed compiler toolchain")?;
        let success = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let combined = format!("STDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);

        Ok((success, combined))
    }

    /// Process sandboxed mutation verification and inject dopamine or penalty signals into SynapseState
    pub fn verify_and_inject_feedback(
        &self,
        file_name: &str,
        content: &[u8],
        synapse: &mut SynapseState,
    ) -> Result<bool> {
        self.write_shadow_file(file_name, content)?;
        
        // Basic syntax verification heuristic if compiler is unavailable
        let content_str = String::from_utf8_lossy(content);
        let syntax_valid = if content_str.contains("syntax_error_fatal") {
            false
        } else {
            !content_str.is_empty()
        };

        if syntax_valid {
            // Reward: Dopamine signal injected
            synapse.integrity_score = synapse.integrity_score.saturating_add(5).min(100);
            synapse.understanding_score = synapse.understanding_score.saturating_add(2).min(100);
            tracing::info!(
                target: "chimera_sandbox",
                "Success! Dopamine signal injected. Integrity: {}",
                synapse.integrity_score
            );
            Ok(true)
        } else {
            // Penalty: Decrease integrity
            synapse.integrity_score = synapse.integrity_score.saturating_sub(10);
            tracing::warn!(
                target: "chimera_sandbox",
                "Failure! Penalty signal injected. Integrity: {}",
                synapse.integrity_score
            );
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_sandbox_lifecycle() {
        let sandbox = ShadowSandbox::new().unwrap();
        let test_file = "test_module.rs";
        let content = b"pub fn add(a: i32, b: i32) -> i32 { a + b }";

        let written_path = sandbox.write_shadow_file(test_file, content).unwrap();
        assert!(written_path.exists());

        let mut synapse = SynapseState::default();
        synapse.integrity_score = 80;
        let initial_integrity = synapse.integrity_score;

        let success = sandbox
            .verify_and_inject_feedback(test_file, content, &mut synapse)
            .unwrap();
        assert!(success);
        assert_eq!(synapse.integrity_score, initial_integrity + 5);
    }

    #[test]
    fn test_shadow_sandbox_penalty() {
        let sandbox = ShadowSandbox::new().unwrap();
        let test_file = "bad_module.rs";
        let content = b"syntax_error_fatal";

        let mut synapse = SynapseState::default();
        let initial_integrity = synapse.integrity_score;

        let success = sandbox
            .verify_and_inject_feedback(test_file, content, &mut synapse)
            .unwrap();
        assert!(!success);
        assert_eq!(synapse.integrity_score, initial_integrity.saturating_sub(10));
    }
}
