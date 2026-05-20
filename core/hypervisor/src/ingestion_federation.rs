// Phase 8: NATS Federation Integration for Data Ingestion
// Real-time broadcasting of ingestion events, quality metrics, and specialist updates
// across the federation to enable cross-hive collaboration and transparency

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::skill_system::SkillType;
use std::collections::HashMap;

/// Root topic for all ingestion events
pub const INGESTION_TOPIC_ROOT: &str = "federation.ingestion";

/// Full topic hierarchy for ingestion events
pub struct IngestionTopics;

impl IngestionTopics {
    /// Main ingestion events topic
    pub fn events() -> &'static str {
        "federation.ingestion.events"
    }

    /// Per-specialist ingestion events
    pub fn events_for_specialist(specialist_id: &str) -> String {
        format!("federation.ingestion.events.{}", specialist_id)
    }

    /// Data classification results
    pub fn classification_results() -> &'static str {
        "federation.ingestion.classification"
    }

    /// Per-domain classification results
    pub fn classification_by_domain(domain: &str) -> String {
        format!("federation.ingestion.classification.{}", domain)
    }

    /// Quality assessments
    pub fn quality_metrics() -> &'static str {
        "federation.ingestion.quality"
    }

    /// Specialist XP awards
    pub fn specialist_updates() -> &'static str {
        "federation.ingestion.specialist_updates"
    }

    /// Specialist updates per ID
    pub fn specialist_updates_for(specialist_id: &str) -> String {
        format!("federation.ingestion.specialist_updates.{}", specialist_id)
    }

    /// System statistics and health
    pub fn system_stats() -> &'static str {
        "federation.ingestion.stats"
    }

    /// Failure notifications
    pub fn failures() -> &'static str {
        "federation.ingestion.failures"
    }

    /// Federation-wide ingestion queries
    pub fn queries() -> &'static str {
        "federation.ingestion.queries"
    }
}

/// Represents an ingestion event published to the federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionEvent {
    pub event_id: String,
    pub data_id: String,
    pub filename: Option<String>,
    pub file_format: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub detected_domains: Vec<String>,
    pub primary_domain: Option<String>,
    pub classification_confidence: f32,
    pub quality_score: f32,
    pub complexity: f32,
    pub timestamp: DateTime<Utc>,
    pub status: IngestionStatus,
}

/// Status of ingestion in the federation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngestionStatus {
    Received,
    Classified,
    Matched,
    Distilled,
    EventsGenerated,
    Published,
    Failed,
}

/// Classification result published to federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub event_id: String,
    pub data_id: String,
    pub domains: HashMap<String, f32>, // domain -> confidence
    pub primary_domain: String,
    pub primary_confidence: f32,
    pub secondary_domains: Vec<(String, f32)>,
    pub structure_detected: StructureInfo,
    pub complexity_score: f32,
    pub timestamp: DateTime<Utc>,
}

/// Information about detected structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructureInfo {
    pub format: String,
    pub is_timeseries: bool,
    pub record_count: Option<usize>,
    pub field_count: usize,
    pub nesting_depth: usize,
}

/// Quality metrics for ingested data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetric {
    pub metric_id: String,
    pub data_id: String,
    pub overall_score: f32,
    pub format_quality: f32,
    pub semantic_quality: f32,
    pub training_value: f32,
    pub assessment_notes: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// Specialist update from ingestion event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialistUpdate {
    pub update_id: String,
    pub specialist_id: String,
    pub xp_gained: u32,
    pub skill_type: String, // RAG, DAG, MCP, API, etc.
    pub quality_multiplier: f32,
    pub difficulty_multiplier: f32,
    pub source_data_id: String,
    pub source_filename: Option<String>,
    pub is_breakthrough: bool,
    pub timestamp: DateTime<Utc>,
}

