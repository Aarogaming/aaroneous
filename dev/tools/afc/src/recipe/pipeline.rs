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

    /// Pre-configured Verification Gate Pipeline executing via Git Bash
    pub fn verification_gates(
        repo_root: &Path,
        enforce_clippy: bool,
        enforce_test: bool,
        enforce_fmt: bool,
    ) -> Self {
        let mut pipeline = Self::new("Verification Gates");
        let cwd = repo_root.to_path_buf();

        if enforce_fmt {
            let fmt_step = Step::bash(
                "Cargo Format Gate",
                "cargo fmt --all -- --check",
                cwd.clone(),
            )
            .with_timeout(Duration::from_secs(60));
            pipeline = pipeline.add_step(fmt_step);
        }

        let check_step = Step::bash("Cargo Check Gate", "cargo check --workspace", cwd.clone())
            .with_timeout(Duration::from_secs(180));
        pipeline = pipeline.add_step(check_step);

        if enforce_clippy {
            let clippy_step = Step::bash(
                "Clippy Lint Gate",
                "cargo clippy --workspace --all-targets --all-features -- -D warnings",
                cwd.clone(),
            )
            .with_timeout(Duration::from_secs(300));
            pipeline = pipeline.add_step(clippy_step);
        }

        if enforce_test {
            let test_step = Step::bash("Unit Test Gate", "cargo test --workspace", cwd.clone())
                .with_timeout(Duration::from_secs(300));
            pipeline = pipeline.add_step(test_step);
        }

        pipeline
    }

    /// Comprehensive Systems Health & Architectural Parity Pipeline
    /// Translates the 4-phase SystemsHealthAuditor directives into actionable execution stages
    pub fn systems_health_pipeline(repo_root: &Path) -> Self {
        let cwd = repo_root.to_path_buf();
        Self::new("Comprehensive Systems Health Pipeline")
            .add_step(
                Step::bash(
                    "Phase 1: Format Verification",
                    "cargo fmt --all -- --check",
                    cwd.clone(),
                )
                .with_timeout(Duration::from_secs(60)),
            )
            .add_step(
                Step::bash(
                    "Phase 2: Compilation & AST Check",
                    "cargo check --workspace",
                    cwd.clone(),
                )
                .with_timeout(Duration::from_secs(240)),
            )
            .add_step(
                Step::bash(
                    "Phase 3: Structural Correctness (Clippy Hygiene)",
                    "cargo clippy --workspace --all-targets --all-features -- -D warnings",
                    cwd.clone(),
                )
                .with_timeout(Duration::from_secs(300)),
            )
            .add_step(
                Step::bash(
                    "Phase 4: Concurrency & State Invariant Tests",
                    "cargo test --workspace",
                    cwd.clone(),
                )
                .with_timeout(Duration::from_secs(360)),
            )
            .add_step(
                Step::bash(
                    "Phase 5: Supply Chain & Duplicate Sweep",
                    "cargo tree -d",
                    cwd.clone(),
                )
                .with_timeout(Duration::from_secs(120)),
            )
            .add_step(
                Step::bash(
                    "Phase 6: Release Build Verification",
                    "cargo build --release -p a_run",
                    cwd,
                )
                .with_timeout(Duration::from_secs(480)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systems_health_pipeline_construction() {
        let repo = Path::new(".");
        let pipeline = RecipePipeline::systems_health_pipeline(repo);
        assert_eq!(pipeline.steps.len(), 6);
        assert_eq!(pipeline.steps[0].name, "Phase 1: Format Verification");
        assert_eq!(
            pipeline.steps[5].name,
            "Phase 6: Release Build Verification"
        );
    }
}
