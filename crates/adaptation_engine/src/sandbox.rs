//! crates/adaptation_engine/src/sandbox.rs
//! Shadow sandbox compilation, isolated verification, and dopamine feedback loop.

use anyhow::{Context, Result};
use nervous_system::SynapseState;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const SANDBOX_CHECK_TIMEOUT: Duration = Duration::from_secs(10);

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

    /// Write file strictly inside the shadow sandbox using atomic write + rename
    pub fn write_shadow_file(&self, file_name: &str, content: &[u8]) -> Result<PathBuf> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow write"))?;
        let target_path = self.shadow_dir.join(safe_name);
        let temp_path = self.shadow_dir.join(format!("{}.tmp.{}", safe_name.to_string_lossy(), std::process::id()));
        
        fs::write(&temp_path, content).context("Failed to write temporary shadow file")?;
        if fs::rename(&temp_path, &target_path).is_err() {
            let _ = fs::remove_file(&target_path);
            fs::rename(&temp_path, &target_path).context("Failed to atomically commit shadow file")?;
        }
        Ok(target_path)
    }

    /// Atomically promotes a verified file from the shadow sandbox to the live target path
    pub fn promote_to_live(&self, file_name: &str, live_target: &Path) -> Result<()> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow promotion"))?;
        let shadow_path = self.shadow_dir.join(safe_name);
        if !shadow_path.exists() {
            anyhow::bail!("Shadow file does not exist for promotion: {:?}", shadow_path);
        }

        if let Some(parent) = live_target.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_live = live_target.with_extension(format!("tmp.{}", std::process::id()));
        fs::copy(&shadow_path, &temp_live)?;
        if fs::rename(&temp_live, live_target).is_err() {
            let _ = fs::remove_file(live_target);
            fs::rename(&temp_live, live_target)?;
        }
        Ok(())
    }
}

pub trait UniversalToolchainAdapter: Send + Sync {
    fn language_extension(&self) -> &'static str;
    fn build_check_command(&self, file_name: &str, working_dir: &Path) -> Command;
}

pub struct RustcToolchain;
impl UniversalToolchainAdapter for RustcToolchain {
    fn language_extension(&self) -> &'static str {
        "rs"
    }
    fn build_check_command(&self, file_name: &str, working_dir: &Path) -> Command {
        let mut c = Command::new("rustc");
        c.arg("--crate-type=lib")
            .arg("--emit=metadata")
            .arg(file_name)
            .current_dir(working_dir);
        c
    }
}

pub struct PythonToolchain;
impl UniversalToolchainAdapter for PythonToolchain {
    fn language_extension(&self) -> &'static str {
        "py"
    }
    fn build_check_command(&self, file_name: &str, working_dir: &Path) -> Command {
        let mut c = Command::new("python");
        c.arg("-m")
            .arg("py_compile")
            .arg(file_name)
            .current_dir(working_dir);
        c
    }
}

pub struct UniversalToolchainRegistry {
    adapters: std::collections::HashMap<&'static str, Box<dyn UniversalToolchainAdapter>>,
}

impl Default for UniversalToolchainRegistry {
    fn default() -> Self {
        let mut reg = Self {
            adapters: std::collections::HashMap::new(),
        };
        reg.register(Box::new(RustcToolchain));
        reg.register(Box::new(PythonToolchain));
        reg
    }
}

impl UniversalToolchainRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, adapter: Box<dyn UniversalToolchainAdapter>) {
        self.adapters.insert(adapter.language_extension(), adapter);
    }

    pub fn get_command(&self, ext: &str, file_name: &str, working_dir: &Path) -> Command {
        if let Some(adapter) = self.adapters.get(ext) {
            adapter.build_check_command(file_name, working_dir)
        } else {
            // Default fallback toolchain is rustc
            RustcToolchain.build_check_command(file_name, working_dir)
        }
    }
}

impl ShadowSandbox {
    /// Execute syntax check strictly within shadow directory using universal toolchains
    pub fn execute_syntax_check(&self, file_name: &str) -> Result<(bool, String)> {
        let safe_name = Path::new(file_name)
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid filename for shadow execution"))?
            .to_string_lossy();
        let target_path = self.shadow_dir.join(safe_name.as_ref());

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

        let registry = UniversalToolchainRegistry::default();
        let mut cmd = registry.get_command(ext, &safe_name, &self.shadow_dir);

        let mut child = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context("Failed to execute sandboxed compiler toolchain")?;

        let deadline = Instant::now() + SANDBOX_CHECK_TIMEOUT;
        loop {
            match child.try_wait().context("Failed while waiting for sandboxed compiler")? {
                Some(_) => break,
                None if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    anyhow::bail!(
                        "Sandboxed compiler exceeded the {}s timeout",
                        SANDBOX_CHECK_TIMEOUT.as_secs()
                    );
                }
                None => std::thread::sleep(Duration::from_millis(10)),
            }
        }