/// Ingestion system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionStats {
    pub stats_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub files_received: u64,
    pub files_processed: u64,
    pub files_failed: u64,
    pub total_xp_distributed: u32,
    pub average_quality_score: f32,
    pub domains_detected: Vec<String>,
    pub specialist_utilization: HashMap<String, u32>, // specialist_id -> xp_awarded
    pub processing_time_ms_avg: u32,
}

/// Failure event published to federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureEvent {
    pub failure_id: String,
    pub data_id: String,
    pub filename: Option<String>,
    pub failure_reason: String,
    pub stage_failed: String, // ingestion, classification, matching, distillation
    pub error_details: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Query request for ingestion data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionQuery {
    pub query_id: String,
    pub requested_by: String,
    pub query_type: IngestionQueryType,
    pub filters: QueryFilters,
    pub limit: usize,
    pub timestamp: DateTime<Utc>,
}

/// Types of ingestion queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestionQueryType {
    /// Get all events for a specialist
    SpecialistEvents,
    /// Get all events for a domain
    DomainEvents,
    /// Get high-quality training data
    HighQualityData,
    /// Get specialist XP history
    SpecialistXPHistory,
    /// Get system statistics
    SystemStats,
    /// Get recent failures
    RecentFailures,
    /// Custom query
    Custom(String),
}

/// Filters for ingestion queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryFilters {
    pub specialist_id: Option<String>,
    pub domain: Option<String>,
    pub min_quality_score: Option<f32>,
    pub min_confidence: Option<f32>,
    pub time_range: Option<TimeRange>,
    pub file_format: Option<String>,
}

/// Time range for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Query response containing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub request_id: String,
    pub query_type: String,
    pub result_count: usize,
    pub events: Vec<IngestionEvent>,
    pub stats: Option<IngestionStats>,
    pub timestamp: DateTime<Utc>,
    pub response_time_ms: u32,
}

/// Configuration for NATS federation integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub nats_url: String,
    pub enable_publishing: bool,
    pub enable_subscription: bool,
    pub publish_interval_secs: u64,
    pub batch_size: usize,
    pub compression: bool,
    pub quality_threshold_for_publishing: f32,
    pub retain_events_days: u64,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".to_string(),
            enable_publishing: true,
            enable_subscription: true,
            publish_interval_secs: 5,
            batch_size: 10,
            compression: false,
            quality_threshold_for_publishing: 0.5,
            retain_events_days: 7,
        }
    }
}

/// Helper to convert SkillType to string for serialization
pub fn skill_type_to_string(skill_type: &SkillType) -> String {
    format!("{:?}", skill_type)
}

