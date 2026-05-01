/// Batch Processing System for Proposal Aggregation
/// 
/// Combines multiple proposals from specialists for more efficient
/// processing and reduced overhead

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

/// Batch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of proposals per batch
    pub max_batch_size: usize,
    /// Maximum time to wait for a full batch (ms)
    pub max_wait_time_ms: u64,
    /// Minimum proposals to start processing
    pub min_batch_size: usize,
    /// Enable adaptive batching based on workload
    pub adaptive: bool,
}

impl BatchConfig {
    /// Create aggressive batch config (large batches, shorter wait)
    pub fn aggressive() -> Self {
        Self {
            max_batch_size: 256,
            max_wait_time_ms: 50,
            min_batch_size: 32,
            adaptive: true,
        }
    }

    /// Create balanced batch config
    pub fn balanced() -> Self {
        Self {
            max_batch_size: 128,
            max_wait_time_ms: 100,
            min_batch_size: 16,
            adaptive: true,
        }
    }

    /// Create conservative batch config (small batches, longer wait)
    pub fn conservative() -> Self {
        Self {
            max_batch_size: 32,
            max_wait_time_ms: 500,
            min_batch_size: 4,
            adaptive: false,
        }
    }
}

/// A batch of proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalBatch {
    pub batch_id: String,
    pub proposals: Vec<crate::federation::specialist::ProposedAction>,
    pub created_at_ms: u64,
    pub ready_for_processing: bool,
    pub processing_started_at_ms: Option<u64>,
    pub processing_duration_ms: Option<u64>,
}

impl ProposalBatch {
    /// Create new batch
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            batch_id: format!("batch_{}", uuid::Uuid::new_v4()),
            proposals: Vec::new(),
            created_at_ms: now,
            ready_for_processing: false,
            processing_started_at_ms: None,
            processing_duration_ms: None,
        }
    }

    /// Add proposal to batch
    pub fn add_proposal(&mut self, proposal: crate::federation::specialist::ProposedAction) -> bool {
        self.proposals.push(proposal);
        true
    }

    /// Get batch size
    pub fn size(&self) -> usize {
        self.proposals.len()
    }

    /// Mark batch as ready
    pub fn mark_ready(&mut self) {
        self.ready_for_processing = true;
    }

    /// Mark batch as being processed
    pub fn mark_processing_started(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.processing_started_at_ms = Some(now);
    }

    /// Mark batch as completed
    pub fn mark_processing_completed(&mut self) {
        if let Some(start) = self.processing_started_at_ms {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.processing_duration_ms = Some(now - start);
        }
    }

    /// Get average proposal priority
    pub fn avg_priority(&self) -> f32 {
        if self.proposals.is_empty() {
            return 0.0;
        }
        let total: u32 = self
            .proposals
            .iter()
            .map(|p| match p.priority {
                crate::federation::specialist::ProposalPriority::Background => 0,
                crate::federation::specialist::ProposalPriority::Normal => 1,
                crate::federation::specialist::ProposalPriority::UserFacing => 2,
                crate::federation::specialist::ProposalPriority::Urgent => 3,
            })
            .sum();
        total as f32 / self.proposals.len() as f32
    }

    /// Get average confidence
    pub fn avg_confidence(&self) -> f32 {
        if self.proposals.is_empty() {
            return 0.0;
        }
        let total: f32 = self.proposals.iter().map(|p| p.confidence).sum();
        total / self.proposals.len() as f32
    }

    /// Age of batch in milliseconds
    pub fn age_ms(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now - self.created_at_ms
    }
}

impl Default for ProposalBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch manager for processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchManager {
    pub config: BatchConfig,
    pub current_batch: Option<ProposalBatch>,
    pub completed_batches: VecDeque<ProposalBatch>,
    pub total_proposals_batched: u64,
    pub total_batches_processed: u64,
}

