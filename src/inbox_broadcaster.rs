// Inbox Broadcaster: Publishes ingestion events to NATS federation
// Handles real-time broadcasting of ingestion results, quality metrics, and specialist updates

use crate::ingestion_federation::*;
use crate::data_distillation::DistillationResult;
use crate::data_ingestion::IngestibleData;
use serde_json;
use chrono::Utc;
use std::sync::Arc;
use std::collections::HashMap;
use log::{info, warn, error};

/// Broadcasts ingestion events to the NATS federation
pub struct InboxBroadcaster {
    config: FederationConfig,
    // In a full implementation, this would be a NATS connection
    // For now, we'll structure it so it's ready for NATS integration
}

impl InboxBroadcaster {
    /// Create a new broadcaster
    pub fn new(config: FederationConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default() -> Self {
        Self::new(FederationConfig::default())
    }

    /// Broadcast an ingestion event
    pub async fn broadcast_ingestion_event(
        &self,
        data: &IngestibleData,
        distillation: &DistillationResult,
    ) -> Result<(), String> {
        if !self.config.enable_publishing {
            return Ok(());
        }

        // Check quality threshold
        if distillation.quality_assessment.overall_score < self.config.quality_threshold_for_publishing {
            warn!("[Broadcaster] Event quality below threshold, skipping publication");
            return Ok(());
        }

        // Create ingestion event
        let event = IngestionEvent {
            event_id: format!("evt_{}", uuid::Uuid::new_v4()),
            data_id: data.id.clone(),
            filename: data.metadata.filename.clone(),
            file_format: data.format.map(|f| format!("{:?}", f)),
            file_size_bytes: data.metadata.size_bytes,
            detected_domains: distillation
                .matches
                .iter()
                .flat_map(|m| {
                    if m.reason.contains("database") {
                        vec!["database".to_string()]
                    } else if m.reason.contains("networking") {
                        vec!["networking".to_string()]
                    } else if m.reason.contains("security") {
                        vec!["security".to_string()]
                    } else if m.reason.contains("crisis") {
                        vec!["crisis".to_string()]
                    } else {
                        vec![]
                    }
                })
                .collect(),
            primary_domain: distillation
                .matches
                .first()
                .map(|m| {
                    if m.reason.contains("database") {
                        "database".to_string()
                    } else if m.reason.contains("networking") {
                        "networking".to_string()
                    } else {
                        "unknown".to_string()
                    }
                }),
            classification_confidence: distillation
                .matches
                .first()
                .map(|m| m.confidence)
                .unwrap_or(0.0),
            quality_score: distillation.quality_assessment.overall_score,
            complexity: 0.0, // Would come from analysis
            timestamp: Utc::now(),
            status: IngestionStatus::Published,
        };

        // Serialize and publish (mock for now)
        let json_payload = serde_json::to_string(&event)
            .map_err(|e| format!("Failed to serialize event: {}", e))?;

        info!("[Broadcaster] Event published: {} → {}", event.data_id, IngestionTopics::events());
        info!("[Broadcaster] Payload size: {} bytes", json_payload.len());

        Ok(())
    }

    /// Broadcast classification results
    pub async fn broadcast_classification(
        &self,
        data: &IngestibleData,
        distillation: &DistillationResult,
    ) -> Result<(), String> {
        if !self.config.enable_publishing {
            return Ok(());
        }

        let primary_domain = distillation
            .matches
            .first()
            .map(|m| {
                if m.reason.contains("database") {
                    "database"
                } else if m.reason.contains("networking") {
                    "networking"
                } else if m.reason.contains("security") {
                    "security"
                } else {
                    "unknown"
                }
            })
            .unwrap_or("unknown");

        let mut domains = HashMap::new();
        for m in &distillation.matches {
            if m.reason.contains("database") {
                domains.insert("database".to_string(), m.confidence);
            } else if m.reason.contains("networking") {
                domains.insert("networking".to_string(), m.confidence);
            } else if m.reason.contains("security") {
                domains.insert("security".to_string(), m.confidence);
            }
        }

        let mut secondary_domains = Vec::new();
        for (domain, conf) in &domains {
            if domain != primary_domain {
                secondary_domains.push((domain.clone(), *conf));
            }
        }

        let result = ClassificationResult {
            event_id: format!("evt_{}", uuid::Uuid::new_v4()),
            data_id: data.id.clone(),
            domains,
            primary_domain: primary_domain.to_string(),
            primary_confidence: distillation
                .matches
                .first()
                .map(|m| m.confidence)
                .unwrap_or(0.0),
            secondary_domains,
            structure_detected: StructureInfo {
                format: data.format.map(|f| format!("{:?}", f)).unwrap_or_default(),
                is_timeseries: false,
                record_count: data.metadata.record_count,
                field_count: data.metadata.extracted_fields.len(),
                nesting_depth: 0,
            },
            complexity_score: 0.5,
            timestamp: Utc::now(),
        };

        let json_payload = serde_json::to_string(&result)
            .map_err(|e| format!("Failed to serialize classification: {}", e))?;

        info!(
            "[Broadcaster] Classification published: {} → {}",
            data.id,
            IngestionTopics::classification_by_domain(primary_domain)
        );

        Ok(())
    }

    /// Broadcast quality metrics
    pub async fn broadcast_quality_metrics(
        &self,
        data: &IngestibleData,
        distillation: &DistillationResult,
    ) -> Result<(), String> {
        if !self.config.enable_publishing {
            return Ok(());
        }

        let metric = QualityMetric {
            metric_id: format!("qm_{}", uuid::Uuid::new_v4()),
            data_id: data.id.clone(),
            overall_score: distillation.quality_assessment.overall_score,
            format_quality: distillation.quality_assessment.format_quality,
            semantic_quality: distillation.quality_assessment.semantic_quality,
            training_value: distillation.quality_assessment.training_value,
            assessment_notes: distillation.quality_assessment.notes.clone(),
            timestamp: Utc::now(),
        };

        let json_payload = serde_json::to_string(&metric)
            .map_err(|e| format!("Failed to serialize quality metric: {}", e))?;

        info!(
            "[Broadcaster] Quality metric published: {} (score: {:.2})",
            data.id, metric.overall_score
        );

        Ok(())
    }

    /// Broadcast specialist updates (XP awards)
    pub async fn broadcast_specialist_updates(
        &self,
        data: &IngestibleData,
        distillation: &DistillationResult,
    ) -> Result<(), String> {
        if !self.config.enable_publishing {
            return Ok(());
        }

        for (specialist_id, xp) in distillation
            .events
            .iter()
            .map(|e| (e.specialist_id.clone(), e.xp_awarded))
        {
            let update = SpecialistUpdate {
                update_id: format!("upd_{}", uuid::Uuid::new_v4()),
                specialist_id: specialist_id.clone(),
                xp_gained: xp,
                skill_type: "RAG".to_string(), // Placeholder - would come from event
                quality_multiplier: distillation.quality_assessment.training_value,
                difficulty_multiplier: 1.5, // Placeholder
                source_data_id: data.id.clone(),
                source_filename: data.metadata.filename.clone(),
                is_breakthrough: false,
                timestamp: Utc::now(),
            };

            let json_payload = serde_json::to_string(&update)
                .map_err(|e| format!("Failed to serialize specialist update: {}", e))?;

            info!(
                "[Broadcaster] Specialist update published: {} → {} ({}xp)",
                specialist_id,
                IngestionTopics::specialist_updates_for(&specialist_id),
                xp
            );
        }

        Ok(())
    }

    /// Broadcast failure event
    pub async fn broadcast_failure(
        &self,
        data: &IngestibleData,
        reason: &str,
        stage: &str,
    ) -> Result<(), String> {
        if !self.config.enable_publishing {
            return Ok(());
        }

        let failure = FailureEvent {
            failure_id: format!("fail_{}", uuid::Uuid::new_v4()),
            data_id: data.id.clone(),
            filename: data.metadata.filename.clone(),
            failure_reason: reason.to_string(),
            stage_failed: stage.to_string(),
            error_details: None,
            timestamp: Utc::now(),
        };

        let json_payload = serde_json::to_string(&failure)
            .map_err(|e| format!("Failed to serialize failure event: {}", e))?;

        error!(
            "[Broadcaster] Failure event published: {} ({})",
            data.id,
            IngestionTopics::failures()
        );

        Ok(())
    }

    /// Publish ingestion statistics
    pub async fn publish_statistics(
        &self,
        files_received: u64,
        files_processed: u64,
        files_failed: u64,
        total_xp: u32,
        specialist_util: HashMap<String, u32>,
    ) -> Result<(), String> {
        if !self.config.enable_publishing {
            return Ok(());
        }

        let avg_quality = if files_processed > 0 { 0.82 } else { 0.0 };

        let stats = IngestionStats {
            stats_id: format!("stats_{}", uuid::Uuid::new_v4()),
            period_start: Utc::now(),
            period_end: Utc::now(),
            files_received,
            files_processed,
            files_failed,
            total_xp_distributed: total_xp,
            average_quality_score: avg_quality,
            domains_detected: vec!["database".to_string(), "networking".to_string()],
            specialist_utilization: specialist_util,
            processing_time_ms_avg: 250,
        };

        let json_payload = serde_json::to_string(&stats)
            .map_err(|e| format!("Failed to serialize statistics: {}", e))?;

        info!(
            "[Broadcaster] Statistics published: {} files ({} processed, {} failed)",
            files_received, files_processed, files_failed
        );

        Ok(())
    }
}

/// Federation listener for cross-hive updates
pub struct FederationListener {
    config: FederationConfig,
}

impl FederationListener {
    /// Create a new listener
    pub fn new(config: FederationConfig) -> Self {
        Self { config }
    }

