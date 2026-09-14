//! Control-plane task supervision with injected monotonic time. Registration
//! and status collection allocate; this adapter is not a real-time reducer.
//! It retries task steps, not OS processes. Panics are process failures under
//! the workspace's release `panic = "abort"` profile.
#![deny(unsafe_code)]

use anyhow::{Result, bail};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartPolicy {
    Never,
    Always,
    OnFailure { max_retries: u32, backoff_ms: u64 },
}

#[derive(Debug, Clone)]
pub struct SupervisorBudget {
    pub max_restarts: u32,
    pub window_duration_secs: u64,
    pub cpu_affinity_mask: Option<u64>,
}

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
    /// Compatibility callback for a host with an unwind boundary; this adapter
    /// does not claim to recover from process-aborting panics.
    fn on_panic(&mut self, err: &str) -> Result<()>;
    /// A task-specific OS adapter must explicitly support requested affinity.
    fn configure_affinity(&mut self, _mask: u64) -> Result<()> {
        bail!("Task does not provide an affinity adapter")
    }
}

#[derive(Debug, Clone, Default)]
pub struct SupervisorConfig {
    pub default_backoff_ms: u64,
}

struct TaskEntry {
    task: Box<dyn SupervisedTask + Send>,
    policy: RestartPolicy,
    budget: SupervisorBudget,
    status: TaskStatus,
    failures: u32,
    retry_at_ms: u64,
    window_start_ms: u64,
    restarts_in_window: u32,
}

pub struct Supervisor {
    config: SupervisorConfig,
    tasks: BTreeMap<String, TaskEntry>,
    last_tick_ms: u64,
}

impl Supervisor {
    pub fn new(config: SupervisorConfig) -> Self {
        Self {
            config,
            tasks: BTreeMap::new(),
            last_tick_ms: 0,
        }
    }

    pub fn register_task(
        &mut self,
        mut task: Box<dyn SupervisedTask + Send>,
        policy: RestartPolicy,
        budget: SupervisorBudget,
    ) -> Result<()> {
        if self.tasks.contains_key(task.name()) {
            bail!("Task already registered");
        }
        if budget.window_duration_secs == 0 {
            bail!("Restart window must be nonzero");
        }
        if let Some(mask) = budget.cpu_affinity_mask {
            if mask == 0 {
                bail!("Affinity mask must be nonzero");
            }
            task.configure_affinity(mask)?;
        }
        self.tasks.insert(
            task.name().to_owned(),
            TaskEntry {
                task,
                policy,
                budget,
                status: TaskStatus::Idle,
                failures: 0,
                retry_at_ms: self.last_tick_ms,
                window_start_ms: self.last_tick_ms,
                restarts_in_window: 0,
            },
        );
        Ok(())
    }

