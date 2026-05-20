use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub cron_expression: Option<String>,
    pub interval_secs: Option<u64>,
    pub intent_content: String,
    pub last_run_ms: u64,
    pub status: String, // "Scheduled", "Pending", "Running", "Cancelled"
}

pub struct AutonomousScheduler {
    pub tasks: HashMap<String, ScheduledTask>,
}

impl AutonomousScheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
        }
    }

    pub fn schedule_task(&mut self, task: ScheduledTask) {
        self.tasks.insert(task.id.clone(), task);
    }

    pub fn remove_task(&mut self, id: &str) {
        self.tasks.remove(id);
    }
}
