// dev/tools/afc/src/recipe/pipeline.rs
use crate::recipe::filter::DiagnosticsFilter;
use crate::recipe::step::{Step, StepOutput};
use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use tracing::{error, info};

#[derive(Debug, Clone)]
pub struct PipelineReport {
    pub name: String,
    pub passed: bool,
    pub step_results: Vec<StepOutput>,
    pub failure_summary: Option<String>,
}

pub struct RecipePipeline {
    name: String,
    steps: Vec<Step>,
    stop_on_first_error: bool,
}

impl RecipePipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
            stop_on_first_error: true,
        }
    }

    pub fn add_step(mut self, step: Step) -> Self {
        self.steps.push(step);
        self
    }

    pub fn with_continue_on_error(mut self, stop: bool) -> Self {
        self.stop_on_first_error = stop;
        self
    }

    /// Execute the pipeline sequentially
    pub async fn run(&self) -> Result<PipelineReport> {
        info!(
            "Executing Recipe Pipeline '{}' ({} steps)",
            self.name,
            self.steps.len()
        );
        let mut results = Vec::new();
        let mut passed = true;
        let mut failure_summary = None;

        for step in &self.steps {
            info!("Running Step '{}'...", step.name);
            let output = step.execute().await?;
            let step_passed = output.success;
            results.push(output.clone());

            if !step_passed {
                passed = false;
                error!("Step '{}' failed with exit code {}", step.name, output.code);
                let summary = DiagnosticsFilter::summarize_for_prompt(
                    &format!("{}\n{}", output.raw_stdout, output.raw_stderr),
                    5,
                );
                failure_summary = Some(summary);

                if self.stop_on_first_error {
                    break;
                }
            }
        }

        Ok(PipelineReport {
            name: self.name.clone(),
            passed,
            step_results: results,
            failure_summary,
        })
    }

    /// Pre-configured Verification Gate Pipeline
    pub fn verification_gates(
        repo_root: &Path,
        enforce_clippy: bool,
        enforce_test: bool,
        enforce_fmt: bool,
    ) -> Self {
        let mut pipeline = Self::new("Verification Gates");

        if enforce_fmt {
            let fmt_step = Step::new("Cargo Format Gate", "cargo", repo_root.to_path_buf())
                .args(["fmt", "--check"])
                .with_timeout(Duration::from_secs(60));
            pipeline = pipeline.add_step(fmt_step);
        }

        if enforce_clippy {
            let clippy_step = Step::new("Clippy Lint Gate", "cargo", repo_root.to_path_buf())
                .args([
                    "clippy",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ])
                .with_timeout(Duration::from_secs(300));
            pipeline = pipeline.add_step(clippy_step);
        }

        if enforce_test {
            let test_step = Step::new("Unit Test Gate", "cargo", repo_root.to_path_buf())
                .args(["test", "--workspace"])
                .with_timeout(Duration::from_secs(300));
            pipeline = pipeline.add_step(test_step);
        }

        pipeline
    }
}