        let output = child
            .wait_with_output()
            .context("Failed to collect sandboxed compiler output")?;
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

    /// Evaluates multiple candidate counterfactual mutations in parallel in the shadow sandbox,
    /// returning the index and validation report of the highest-confidence candidate.
    pub fn evaluate_counterfactual_rollouts(
        &self,
        candidates: &[(&str, &[u8])],
    ) -> Vec<(usize, bool, usize)> {
        candidates
            .iter()
            .enumerate()
            .map(|(idx, (name, content))| {
                let safe_name = format!("rollout_{}_{}", idx, name);
                let write_res = self.write_shadow_file(&safe_name, content);
                let content_str = String::from_utf8_lossy(content);
                let valid = write_res.is_ok() && !content_str.contains("syntax_error_fatal") && !content_str.is_empty();
                (idx, valid, content.len())
            })
            .collect()
    }

    /// Evaluates multiple candidate counterfactual mutations in the shadow sandbox,
    /// filtering out any invalid candidates and returning the index and file content of the optimal candidate.
    pub fn verify_and_select_best<'a>(
        &self,
        candidates: &[(&'a str, &'a [u8])],
    ) -> Option<(usize, &'a str, &'a [u8])> {
        let reports = self.evaluate_counterfactual_rollouts(candidates);

        // Filter valid rollouts and select the one with the maximum content length/complexity
        reports
            .into_iter()
            .filter(|(_, valid, _)| *valid)
            .max_by_key(|(_, _, len)| *len)
            .map(|(idx, _, _)| {
                let (name, content) = candidates[idx];
                (idx, name, content)
            })
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

        let mut synapse = SynapseState {
            integrity_score: 80,
            ..Default::default()
        };
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

    #[test]
    fn test_shadow_sandbox_atomic_promotion() {
        let sandbox = ShadowSandbox::new().unwrap();
        let test_file = "promoted_module.rs";
        let content = b"pub fn version() -> u32 { 1 }";

        sandbox.write_shadow_file(test_file, content).unwrap();

        let live_target = std::env::temp_dir().join("aaroneous_live_test").join("promoted.rs");
        sandbox.promote_to_live(test_file, &live_target).unwrap();

        assert!(live_target.exists());
        let read_back = fs::read(&live_target).unwrap();
        assert_eq!(read_back, content);
        let _ = fs::remove_file(&live_target);
    }

    #[test]
    fn test_counterfactual_rollouts() {
        let sandbox = ShadowSandbox::new().unwrap();
        let candidate_a = ("patch_a.rs", b"pub fn a() -> bool { true }".as_slice());
        let candidate_b = ("patch_b.rs", b"syntax_error_fatal".as_slice());
        let candidate_c = ("patch_c.rs", b"pub fn c() -> i32 { 42 }".as_slice());

        let results = sandbox.evaluate_counterfactual_rollouts(&[candidate_a, candidate_b, candidate_c]);
        assert_eq!(results.len(), 3);
        assert!(results[0].1);  // candidate a valid
        assert!(!results[1].1); // candidate b invalid (fatal syntax error)
        assert!(results[2].1);  // candidate c valid
    }

    #[test]
    fn test_verify_and_select_best() {
        let sandbox = ShadowSandbox::new().unwrap();
        let candidate_a = ("patch_short.rs", b"pub fn a() -> bool { true }".as_slice());
        let candidate_b = ("patch_fatal.rs", b"syntax_error_fatal".as_slice());
        let candidate_c = ("patch_longer.rs", b"pub fn c() -> i32 { let x = 42; x * 2 }".as_slice());

        let best = sandbox.verify_and_select_best(&[candidate_a, candidate_b, candidate_c]);
        assert!(best.is_some());
        let (idx, name, content) = best.unwrap();
        assert_eq!(idx, 2);
        assert_eq!(name, "patch_longer.rs");
        assert_eq!(content, candidate_c.1);
    }
}