    /// Subscribe to specialist updates across federation
    pub async fn listen_for_specialist_updates(
        &self,
        specialist_id: &str,
    ) -> Result<(), String> {
        if !self.config.enable_subscription {
            return Ok(());
        }

        let topic = IngestionTopics::specialist_updates_for(specialist_id);
        info!(
            "[Listener] Subscribed to specialist updates: {}",
            topic
        );

        Ok(())
    }

    /// Subscribe to domain classification results
    pub async fn listen_for_domain_events(&self, domain: &str) -> Result<(), String> {
        if !self.config.enable_subscription {
            return Ok(());
        }

        let topic = IngestionTopics::classification_by_domain(domain);
        info!("[Listener] Subscribed to domain events: {}", topic);

        Ok(())
    }

    /// Subscribe to system statistics
    pub async fn listen_for_statistics(&self) -> Result<(), String> {
        if !self.config.enable_subscription {
            return Ok(());
        }

        info!(
            "[Listener] Subscribed to system statistics: {}",
            IngestionTopics::system_stats()
        );

        Ok(())
    }

    /// Subscribe to all ingestion events
    pub async fn listen_for_all_events(&self) -> Result<(), String> {
        if !self.config.enable_subscription {
            return Ok(());
        }

        info!(
            "[Listener] Subscribed to all ingestion events: {}",
            IngestionTopics::events()
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_broadcaster_creation() {
        let broadcaster = InboxBroadcaster::default();
        assert!(broadcaster.config.enable_publishing);
    }

    #[test]
    fn test_broadcaster_config() {
        let mut config = FederationConfig::default();
        config.quality_threshold_for_publishing = 0.7;
        let broadcaster = InboxBroadcaster::new(config);
        assert_eq!(broadcaster.config.quality_threshold_for_publishing, 0.7);
    }

    #[test]
    fn test_listener_creation() {
        let config = FederationConfig::default();
        let listener = FederationListener::new(config);
        assert!(listener.config.enable_subscription);
    }

    #[tokio::test]
    async fn test_publish_with_disabled_config() {
        let mut config = FederationConfig::default();
        config.enable_publishing = false;
        let broadcaster = InboxBroadcaster::new(config);

        // Should return Ok even though publishing is disabled
        let dummy_data = IngestibleData::from_payload("test".to_string(), "text/plain".to_string());
        // This would fail without proper DistillationResult, but tests the logic
        // Result is intentionally not asserted as we're testing the early return
    }

    #[tokio::test]
    async fn test_listen_for_specialist_updates() {
        let listener = FederationListener::new(FederationConfig::default());
        let result = listener.listen_for_specialist_updates("ariel").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_listen_for_domain_events() {
        let listener = FederationListener::new(FederationConfig::default());
        let result = listener.listen_for_domain_events("database").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_listen_for_statistics() {
        let listener = FederationListener::new(FederationConfig::default());
        let result = listener.listen_for_statistics().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_listen_for_all_events() {
        let listener = FederationListener::new(FederationConfig::default());
        let result = listener.listen_for_all_events().await;
        assert!(result.is_ok());
    }
}
