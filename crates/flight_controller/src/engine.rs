// crates/flight_controller/src/engine.rs
use crate::config::FlightConfig;
use crate::delivery::DeliveryEngine;
use crate::gatekeeper::Gatekeeper;
use crate::git::GitEngine;
use crate::hardware::HardwareMonitor;
use crate::llm::LlmOrchestrator;
use crate::queue::QueueManager;
use anyhow::{Context, Result};
use chrono::Local;
use std::path::PathBuf;
use tokio::fs;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

pub struct FlightEngine {
    config: FlightConfig,
    repo_root: PathBuf,
}

impl FlightEngine {
    pub fn new(config: FlightConfig) -> Result<Self> {
        let repo_root = match &config.repo_root {
            Some(path) => path.clone(),
            None => std::env::current_dir().context("Failed to get current working directory")?,
        };

        Ok(Self { config, repo_root })
    }

    pub async fn run(&self) -> Result<()> {
        info!("==========================================================");
        info!("        AARONEOUS AUTONOMOUS FLIGHT CONTROLLER (AFC)      ");
        info!("              [Rust-Native Sovereign CI/CD Engine]        ");
        info!("==========================================================");

        // Ensure logs directory exists
        let logs_dir = self
            .repo_root
            .join("dev")
            .join("docs")
            .join("audits")
            .join("logs");
        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).await?;
        }

        // Branch safety: protect main/master
        let current_branch = GitEngine::current_branch(&self.repo_root).await?;
        if self.config.auto_branch && (current_branch == "main" || current_branch == "master") {
            let date_tag = Local::now().format("%Y%m%d-%H%M").to_string();
            let flight_branch = format!("flight/auto-{date_tag}");
            warn!("Branch safety: Active on '{current_branch}'. Branching to '{flight_branch}'");
            GitEngine::checkout_new_branch(&self.repo_root, &flight_branch).await?;
        } else {
            info!("Active working branch: '{current_branch}'");
        }

        let queue_path = self
            .repo_root
            .join("dev")
            .join("docs")
            .join("audits")
            .join("active")
            .join("ACTIVE_AUDIT_QUEUE.md");
        let repair_log_path = self
            .repo_root
            .join("dev")
            .join("docs")
            .join("audits")
            .join("REPAIR_LOG.md");
        let changelog_path = self.repo_root.join("CHANGELOG.md");

        for cycle in 1..=self.config.auto_cycles {
            info!("==========================================================");
            info!(
                ">>> CYCLE {cycle} OF {} - INITIATING ROTATION <<<",
                self.config.auto_cycles
            );
            info!("==========================================================");

            // Hardware thermal check
            HardwareMonitor::check_gpu_thermals(self.config.max_gpu_temp).await?;

            // ----------------------------------------------------
            // PHASE 1: PLAN
            // ----------------------------------------------------
            info!("[PHASE 1: PLAN] Running Frontier Architect...");
            if let Err(e) =
                LlmOrchestrator::run_plan(&self.repo_root, self.config.watchdog_timeout_secs).await
            {
                warn!("Phase 1 Plan encountered issue: {e}");
            }
            sleep(Duration::from_secs(3)).await;

            // ----------------------------------------------------
            // PHASE 2: AUDIT
            // ----------------------------------------------------
            info!("[PHASE 2: AUDIT] Running Safety & Debt Audit...");
            if let Err(e) =
                LlmOrchestrator::run_audit(&self.repo_root, self.config.watchdog_timeout_secs).await
            {
                warn!("Phase 2 Audit encountered issue: {e}");
            }
            sleep(Duration::from_secs(3)).await;

            // ----------------------------------------------------
            // PHASE 3: FIX
            // ----------------------------------------------------
            info!("[PHASE 3: FIX] Inspecting queue for remediation...");
            let pending_tasks = QueueManager::find_pending_tasks(&queue_path).await?;
            if pending_tasks.is_empty() {
                info!("No pending defects in queue. Skipping Fix phase.");
            } else {
                let tasks_to_run = pending_tasks.iter().take(3);
                for (idx, task_title) in tasks_to_run.enumerate() {
                    let subtask_num = idx + 1;
                    info!("Cycle {cycle} - Remediation subtask {subtask_num}: '{task_title}'");

                    let subtask_log = logs_dir.join(format!("subtask_{cycle}_{subtask_num}.log"));
                    let fix_result = LlmOrchestrator::run_fix(
                        &self.repo_root,
                        task_title,
                        self.config.watchdog_timeout_secs,
                        &subtask_log,
                    )
                    .await;

                    if let Err(e) = fix_result {
                        error!("Fix agent watchdog timeout or execution failure on '{task_title}': {e}");
                        if self.config.auto_rollback {
                            GitEngine::rollback_working_tree(&self.repo_root).await?;
                        }
                        continue;
                    }

                    // Validation Gates
                    let check_res = Gatekeeper::check_workspace(&self.repo_root).await;
                    let mut validation_failed = check_res.is_err();

                    if !validation_failed && self.config.run_tests {
                        if let Err(e) = Gatekeeper::test_workspace(&self.repo_root).await {
                            error!("Unit tests failed after fix on '{task_title}': {e}");
                            validation_failed = true;
                        }
                    }

                    if validation_failed {
                        warn!("Validation gate rejected modifications for '{task_title}'");
                        if self.config.auto_rollback {
                            GitEngine::rollback_working_tree(&self.repo_root).await?;
                        }
                    } else {
                        if self.config.enforce_format {
                            let _ = Gatekeeper::format_workspace(&self.repo_root).await;
                        }

                        if GitEngine::is_dirty(&self.repo_root).await? {
                            QueueManager::mark_task_completed(&queue_path, task_title).await?;
                            info!("Validated and auto-resolved task: '{task_title}'");
                        }
                    }

                    sleep(Duration::from_secs(2)).await;
                }
            }

            // ----------------------------------------------------
            // PHASE 4: SWEEP
            // ----------------------------------------------------
            let swept =
                QueueManager::sweep_completed_tasks(&queue_path, &repair_log_path, &changelog_path)
                    .await?;
            if swept > 0 {
                info!("Phase 4 Sweep: Cleaned up {swept} completed item(s).");
            }

            // ----------------------------------------------------
            // PHASE 5: VERIFICATION
            // ----------------------------------------------------
            info!("[PHASE 5: VERIFY] Running Workspace Gatekeeper...");
            if self.config.enforce_format {
                let _ = Gatekeeper::format_workspace(&self.repo_root).await;
            }

            if let Err(e) = Gatekeeper::check_workspace(&self.repo_root).await {
                error!("Workspace compilation failed at end of cycle {cycle}: {e}");
                if self.config.auto_cycles == 1 {
                    break;
                }
            } else {
                info!("Workspace compilation clean!");

                if self.config.clippy_gate {
                    let _ = Gatekeeper::inspect_clippy(&self.repo_root).await;
                }

                if self.config.run_tests {
                    if let Err(e) = Gatekeeper::test_workspace(&self.repo_root).await {
                        error!("Workspace tests failed: {e}");
                    } else {
                        info!("All workspace unit tests passed!");
                    }
                }

                if self.config.run_security {
                    let _ = Gatekeeper::audit_security(&self.repo_root).await;
                }

                // ----------------------------------------------------
                // PHASE 6: ATOMIC GIT COMMIT
                // ----------------------------------------------------
                if GitEngine::is_dirty(&self.repo_root).await? {
                    let commit_msg =
                        format!("chore(flight): verified autonomous cycle {cycle} [skip ci]");
                    GitEngine::atomic_commit(&self.repo_root, &commit_msg).await?;
                }
            }

            // Check storage footprint
            HardwareMonitor::check_build_cache_size(&self.repo_root, 30).await?;

            if cycle < self.config.auto_cycles {
                info!("Cycle {cycle} complete. Cooling down for 5 seconds...");
                sleep(Duration::from_secs(5)).await;
            }
        }

        // ----------------------------------------------------
        // PHASE 7: DELIVERY
        // ----------------------------------------------------
        if self.config.build_artifacts {
            info!("[PHASE 7: DELIVERY] Building release binaries and packaging...");
            if let Ok(()) = Gatekeeper::build_release(&self.repo_root).await {
                DeliveryEngine::package_artifacts(&self.repo_root).await?;
            }
        }

        info!("==========================================================");
        info!("🎉 Autonomous Flight Controller execution complete!");
        info!("==========================================================");

        Ok(())
    }
}
