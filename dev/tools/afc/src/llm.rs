// dev/tools/afc/src/llm.rs
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};

pub struct LlmOrchestrator;

impl LlmOrchestrator {
    async fn run_opencode_command(
        repo_path: &Path,
        args: &[&str],
        timeout_secs: u64,
        log_path: Option<&Path>,
    ) -> Result<String> {
        let mut cmd = Command::new("npx");
        cmd.current_dir(repo_path)
            .args(args)
            .kill_on_drop(true)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .context("Failed to spawn npx opencode process")?;

        let timeout_duration = Duration::from_secs(timeout_secs);
        let wait_result = timeout(timeout_duration, child.wait_with_output()).await;

        match wait_result {
            Ok(output_result) => {
                let output = output_result.context("Failed to read opencode output")?;
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let full_log = format!("{stdout}\n{stderr}");

                if let Some(path) = log_path {
                    let _ = fs::write(path, &full_log).await;
                }

                if !output.status.success() {
                    warn!(
                        "OpenCode command finished with exit code: {:?}",
                        output.status.code()
                    );
                }

                Ok(full_log)
            }
            Err(_) => {
                warn!(
                    "Watchdog timeout ({timeout_secs}s) reached. Terminated stalled LLM process."
                );
                bail!("LLM task exceeded watchdog timeout of {timeout_secs} seconds");
            }
        }
    }

    pub async fn run_plan(repo_path: &Path, timeout_secs: u64) -> Result<String> {
        info!("LlmOrchestrator: Initiating Phase 1 Plan...");
        let args = [
            "opencode",
            "run",
            "--agent",
            "architect",
            "--command",
            "plan",
            "--thinking",
            "--auto",
            "Top 1 high-depth architectural enhancement or missing crate README/spec sheet",
        ];
        Self::run_opencode_command(repo_path, &args, timeout_secs, None).await
    }

    pub async fn run_audit(repo_path: &Path, timeout_secs: u64) -> Result<String> {
        info!("LlmOrchestrator: Initiating Phase 2 Audit...");
        let args = [
            "opencode",
            "run",
            "--agent",
            "auditor",
            "--command",
            "audit",
            "--thinking",
            "--auto",
        ];
        Self::run_opencode_command(repo_path, &args, timeout_secs, None).await
    }

    pub async fn run_specialized_audit(
        repo_path: &Path,
        command: &str,
        focus_prompt: Option<&str>,
        timeout_secs: u64,
    ) -> Result<String> {
        info!("LlmOrchestrator: Initiating specialized audit command '{command}'...");
        let mut args = vec![
            "opencode",
            "run",
            "--agent",
            "auditor",
            "--command",
            command,
            "--thinking",
            "--auto",
        ];
        if let Some(prompt) = focus_prompt {
            args.push(prompt);
        }
        Self::run_opencode_command(repo_path, &args, timeout_secs, None).await
    }

    pub async fn run_fix(
        repo_path: &Path,
        task_title: &str,
        timeout_secs: u64,
        subtask_log: &Path,
    ) -> Result<String> {
        info!("LlmOrchestrator: Initiating Phase 3 Fix for '{task_title}'...");
        let prompt = format!("Remediate pending task: {task_title}");
        let args = [
            "opencode",
            "run",
            "--agent",
            "auditor",
            "--command",
            "fix",
            "--thinking",
            "--auto",
            &prompt,
        ];
        Self::run_opencode_command(repo_path, &args, timeout_secs, Some(subtask_log)).await
    }
}