impl BatchManager {
    /// Create new batch manager
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            current_batch: None,
            completed_batches: VecDeque::new(),
            total_proposals_batched: 0,
            total_batches_processed: 0,
        }
    }

    /// Add proposal to current batch
    pub fn add_proposal(&mut self, proposal: crate::federation::specialist::ProposedAction) -> Option<ProposalBatch> {
        // Create new batch if needed
        if self.current_batch.is_none() {
            self.current_batch = Some(ProposalBatch::new());
        }

        let batch = self.current_batch.as_mut().unwrap();
        batch.add_proposal(proposal);
        self.total_proposals_batched += 1;

        // Check if batch is ready to process
        if batch.size() >= self.config.max_batch_size {
            batch.mark_ready();
            let completed = self.current_batch.take();
            self.current_batch = None;
            return completed;
        }

        None
    }

    /// Check if current batch should be flushed
    pub fn should_flush(&self) -> bool {
        if let Some(batch) = &self.current_batch {
            let age = batch.age_ms();
            batch.size() >= self.config.min_batch_size && age >= self.config.max_wait_time_ms
        } else {
            false
        }
    }

    /// Flush current batch (return it even if not full)
    pub fn flush_batch(&mut self) -> Option<ProposalBatch> {
        if let Some(mut batch) = self.current_batch.take() {
            if !batch.proposals.is_empty() {
                batch.mark_ready();
                return Some(batch);
            }
        }
        None
    }

    /// Process a batch.
    ///
    /// Marks the batch as processing started and completed, tracks timing,
    /// and stores it in the completed ring buffer. No actual work is done
    /// here — the caller (e.g. the Sentinel arbitration loop) is responsible
    /// for executing the contained proposals.
    ///
    /// Note: the original implementation used `std::thread::sleep(10ms)` here,
    /// which would block the async executor. This version does not sleep.
    pub fn process_batch(&mut self, mut batch: ProposalBatch) {
        batch.mark_processing_started();
        // No blocking sleep — work is done by the caller before/after this call
        batch.mark_processing_completed();

        self.completed_batches.push_back(batch);
        self.total_batches_processed += 1;

        // Keep history bounded
        while self.completed_batches.len() > 1000 {
            self.completed_batches.pop_front();
        }
    }

    /// Get batch throughput (proposals per second)
    pub fn throughput(&self) -> f32 {
        if self.completed_batches.is_empty() {
            return 0.0;
        }

        let oldest = &self.completed_batches[0];
        let newest = &self.completed_batches[self.completed_batches.len() - 1];

        if let (Some(oldest_start), Some(newest_start), Some(newest_end)) = (oldest.processing_started_at_ms, newest.processing_started_at_ms, newest.processing_duration_ms) {
            let total_time_ms = (newest_start + newest_end).saturating_sub(oldest_start);
            if total_time_ms > 0 {
                return (self.total_proposals_batched as f32 / total_time_ms as f32) * 1000.0;
            }
        }

        0.0
    }

    /// Get average batch size
    pub fn avg_batch_size(&self) -> f32 {
        if self.completed_batches.is_empty() {
            return 0.0;
        }
        let total: usize = self.completed_batches.iter().map(|b| b.size()).sum();
        total as f32 / self.completed_batches.len() as f32
    }

    /// Get average processing time per batch
    pub fn avg_batch_processing_time_ms(&self) -> f32 {
        let batches_with_duration: Vec<_> = self
            .completed_batches
            .iter()
            .filter_map(|b| b.processing_duration_ms)
            .collect();

        if batches_with_duration.is_empty() {
            return 0.0;
        }

        let total: u64 = batches_with_duration.iter().sum();
        total as f32 / batches_with_duration.len() as f32
    }
}

impl Default for BatchManager {
    fn default() -> Self {
        Self::new(BatchConfig::balanced())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_sizes() {
        let aggressive = BatchConfig::aggressive();
        let balanced = BatchConfig::balanced();
        let conservative = BatchConfig::conservative();

        assert!(aggressive.max_batch_size > balanced.max_batch_size);
        assert!(balanced.max_batch_size > conservative.max_batch_size);
    }

    #[test]
    fn test_proposal_batch_creation() {
        let batch = ProposalBatch::new();
        assert_eq!(batch.size(), 0);
        assert!(!batch.ready_for_processing);
    }

    #[test]
    fn test_proposal_batch_add() {
        let mut batch = ProposalBatch::new();
        let proposal = crate::federation::specialist::ProposedAction {
            id: "test".to_string(),
            specialist: crate::federation::specialist::SpecialistId::Sentinel,
            action_type: "test".to_string(),
            description: "test".to_string(),
            confidence: 0.8,
            required_resources: crate::federation::specialist::ResourceRequest::default(),
            priority: crate::federation::specialist::ProposalPriority::Normal,
            tags: vec![],
        };

        batch.add_proposal(proposal);
        assert_eq!(batch.size(), 1);
    }

    #[test]
    fn test_proposal_batch_mark_ready() {
        let mut batch = ProposalBatch::new();
        batch.mark_ready();
        assert!(batch.ready_for_processing);
    }

    #[test]
    fn test_batch_manager_add_proposal() {
        let mut manager = BatchManager::new(BatchConfig::balanced());
        let proposal = crate::federation::specialist::ProposedAction {
            id: "test".to_string(),
            specialist: crate::federation::specialist::SpecialistId::Sentinel,
            action_type: "test".to_string(),
            description: "test".to_string(),
            confidence: 0.8,
            required_resources: crate::federation::specialist::ResourceRequest::default(),
            priority: crate::federation::specialist::ProposalPriority::Normal,
            tags: vec![],
        };

        manager.add_proposal(proposal);
        assert_eq!(manager.total_proposals_batched, 1);
        assert!(manager.current_batch.is_some());
    }

    #[test]
    fn test_batch_manager_flush() {
        let mut manager = BatchManager::new(BatchConfig::conservative());
        let proposal = crate::federation::specialist::ProposedAction {
            id: "test".to_string(),
            specialist: crate::federation::specialist::SpecialistId::Sentinel,
            action_type: "test".to_string(),
            description: "test".to_string(),
            confidence: 0.8,
            required_resources: crate::federation::specialist::ResourceRequest::default(),
            priority: crate::federation::specialist::ProposalPriority::Normal,
            tags: vec![],
        };

        manager.add_proposal(proposal);
        let flushed = manager.flush_batch();
        assert!(flushed.is_some());
        assert!(manager.current_batch.is_none());
    }
}
