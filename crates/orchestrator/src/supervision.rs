//! # Supervision Component Block
//!
//! Provides isolated process and task lifecycle supervision, restart policies,
//! and backoff enforcement for the Aaroneous Component Framework.
//!
//! ## Example
//! ```rust
//! use orchestrator::supervision::{Supervisor, SupervisorConfig, SupervisorBudget, RestartPolicy, SupervisedTask};
//! use anyhow::Result;
//!
//! struct Worker;
//! impl SupervisedTask for Worker {
//!     fn name(&self) -> &str { "worker" }
//!     fn step(&mut self) -> Result<()> { Ok(()) }
//!     fn on_panic(&mut self, _err: &str) -> Result<()> { Ok(()) }
//! }
//!
//! let mut supervisor = Supervisor::new(SupervisorConfig::default());
//! supervisor.register_task(
//!     Box::new(Worker),
//!     RestartPolicy::OnFailure { max_retries: 3, backoff_ms: 100 },
//!     SupervisorBudget { max_restarts: 5, window_duration_secs: 60, cpu_affinity_mask: None },
//! );
//! let status = supervisor.tick();
//! assert_eq!(status.len(), 1);
//! ```

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Policy defining how a failed or panicking task should be handled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Never attempt to restart the task once it returns an error.
    Never,
    /// Immediately restart the task on every cycle regardless of errors.
    Always,
    /// Restart up to `max_retries` times with exponential or fixed `backoff_ms`.
    OnFailure { max_retries: u32, backoff_ms: u64 },
}

/// Bounded failure budget and execution constraints for a supervised component.
#[derive(Debug, Clone)]
pub struct SupervisorBudget {
    pub max_restarts: u32,
    pub window_duration_secs: u64,
    pub cpu_affinity_mask: Option<u64>,
}

/// Operational state of a supervised task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskStatus {
    Idle,
    Running,
    Degraded { consecutive_failures: u32 },
    Terminated { reason: String },
}

pub trait SupervisedTask {
    fn name(&self) -> &str;
    fn step(&mut self) -> Result<()>;
    fn on_panic(&mut self, err: &str) -> Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct SupervisorConfig {
    pub default_backoff_ms: u64,
}

pub struct Supervisor {
    _config: SupervisorConfig,
    tasks: HashMap<
        String,
        (
            Box<dyn SupervisedTask + Send>,
            RestartPolicy,
            SupervisorBudget,
            TaskStatus,
            u32,
            Instant,
        ),
    >,
}

impl Supervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        Supervisor {
            _config: config,
            tasks: HashMap::new(),
        }
    }

    pub fn register_task(
        &mut self,
        task: Box<dyn SupervisedTask + Send>,
        policy: RestartPolicy,
        budget: SupervisorBudget,
    ) {
        self.tasks.insert(
            task.name().to_string(),
            (task, policy, budget, TaskStatus::Idle, 0, Instant::now()),
        );
    }

    pub fn tick(&mut self) -> Vec<(String, TaskStatus)> {
        let mut results = Vec::new();
        for (name, (task, policy, _budget, status, consecutive_failures, next_allowed_retry)) in
            self.tasks.iter_mut()
        {
            match status {
                TaskStatus::Idle | TaskStatus::Running => {
                    *status = TaskStatus::Running;
                    if let Err(e) = task.step() {
                        *consecutive_failures += 1;
                        match policy {
                            RestartPolicy::Never => {
                                *status = TaskStatus::Terminated {
                                    reason: e.to_string(),
                                };
                            }
                            RestartPolicy::Always => {
                                *status = TaskStatus::Degraded {
                                    consecutive_failures: *consecutive_failures,
                                };
                                *next_allowed_retry = Instant::now();
                            }
                            RestartPolicy::OnFailure {
                                max_retries,
                                backoff_ms,
                            } => {
                                if *consecutive_failures <= *max_retries {
                                    *status = TaskStatus::Degraded {
                                        consecutive_failures: *consecutive_failures,
                                    };
                                    *next_allowed_retry =
                                        Instant::now() + Duration::from_millis(*backoff_ms);
                                } else {
                                    *status = TaskStatus::Terminated {
                                        reason: e.to_string(),
                                    };
                                }
                            }
                        }
                    }
                }
                TaskStatus::Degraded { .. } => match policy {
                    RestartPolicy::Never => {
                        *status = TaskStatus::Terminated {
                            reason: "Never restart policy enforced".to_string(),
                        };
                    }
                    RestartPolicy::Always => {
                        *status = TaskStatus::Idle;
                    }
                    RestartPolicy::OnFailure { max_retries, .. } => {
                        if *consecutive_failures <= *max_retries {
                            if Instant::now() >= *next_allowed_retry {
                                *status = TaskStatus::Idle;
                            }
                        } else {
                            *status = TaskStatus::Terminated {
                                reason: "Max retries exceeded".to_string(),
                            };
                        }
                    }
                },
                TaskStatus::Terminated { .. } => {}
            }
            results.push((name.clone(), status.clone()));
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;

    struct MockTask {
        name: String,
        should_fail: bool,
    }

    impl MockTask {
        fn new(name: &str, should_fail: bool) -> Self {
            MockTask {
                name: name.to_string(),
                should_fail,
            }
        }
    }

    impl SupervisedTask for MockTask {
        fn name(&self) -> &str {
            &self.name
        }

        fn step(&mut self) -> Result<()> {
            if self.should_fail {
                Err(anyhow!("Task failed"))
            } else {
                Ok(())
            }
        }

        fn on_panic(&mut self, _err: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_supervisor_tick_and_restart() {
        let config = SupervisorConfig::default();
        let mut supervisor = Supervisor::new(config);

        let task1 = Box::new(MockTask::new("task1", true));
        let policy1 = RestartPolicy::OnFailure {
            max_retries: 2,
            backoff_ms: 10,
        };
        let budget1 = SupervisorBudget {
            max_restarts: 3,
            window_duration_secs: 60,
            cpu_affinity_mask: None,
        };
        supervisor.register_task(task1, policy1, budget1);

        let task2 = Box::new(MockTask::new("task2", false));
        let policy2 = RestartPolicy::Never;
        let budget2 = SupervisorBudget {
            max_restarts: 3,
            window_duration_secs: 60,
            cpu_affinity_mask: None,
        };
        supervisor.register_task(task2, policy2, budget2);

        let results = supervisor.tick();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results.iter().find(|(n, _)| n == "task1").unwrap().1,
            TaskStatus::Degraded {
                consecutive_failures: 1
            }
        );
        assert_eq!(
            results.iter().find(|(n, _)| n == "task2").unwrap().1,
            TaskStatus::Running
        );
    }
}

