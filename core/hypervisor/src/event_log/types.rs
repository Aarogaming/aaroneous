use serde::{Serialize, Deserialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FederationEvent {
    pub event_id: String,
    pub timestamp: i64,
    pub trace_id: String,
    pub source_repo: String,
    pub source_domain: String,
    pub event_type: String,
    pub operation: String,
    pub payload: HashMap<String, Value>,
    pub consensus_round: Option<u64>,
    pub replicas_acked: Vec<String>,
    pub applied_at: Option<i64>,
}

/// Standard event types for the federation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum EventType {
    Intent,
    Proposal,
    Execution,
    Checkpoint,
    Consensus,
    Error,
    System,
    Mutation,
}

/// Standard operations within the federation.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Operation {
    Create,
    Update,
    Delete,
    Query,
    Execute,
    Validate,
    Reject,
    Accept,
}

/// Event log for recording and querying federation events.
#[derive(Debug, Clone)]
pub struct EventLog {
    events: std::sync::Arc<std::sync::Mutex<Vec<FederationEvent>>>,
    max_events: usize,
}

impl EventLog {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            max_events,
        }
    }

    pub fn record(&self, event: FederationEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
            if events.len() > self.max_events {
                let remove_count = events.len() - self.max_events;
                events.drain(0..remove_count);
            }
        }
    }

    pub async fn append(&self, event: FederationEvent) -> Result<(), ()> {
        self.record(event);
        Ok(())
    }

    pub fn get_recent(&self, count: usize) -> Vec<FederationEvent> {
        if let Ok(events) = self.events.lock() {
            let len = events.len();
            let start = len.saturating_sub(count);
            events[start..].to_vec()
        } else {
            vec![]
        }
    }

    pub fn len(&self) -> usize {
        self.events.lock().map(|e| e.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
