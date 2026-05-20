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
}