    /// `now_ms` is supplied by acquisition. Equal inputs and task outcomes give
    /// equal scheduling decisions. Backwards time is rejected without mutation.
    pub fn tick(&mut self, now_ms: u64) -> Result<Vec<(String, TaskStatus)>> {
        if now_ms < self.last_tick_ms {
            bail!("Supervisor time moved backwards");
        }
        self.last_tick_ms = now_ms;
        for entry in self.tasks.values_mut() {
            if matches!(entry.status, TaskStatus::Terminated { .. }) {
                continue;
            }
            let window_ms = entry.budget.window_duration_secs.saturating_mul(1000);
            if now_ms.saturating_sub(entry.window_start_ms) >= window_ms {
                entry.window_start_ms = now_ms;
                entry.restarts_in_window = 0;
            }
            if matches!(entry.status, TaskStatus::Degraded { .. }) {
                if now_ms < entry.retry_at_ms
                    || entry.restarts_in_window >= entry.budget.max_restarts
                {
                    continue;
                }
                entry.restarts_in_window += 1;
            }
            match entry.task.step() {
                Ok(()) => {
                    entry.failures = 0;
                    entry.status = TaskStatus::Running;
                }
                Err(error) => {
                    entry.failures = entry.failures.saturating_add(1);
                    let backoff = match entry.policy {
                        RestartPolicy::Never => None,
                        RestartPolicy::Always => Some(self.config.default_backoff_ms),
                        RestartPolicy::OnFailure {
                            max_retries,
                            backoff_ms,
                        } if entry.failures <= max_retries => Some(backoff_ms),
                        RestartPolicy::OnFailure { .. } => None,
                    };
                    if let Some(backoff) = backoff {
                        entry.status = TaskStatus::Degraded {
                            consecutive_failures: entry.failures,
                        };
                        entry.retry_at_ms = now_ms.saturating_add(backoff);
                    } else {
                        entry.status = TaskStatus::Terminated {
                            reason: error.to_string(),
                        };
                    }
                }
            }
        }
        Ok(self
            .tasks
            .iter()
            .map(|(name, entry)| (name.clone(), entry.status.clone()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Worker {
        outcomes: std::collections::VecDeque<bool>,
    }
    impl SupervisedTask for Worker {
        fn name(&self) -> &str {
            "worker"
        }
        fn step(&mut self) -> Result<()> {
            if self.outcomes.pop_front().unwrap_or(false) {
                Ok(())
            } else {
                bail!("failed")
            }
        }
        fn on_panic(&mut self, _: &str) -> Result<()> {
            Ok(())
        }
    }
    fn budget() -> SupervisorBudget {
        SupervisorBudget {
            max_restarts: 1,
            window_duration_secs: 1,
            cpu_affinity_mask: None,
        }
    }
    #[test]
    fn backoff_budget_and_window_are_enforced() -> Result<()> {
        let mut supervisor = Supervisor::new(SupervisorConfig {
            default_backoff_ms: 10,
        });
        supervisor.register_task(
            Box::new(Worker {
                outcomes: [false, false, true].into(),
            }),
            RestartPolicy::Always,
            budget(),
        )?;
        assert_eq!(
            supervisor.tick(0)?[0].1,
            TaskStatus::Degraded {
                consecutive_failures: 1
            }
        );
        assert_eq!(
            supervisor.tick(9)?[0].1,
            TaskStatus::Degraded {
                consecutive_failures: 1
            }
        );
        assert_eq!(
            supervisor.tick(10)?[0].1,
            TaskStatus::Degraded {
                consecutive_failures: 2
            }
        );
        assert_eq!(
            supervisor.tick(999)?[0].1,
            TaskStatus::Degraded {
                consecutive_failures: 2
            }
        );
        assert_eq!(supervisor.tick(1000)?[0].1, TaskStatus::Running);
        assert!(supervisor.tick(999).is_err());
        Ok(())
    }
    #[test]
    fn successful_step_resets_consecutive_failure_limit() -> Result<()> {
        let mut supervisor = Supervisor::new(SupervisorConfig::default());
        let mut limits = budget();
        limits.max_restarts = 10;
        supervisor.register_task(
            Box::new(Worker {
                outcomes: [false, true, false, false].into(),
            }),
            RestartPolicy::OnFailure {
                max_retries: 1,
                backoff_ms: 0,
            },
            limits,
        )?;
        supervisor.tick(0)?;
        assert_eq!(supervisor.tick(1)?[0].1, TaskStatus::Running);
        assert_eq!(
            supervisor.tick(2)?[0].1,
            TaskStatus::Degraded {
                consecutive_failures: 1
            }
        );
        assert!(matches!(
            supervisor.tick(3)?[0].1,
            TaskStatus::Terminated { .. }
        ));
        Ok(())
    }
    #[test]
    fn unsupported_affinity_is_not_silently_ignored() {
        let mut supervisor = Supervisor::new(SupervisorConfig::default());
        let mut limits = budget();
        limits.cpu_affinity_mask = Some(1);
        assert!(
            supervisor
                .register_task(
                    Box::new(Worker {
                        outcomes: [].into()
                    }),
                    RestartPolicy::Never,
                    limits
                )
                .is_err()
        );
    }
}
