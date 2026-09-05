// dev/tools/afc/src/engine.rs
use crate::config::FlightConfig;
use crate::delivery::DeliveryEngine;
use crate::gatekeeper::Gatekeeper;
use crate::git::GitEngine;
use crate::hardware::HardwareMonitor;
use crate::llm::LlmOrchestrator;
use crate::queue::QueueManager;
use crate::state::{FlightState, StateMachine};
use anyhow::Result;
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
        let repo_root = config.resolve_repo_root();
        Ok(Self { config, repo_root })
    }

    pub async fn run(&self) -> Result<()> {
        info!("==========================================================");
        info!("        AARONEOUS AUTONOMOUS FLIGHT CONTROLLER (AFC)      ");
        info!("              [Out-of-Tree Sovereign Hypervisor]          ");
        info!("==========================================================");
        info!("Target Repository: {:?}", self.repo_root);

        let mut state_machine = StateMachine::new();

        let logs_dir = self
            .repo_root
            .join("dev")
            .join("docs")
            .join("audits")
            .join("logs");
        if !logs_dir.exists() {
            fs::create_dir_all(&logs_dir).await?;
        }

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

            HardwareMonitor::check_gpu_thermals(self.config.max_gpu_temp).await?;

            // ----------------------------------------------------
            // PHASE 1: PLAN
            // ----------------------------------------------------
            if self.config.phase_plan {
                info!("[PHASE 1: PLAN] Running Frontier Architect...");
                state_machine.transition_to(
                    FlightState::Planning {
                        spec_focus: "Frontier Architect Roadmap".into(),
                    },
                    format!("Cycle {cycle}: Phase 1 Plan"),
                )?;

                if let Err(e) =
                    LlmOrchestrator::run_plan(&self.repo_root, self.config.watchdog_timeout_secs)
                        .await
                {
                    warn!("Phase 1 Plan encountered issue: {e}");
                }
                sleep(Duration::from_secs(3)).await;
            }

            // ----------------------------------------------------
            // PHASE 2: AUDIT
            // ----------------------------------------------------
            if self.config.phase_audit {
                info!("[PHASE 2: AUDIT] Running Safety & Debt Audit...");
                state_machine.transition_to(
                    FlightState::Auditing {
                        category: "Holistic Architectural & Resilience Audit".into(),
                    },
                    format!("Cycle {cycle}: Phase 2 Audit"),
                )?;

                if self.config.audit_health {
                    let _ = LlmOrchestrator::run_specialized_audit(
                        &self.repo_root,
                        "audit-health",
                        None,
                        self.config.watchdog_timeout_secs,
                    )
                    .await;
                }
                if self.config.audit_resilience {
                    let _ = LlmOrchestrator::run_specialized_audit(
                        &self.repo_root,
                        "audit-resilience",
                        None,
                        self.config.watchdog_timeout_secs,
                    )
                    .await;
                }
                if let Err(e) =
                    LlmOrchestrator::run_audit(&self.repo_root, self.config.watchdog_timeout_secs)
                        .await
                {
                    warn!("Phase 2 Audit encountered issue: {e}");
                }
                sleep(Duration::from_secs(3)).await;
            }

            // ----------------------------------------------------
            // PHASE 3: FIX
            // ----------------------------------------------------
            if self.config.phase_fix {
                info!("[PHASE 3: FIX] Inspecting queue for remediation...");
                let pending_tasks = QueueManager::find_pending_tasks(&queue_path).await?;
                if pending_tasks.is_empty() {
                    info!("No pending defects in queue. Skipping Fix phase.");
                } else {
                    let tasks_to_run = pending_tasks.iter().take(3);
                    for (idx, task_title) in tasks_to_run.enumerate() {
                        let subtask_num = idx + 1;
                        info!("Cycle {cycle} - Remediation subtask {subtask_num}: '{task_title}'");

                        state_machine.transition_to(
                            FlightState::IsolatedRemediation {
                                task_id: format!("cycle_{cycle}_task_{subtask_num}"),
                                target_file: PathBuf::new(),
                                target_lines: (0, 0),
                                defect_description: task_title.clone(),
                                compiler_feedback: None,
                            },
                            format!("Cycle {cycle}: Task {subtask_num} remediation"),
                        )?;

                        let subtask_log =
                            logs_dir.join(format!("subtask_{cycle}_{subtask_num}.log"));

                        let endpoint_status =
                            crate::model_probe::ModelProbe::check_endpoint(&self.repo_root).await;

                        let fix_result = if endpoint_status.is_connected() {
                            info!("Cycle {cycle} - Engaging Sovereign REPL loop with local model for '{task_title}'");
                            match self
                                .run_sovereign_repl(task_title, 8, &mut state_machine)
                                .await
                            {
                                Ok(summary) => {
                                    if summary.completed {
                                        info!(
                                            "Sovereign REPL successfully resolved '{task_title}': {}",
                                            summary.outcome_summary
                                        );
                                        Ok(())
                                    } else {
                                        warn!("Sovereign REPL ended without completion. Falling back to CLI agent...");
                                        LlmOrchestrator::run_fix(
                                            &self.repo_root,
                                            task_title,
                                            self.config.watchdog_timeout_secs,
                                            &subtask_log,
                                        )
                                        .await
                                        .map(|_| ())
                                    }
                                }
                                Err(e) => {
                                    warn!("Sovereign REPL loop error: {e}. Falling back to CLI agent...");
                                    LlmOrchestrator::run_fix(
                                        &self.repo_root,
                                        task_title,
                                        self.config.watchdog_timeout_secs,
                                        &subtask_log,
                                    )
                                    .await
                                    .map(|_| ())
                                }
                            }
                        } else {
                            LlmOrchestrator::run_fix(
                                &self.repo_root,
                                task_title,
                                self.config.watchdog_timeout_secs,
                                &subtask_log,
                            )
                            .await
                            .map(|_| ())
                        };

                        if let Err(e) = fix_result {
                            error!("Fix agent watchdog timeout or execution failure on '{task_title}': {e}");
                            if self.config.auto_rollback {
                                GitEngine::rollback_working_tree(&self.repo_root).await?;
                            }
                            continue;
                        }

                        // Validation Gates
                        state_machine.transition_to(
                            FlightState::VerificationGate {
                                modified_files: Vec::new(),
                            },
                            format!("Cycle {cycle}: Task {subtask_num} validation gate"),
                        )?;

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
            }

            // ----------------------------------------------------
            // PHASE 4: SWEEP
            // ----------------------------------------------------
            if self.config.phase_sweep {
                let swept = QueueManager::sweep_completed_tasks(
                    &queue_path,
                    &repair_log_path,
                    &changelog_path,
                )
                .await?;
                if swept > 0 {
                    info!("Phase 4 Sweep: Cleaned up {swept} completed item(s).");
                }
            }

            // ----------------------------------------------------
            // PHASE 5: VERIFICATION
            // ----------------------------------------------------
            if self.config.phase_verify {
                info!("[PHASE 5: VERIFY] Running Workspace Gatekeeper...");
                state_machine.transition_to(
                    FlightState::VerificationGate {
                        modified_files: Vec::new(),
                    },
                    format!("Cycle {cycle}: Phase 5 Verification"),
                )?;

                if self.config.enforce_format {
                    let _ = Gatekeeper::format_workspace(&self.repo_root).await;
                }

                let pipeline_report = if self.config.audit_health {
                    info!("Running Comprehensive Systems Health Pipeline (6 stages)...");
                    Gatekeeper::run_systems_health_pipeline(&self.repo_root).await?
                } else {
                    Gatekeeper::run_verification_pipeline(
                        &self.repo_root,
                        self.config.clippy_gate,
                        self.config.run_tests,
                        false, // Format already handled above
                    )
                    .await?
                };

                if !pipeline_report.passed {
                    if let Some(ref err) = pipeline_report.failure_summary {
                        error!("Workspace verification pipeline failed at cycle {cycle}:\n{err}");
                    } else {
                        error!("Workspace verification pipeline failed at cycle {cycle}");
                    }
                    if self.config.auto_cycles == 1 {
                        break;
                    }
                } else {
                    info!("All workspace verification gates passed cleanly!");

                    if self.config.run_security {
                        let _ = Gatekeeper::audit_security(&self.repo_root).await;
                    }

                    // ----------------------------------------------------
                    // PHASE 6: ATOMIC GIT COMMIT
                    // ----------------------------------------------------
                    if self.config.phase_commit && GitEngine::is_dirty(&self.repo_root).await? {
                        let commit_msg =
                            format!("chore(flight): verified autonomous cycle {cycle} [skip ci]");
                        state_machine.transition_to(
                            FlightState::CommitLedger {
                                commit_message: commit_msg.clone(),
                            },
                            format!("Cycle {cycle}: Phase 6 Commit"),
                        )?;
                        GitEngine::atomic_commit(&self.repo_root, &commit_msg).await?;
                    }
                }
            }

            HardwareMonitor::check_build_cache_size(&self.repo_root, 30).await?;

            if cycle < self.config.auto_cycles {
                info!("Cycle {cycle} complete. Cooling down for 5 seconds...");
                sleep(Duration::from_secs(5)).await;
            }
        }

        // ----------------------------------------------------
        // PHASE 7: DELIVERY
        // ----------------------------------------------------
        if self.config.phase_deliver && self.config.build_artifacts {
            info!("[PHASE 7: DELIVERY] Building release binaries and packaging...");
            if let Ok(()) = Gatekeeper::build_release(&self.repo_root).await {
                DeliveryEngine::package_artifacts(&self.repo_root).await?;
            }
        }

        state_machine.transition_to(
            FlightState::Completed,
            "All autonomous cycles completed successfully",
        )?;

        // Write state transitions log
        let transitions_log = logs_dir.join("state_transitions.json");
        if let Ok(serialized) = serde_json::to_string_pretty(&state_machine.history) {
            let _ = fs::write(&transitions_log, serialized).await;
        }

        info!("==========================================================");
        info!("Autonomous Flight Controller execution complete!");
        info!("==========================================================");

        Ok(())
    }

    /// Run an autonomous Sovereign REPL cycle against the local LLM endpoint
    pub async fn run_sovereign_repl(
        &self,
        task_prompt: &str,
        max_turns: usize,
        state_machine: &mut StateMachine,
    ) -> Result<crate::repl::ReplSummary> {
        let probe = crate::model_probe::ModelProbe::check_endpoint(&self.repo_root).await;
        let (endpoint_url, model_name) = match probe {
            crate::model_probe::ModelEndpointStatus::Connected { ref endpoint, .. } => {
                (endpoint.clone(), probe.resolved_model_id())
            }
            _ => ("http://127.0.0.1:1234".to_string(), "qwen2.5-coder-7b-instruct".to_string()),
        };
        let client = crate::router::TypedRouterClient::new(endpoint_url, 1234, None);
        let repl = crate::repl::SovereignRepl::new(self.repo_root.clone(), client, model_name);
        repl.run_autonomous_cycle(task_prompt, max_turns, state_machine)
            .await
    }
}
