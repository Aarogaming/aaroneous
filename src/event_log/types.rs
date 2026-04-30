use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Unique event identifier
pub type EventId = String;

/// Offset into event log
pub type LogOffset = u64;

/// Federation event - immutable record of all significant actions
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FederationEvent {
    /// Unique event ID (UUID4)
    pub event_id: String,
    /// Unix timestamp (milliseconds)
    pub timestamp: i64,
    /// Distributed trace ID (links to tracing system)
    pub trace_id: String,
    /// Source repository (e.g., "AaroneousAutomationSuite", "Guild")
    pub source_repo: String,
    /// Source domain (e.g., "leadership", "intelligence", "knowledge")
    pub source_domain: String,
    /// Event classification
    pub event_type: EventType,
    /// Operation being performed
    pub operation: Operation,
    /// Event-specific payload
    pub payload: HashMap<String, serde_json::Value>,
    /// Raft consensus round (if mutation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_round: Option<u64>,
    /// Which replicas have acknowledged this event
    pub replicas_acked: Vec<String>,
    /// When this event was applied to state machine
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<i64>,
}

impl FederationEvent {
    /// Create new event
    pub fn new(
        trace_id: impl Into<String>,
        source_repo: impl Into<String>,
        source_domain: impl Into<String>,
        event_type: EventType,
        operation: Operation,
    ) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            trace_id: trace_id.into(),
            source_repo: source_repo.into(),
            source_domain: source_domain.into(),
            event_type,
            operation,
            payload: HashMap::new(),
            consensus_round: None,
            replicas_acked: Vec::new(),
            applied_at: None,
        }
    }

    /// Add payload data
    pub fn with_payload(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.payload.insert(key.into(), value);
        self
    }

    /// Set consensus round
    pub fn with_consensus_round(mut self, round: u64) -> Self {
        self.consensus_round = Some(round);
        self
    }

    /// Mark as applied to state machine
    pub fn mark_applied(mut self) -> Self {
        self.applied_at = Some(chrono::Utc::now().timestamp_millis());
        self
    }

    /// Check if this is a mutation that requires consensus
    pub fn requires_consensus(&self) -> bool {
        matches!(self.event_type, EventType::Mutation)
    }
}

/// Event type classification
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventType {
    /// Federation startup or node join
    Boot,
    /// Plugin loading/initialization
    PluginLoad,
    /// Plugin execution result
    PluginExec,
    /// State machine mutation (requires consensus)
    Mutation,
    /// Periodic health status
    HealthCheck,
    /// Critic loop validation result
    Validation,
    /// Autonomous repair action
    Repair,
    /// Knowledge extraction and compression
    Distillation,
    /// Cascade or error event
    Failure,
    /// Recovery operation
    Recovery,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Boot => write!(f, "BOOT"),
            EventType::PluginLoad => write!(f, "PLUGIN_LOAD"),
            EventType::PluginExec => write!(f, "PLUGIN_EXEC"),
            EventType::Mutation => write!(f, "MUTATION"),
            EventType::HealthCheck => write!(f, "HEALTH_CHECK"),
            EventType::Validation => write!(f, "VALIDATION"),
            EventType::Repair => write!(f, "REPAIR"),
            EventType::Distillation => write!(f, "DISTILLATION"),
            EventType::Failure => write!(f, "FAILURE"),
            EventType::Recovery => write!(f, "RECOVERY"),
        }
    }
}

/// Operation being performed
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op_type", content = "target")]
pub enum Operation {
    /// Create new resource
    Create(String),
    /// Update existing resource
    Update(String),
    /// Delete resource
    Delete(String),
    /// Query/read resource
    Query(String),
    /// Replicate to peers
    Replicate(String),
    /// Rollback to previous state
    Rollback(String),
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operation::Create(t) => write!(f, "CREATE({})", t),
            Operation::Update(t) => write!(f, "UPDATE({})", t),
            Operation::Delete(t) => write!(f, "DELETE({})", t),
            Operation::Query(t) => write!(f, "QUERY({})", t),
            Operation::Replicate(t) => write!(f, "REPLICATE({})", t),
            Operation::Rollback(t) => write!(f, "ROLLBACK({})", t),
        }
    }
}

/// Event log error types
#[derive(Clone, Debug)]
pub enum EventLogError {
    /// Database error
    DatabaseError(String),
    /// Event not found
    NotFound(String),
    /// Serialization error
    SerializationError(String),
    /// Replication error
    ReplicationError(String),
    /// Invalid offset
    InvalidOffset(String),
    /// Snapshot error
    SnapshotError(String),
    /// IO error
    IoError(String),
    /// Internal error
    InternalError(String),
}

impl fmt::Display for EventLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventLogError::DatabaseError(e) => write!(f, "Database error: {}", e),
            EventLogError::NotFound(e) => write!(f, "Not found: {}", e),
            EventLogError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            EventLogError::ReplicationError(e) => write!(f, "Replication error: {}", e),
            EventLogError::InvalidOffset(e) => write!(f, "Invalid offset: {}", e),
            EventLogError::SnapshotError(e) => write!(f, "Snapshot error: {}", e),
            EventLogError::IoError(e) => write!(f, "IO error: {}", e),
            EventLogError::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for EventLogError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("plugin".to_string()),
        );

        assert_eq!(event.trace_id, "trace-1");
        assert_eq!(event.source_repo, "AAS");
        assert!(event.requires_consensus());
        assert!(event.applied_at.is_none());
    }

    #[test]
    fn test_event_with_payload() {
        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Update("config".to_string()),
        )
        .with_payload("key", serde_json::json!("value"));

        assert_eq!(event.payload.get("key").unwrap(), "value");
    }

    #[test]
    fn test_event_mark_applied() {
        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("test".to_string()),
        )
        .mark_applied();

        assert!(event.applied_at.is_some());
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::Boot.to_string(), "BOOT");
        assert_eq!(EventType::Mutation.to_string(), "MUTATION");
    }

    #[test]
    fn test_operation_display() {
        assert_eq!(
            Operation::Create("test".to_string()).to_string(),
            "CREATE(test)"
        );
    }

    #[test]
    fn test_event_serialization() {
        let event = FederationEvent::new(
            "trace-1",
            "AAS",
            "leadership",
            EventType::Mutation,
            Operation::Create("plugin".to_string()),
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: FederationEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(event, deserialized);
    }
}
