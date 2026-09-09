//! Legacy Monolith — messy, tightly coupled file for translation pipeline testing.
//!
//! This file is a synthetic control group that exercises AST extraction and
//! the LLM translation fallback routines. It contains mixed concerns, legacy
//! patterns, and enough structure to stress-test complexity estimation.

use std::collections::HashMap;

pub struct Config {
    pub name: String,
    pub enabled: bool,
    pub max_retries: u32,
}

pub enum State {
    Idle,
    Running { progress: f64 },
    Failed(String),
}

pub struct Worker {
    pub id: u32,
    pub config: Config,
    pub state: State,
    pub history: Vec<String>,
}

pub fn create_worker(id: u32, name: &str) -> Worker {
    Worker {
        id,
        config: Config {
            name: name.to_string(),
            enabled: true,
            max_retries: 3,
        },
        state: State::Idle,
        history: Vec::new(),
    }
}

pub fn start_worker(worker: &mut Worker) {
    println!("Starting worker {}", worker.id);
    worker.state = State::Running { progress: 0.0 };
    worker.history.push(format!("started at {}", chrono_now()));
}

pub fn process_batch(worker: &mut Worker, items: &[String]) -> Vec<String> {
    let mut results = Vec::new();
    for item in items {
        let result = format!("processed:{}", item);
        results.push(result);
    }
    worker.history.push(format!("processed {} items", items.len()));
    results
}

pub fn record_failure(worker: &mut Worker, reason: &str) {
    worker.state = State::Failed(reason.to_string());
    println!("Worker {} failed: {}", worker.id, reason);
    worker.history.push(format!("failed: {reason}"));
}

pub fn get_status(worker: &Worker) -> String {
    match &worker.state {
        State::Idle => "idle".to_string(),
        State::Running { progress } => format!("running@{progress:.1}%"),
        State::Failed(msg) => format!("failed: {msg}"),
    }
}

pub fn build_report(workers: &[Worker]) -> String {
    let mut lines = Vec::new();
    for w in workers {
        lines.push(format!("worker-{}: {}", w.id, get_status(w)));
    }
    lines.join("\n")
}

pub fn aggregate_metrics(workers: &[Worker]) -> HashMap<String, f64> {
    let mut metrics = HashMap::new();
    let total = workers.len() as f64;
    let running = workers.iter().filter(|w| matches!(w.state, State::Running { .. })).count() as f64;
    let failed = workers.iter().filter(|w| matches!(w.state, State::Failed(_))).count() as f64;
    metrics.insert("total".into(), total);
    metrics.insert("running".into(), running);
    metrics.insert("failed".into(), failed);
    metrics
}

fn chrono_now() -> String {
    "2026-01-01T00:00:00Z".to_string()
}
