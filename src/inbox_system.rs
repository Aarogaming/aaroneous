use crate::data_ingestion::{IngestibleData, IngestionConfig, IngestionStatus, copy_file_non_destructive};
use crate::content_analyzer::ContentAnalyzer;
use crate::data_distillation::{DistillationEngine, DistillationConfig, DataCrystallizer};
use crate::capability_matcher::CapabilityMatcher;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use log::{info, warn, error};

/// Statistics for inbox monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InboxStats {
    pub files_received: u64,
    pub files_processed: u64,
    pub files_failed: u64,
    pub total_xp_distributed: u32,
    pub last_activity: Option<DateTime<Utc>>,
}

/// Inbox System: Orchestrates the complete data ingestion pipeline
pub struct InboxSystem {
    config: IngestionConfig,
    distillation_engine: DistillationEngine,
    capability_matcher: CapabilityMatcher,
    stats: Arc<Mutex<InboxStats>>,
    processing_queue: Arc<Mutex<VecDeque<IngestibleData>>>,
}

impl InboxSystem {
    /// Create a new inbox system
    pub fn new(config: IngestionConfig) -> Self {
        let distillation_engine = DistillationEngine::new(DistillationConfig::default());
        let capability_matcher = CapabilityMatcher::load_default_specialists();

        Self {
            config,
            distillation_engine,
            capability_matcher,
            stats: Arc::new(Mutex::new(InboxStats::default())),
            processing_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Initialize system and create required directories
    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config.init_directories().await?;
        info!("[Inbox] Initialized directories");
        Ok(())
    }

    /// Ingest a file from the inbox folder
    pub async fn ingest_file(&self, file_path: &Path) -> Result<IngestibleData, String> {
        // Validate file
        let metadata = tokio::fs::metadata(file_path)
            .await
            .map_err(|e| format!("Cannot access file: {}", e))?;

        if metadata.len() > (self.config.max_file_size_mb * 1024 * 1024) as u64 {
            return Err(format!("File exceeds maximum size of {} MB", self.config.max_file_size_mb));
        }

        // Create ingestible data
        let mut data = IngestibleData::from_file(file_path.to_path_buf());

        // Copy file non-destructively to processing directory
        let copy_result = copy_file_non_destructive(file_path, &self.config.processing_path)
            .await
            .map_err(|e| format!("Failed to copy file: {}", e))?;

        data.metadata.size_bytes = Some(copy_result.bytes_copied);
        data.set_status(IngestionStatus::Copied);

        // Load content for text files
        if let Some(format) = data.format {
            if format.is_text() && metadata.len() < (self.config.content_sample_size_bytes as u64) {
                match tokio::fs::read_to_string(file_path).await {
                    Ok(content) => {
                        data.metadata.content_sample = Some(content.chars().take(self.config.content_sample_size_bytes).collect());
                        data.content = Some(content);
                    }
                    Err(e) => {
                        warn!("[Inbox] Could not read file content: {}", e);
                    }
                }
            }
        }

        info!("[Inbox] File ingested: {} ({} bytes)", file_path.display(), copy_result.bytes_copied);

        Ok(data)
    }

    /// Process ingested data through the complete pipeline
    pub async fn process_data(&self, mut data: IngestibleData) -> Result<ProcessingResult, String> {
        // Step 1: Analyze content
        let analysis = ContentAnalyzer::analyze(&data);
        data.set_status(IngestionStatus::Classified);
        info!("[Inbox] Content analyzed: {} domains detected", analysis.domains.len());

        // Step 2: Distill into training examples and events
        let distillation_result = self.distillation_engine.distill(&data, &analysis);
        data.set_status(IngestionStatus::Distilled);
        info!("[Inbox] Distilled {} matches from {}", distillation_result.matches.len(), data.id);

        // Step 3: Crystallize results
        let crystallized = DataCrystallizer::crystallize(distillation_result.clone());

        // Step 4: Update statistics
        let mut stats = self.stats.lock().await;
        stats.files_processed += 1;
        stats.last_activity = Some(Utc::now());
        for (_specialist, xp) in &crystallized.xp_awards {
            stats.total_xp_distributed += xp;
        }
        drop(stats);

        // Step 5: Events are generated and ready for processing
        // In a full implementation, these would be published to NATS or EventLoop
        for event in &distillation_result.events {
            info!("[Inbox] Event ready: {} gains {} XP for {}",
                event.specialist_id, event.xp_awarded, event.skill_name);
        }

        data.set_status(IngestionStatus::Completed);

        // Archive processed file
        Self::archive_file(&self.config.processed_path, &data.file_path)
            .await
            .ok();

        Ok(ProcessingResult {
            data_id: data.id.clone(),
            matches: distillation_result.matches.len(),
            events_generated: distillation_result.events.len(),
            total_xp: crystallized.xp_awards.values().sum(),
            quality_score: distillation_result.quality_assessment.overall_score,
            domains: crystallized.domains_identified,
            timestamp: Utc::now(),
        })
    }

    /// Monitor inbox folder for new files (async background task)
    pub async fn monitor_inbox(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("[Inbox] Starting file watcher on {:?}", self.config.inbox_path);

        loop {
            // Scan inbox directory
            match tokio::fs::read_dir(&self.config.inbox_path).await {
                Ok(mut entries) => {
                    while let Some(entry) = entries.next_entry().await? {
                        let path = entry.path();
                        if path.is_file() {
                            // Check if already processed (simple approach: track in memory)
                            match self.ingest_file(&path).await {
                                Ok(data) => {
                                    // Queue for processing
                                    self.processing_queue.lock().await.push_back(data);
                                }
                                Err(e) => {
                                    error!("[Inbox] Failed to ingest {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("[Inbox] Error reading inbox directory: {}", e);
                }
            }

            // Process queued items
            while let Some(data) = self.processing_queue.lock().await.pop_front() {
                match self.process_data(data).await {
                    Ok(result) => {
                        info!("[Inbox] Successfully processed: {} ({} matches, {} events, {} XP)",
                            result.data_id, result.matches, result.events_generated, result.total_xp);
                    }
                    Err(e) => {
                        error!("[Inbox] Processing failed: {}", e);
                    }
                }
            }

            // Wait before next scan
            tokio::time::sleep(tokio::time::Duration::from_millis(self.config.scan_interval_ms)).await;
        }
    }

    /// Archive a processed file
    async fn archive_file(archive_dir: &Path, file_path: &Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = file_path {
            let filename = path.file_name().ok_or("No filename")?;
            let archive_path = archive_dir.join(filename);
            tokio::fs::copy(path, archive_path).await?;
        }
        Ok(())
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> InboxStats {
        self.stats.lock().await.clone()
    }

    /// Get queue size
    pub async fn get_queue_size(&self) -> usize {
        self.processing_queue.lock().await.len()
    }
}

/// Result of processing a single data item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    pub data_id: String,
    pub matches: usize,
    pub events_generated: usize,
    pub total_xp: u32,
    pub quality_score: f32,
    pub domains: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inbox_system_creation() {
        let config = IngestionConfig::default();
        let system = InboxSystem::new(config);
        
        assert_eq!(system.config.max_file_size_mb, 512);
    }

    #[tokio::test]
    async fn test_inbox_stats() {
        let config = IngestionConfig::default();
        let system = InboxSystem::new(config);
        
        let stats = system.get_stats().await;
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_failed, 0);
    }

    #[tokio::test]
    async fn test_processing_queue() {
        let config = IngestionConfig::default();
        let system = InboxSystem::new(config);
        
        assert_eq!(system.get_queue_size().await, 0);
    }
}