/// Helper to convert string back to SkillType (best-effort)
pub fn string_to_skill_type(s: &str) -> SkillType {
    match s.to_uppercase().as_str() {
        "RAG" => SkillType::RAG,
        "DAG" => SkillType::DAG,
        "MCP" => SkillType::MCP,
        "API" => SkillType::API,
        _ => SkillType::RAG, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingestion_topic_paths() {
        assert_eq!(IngestionTopics::events(), "federation.ingestion.events");
        assert_eq!(
            IngestionTopics::events_for_specialist("ariel"),
            "federation.ingestion.events.ariel"
        );
        assert_eq!(
            IngestionTopics::classification_by_domain("database"),
            "federation.ingestion.classification.database"
        );
    }

    #[test]
    fn test_ingestion_event_creation() {
        let event = IngestionEvent {
            event_id: "evt_001".to_string(),
            data_id: "data_123".to_string(),
            filename: Some("test.json".to_string()),
            file_format: Some("json".to_string()),
            file_size_bytes: Some(1024),
            detected_domains: vec!["database".to_string()],
            primary_domain: Some("database".to_string()),
            classification_confidence: 0.95,
            quality_score: 0.85,
            complexity: 0.5,
            timestamp: Utc::now(),
            status: IngestionStatus::Distilled,
        };

        assert_eq!(event.data_id, "data_123");
        assert_eq!(event.status, IngestionStatus::Distilled);
    }

    #[test]
    fn test_specialist_update_serialization() {
        let update = SpecialistUpdate {
            update_id: "upd_001".to_string(),
            specialist_id: "ariel".to_string(),
            xp_gained: 150,
            skill_type: "RAG".to_string(),
            quality_multiplier: 0.9,
            difficulty_multiplier: 1.5,
            source_data_id: "data_123".to_string(),
            source_filename: Some("crisis_log.txt".to_string()),
            is_breakthrough: false,
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&update).unwrap();
        assert!(json.contains("ariel"));
        assert!(json.contains("150"));
    }

    #[test]
    fn test_quality_metric_creation() {
        let metric = QualityMetric {
            metric_id: "qm_001".to_string(),
            data_id: "data_123".to_string(),
            overall_score: 0.85,
            format_quality: 0.9,
            semantic_quality: 0.8,
            training_value: 0.8,
            assessment_notes: vec!["Excellent data quality".to_string()],
            timestamp: Utc::now(),
        };

        assert_eq!(metric.overall_score, 0.85);
        assert_eq!(metric.assessment_notes.len(), 1);
    }

    #[test]
    fn test_classification_result() {
        let mut domains = HashMap::new();
        domains.insert("database".to_string(), 0.95);
        domains.insert("networking".to_string(), 0.70);

        let result = ClassificationResult {
            event_id: "evt_001".to_string(),
            data_id: "data_123".to_string(),
            domains,
            primary_domain: "database".to_string(),
            primary_confidence: 0.95,
            secondary_domains: vec![("networking".to_string(), 0.70)],
            structure_detected: StructureInfo {
                format: "json".to_string(),
                is_timeseries: false,
                record_count: Some(100),
                field_count: 5,
                nesting_depth: 2,
            },
            complexity_score: 0.5,
            timestamp: Utc::now(),
        };

        assert_eq!(result.primary_domain, "database");
        assert_eq!(result.secondary_domains.len(), 1);
    }

    #[test]
    fn test_ingestion_stats() {
        let mut specialist_util = HashMap::new();
        specialist_util.insert("ariel".to_string(), 500);
        specialist_util.insert("merlin".to_string(), 300);

        let stats = IngestionStats {
            stats_id: "stats_001".to_string(),
            period_start: Utc::now(),
            period_end: Utc::now(),
            files_received: 100,
            files_processed: 95,
            files_failed: 5,
            total_xp_distributed: 800,
            average_quality_score: 0.82,
            domains_detected: vec!["database".to_string(), "networking".to_string()],
            specialist_utilization: specialist_util,
            processing_time_ms_avg: 250,
        };

        assert_eq!(stats.files_processed, 95);
        assert_eq!(stats.total_xp_distributed, 800);
    }

    #[test]
    fn test_failure_event() {
        let failure = FailureEvent {
            failure_id: "fail_001".to_string(),
            data_id: "data_123".to_string(),
            filename: Some("corrupted.json".to_string()),
            failure_reason: "Invalid JSON format".to_string(),
            stage_failed: "classification".to_string(),
            error_details: Some("Expected value at line 1 column 1".to_string()),
            timestamp: Utc::now(),
        };

        assert_eq!(failure.stage_failed, "classification");
    }

    #[test]
    fn test_query_with_filters() {
        let mut filters = QueryFilters::default();
        filters.specialist_id = Some("ariel".to_string());
        filters.min_quality_score = Some(0.8);

        let query = IngestionQuery {
            query_id: "qry_001".to_string(),
            requested_by: "merlin".to_string(),
            query_type: IngestionQueryType::SpecialistEvents,
            filters,
            limit: 100,
            timestamp: Utc::now(),
        };

        assert_eq!(query.requested_by, "merlin");
        assert!(query.filters.specialist_id.is_some());
    }

    #[test]
    fn test_federation_config() {
        let config = FederationConfig::default();
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert!(config.enable_publishing);
    }
}
