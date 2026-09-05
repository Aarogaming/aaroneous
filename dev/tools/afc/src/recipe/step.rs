// dev/tools/afc/src/recipe/step.rs
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct Step {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub timeout_duration: Duration,
    pub rollback_on_failure: bool,
}

#[derive(Debug, Clone)]
pub struct StepOutput {
    pub name: String,
    pub success: bool,
    pub code: i32,
    pub raw_stdout: String,
    pub raw_stderr: String,
    pub elapsed: Duration,
}

impl Step {
    pub fn new(name: impl Into<String>, command: impl Into<String>, cwd: PathBuf) -> Self {
        Self {
            name: name.into(),
            command: command.into(),
            args: Vec::new(),
            cwd,
            timeout_duration: Duration::from_secs(180),
            rollback_on_failure: false,
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for a in args {
            self.args.push(a.into());
        }
        self
    }

    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    pub fn with_rollback(mut self, rollback: bool) -> Self {
        self.rollback_on_failure = rollback;
        self
    }

    pub async fn execute(&self) -> Result<StepOutput> {
        let start = Instant::now();
        let mut cmd = Command::new(&self.command);
        cmd.current_dir(&self.cwd)
            .args(&self.args)
            .kill_on_drop(true);

        #[cfg(windows)]
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW

        let child = cmd.output();
        let output = timeout(self.timeout_duration, child)
            .await
            .context(format!(
                "Step '{}' timed out after {:?}",
                self.name, self.timeout_duration
            ))?
            .context(format!("Failed to spawn command '{}'", self.command))?;

        let elapsed = start.elapsed();
        Ok(StepOutput {
            name: self.name.clone(),
            success: output.status.success(),
            code: output.status.code().unwrap_or(-1),
            raw_stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            raw_stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            elapsed,
        })
    }
}
