/// Memory Archival System
///
/// Manages archival of old memory entries to separate storage,
/// cleanup of unused data, and selective restoration.

use crate::specialist_memory::MemoryEntry;
use crate::specialist_memory_compression::{CompressedMemoryEntry, MemoryCompressor};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Configuration for memory archival
#[derive(Debug, Clone)]
pub struct ArchivalConfig {
    /// Archive entries older than this many days
    pub archive_threshold_days: u32,
    /// Maximum entries to keep in hot storage
    pub max_hot_entries: usize,
    /// Maximum entries to keep in warm storage
    pub max_warm_entries: usize,
    /// Enable automatic archival
    pub enable_auto_archival: bool,
    /// Auto-archival interval in hours
    pub auto_archival_interval_hours: u32,
}

impl Default for ArchivalConfig {
    fn default() -> Self {
        Self {
            archive_threshold_days: 30,
            max_hot_entries: 10000,
            max_warm_entries: 50000,
            enable_auto_archival: true,
            auto_archival_interval_hours: 24,
        }
    }
}

/// Statistics for archived memory
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchiveStats {
    pub total_archived: u64,
    pub total_restored: u64,
    pub archive_size_bytes: usize,
    pub last_archival_timestamp: Option<i64>,
    pub entries_by_type: HashMap<String, u64>,
}

/// Memory archive manager
pub struct MemoryArchiveManager {
    config: ArchivalConfig,
    stats: ArchiveStats,
}

impl MemoryArchiveManager {
    /// Create new archive manager
    pub fn new(config: ArchivalConfig) -> Self {
        Self {
            config,
            stats: ArchiveStats::default(),
        }
    }

    /// Determine if entry should be archived
    pub fn should_archive(&self, entry: &MemoryEntry) -> bool {
        let age_days = (Utc::now() - entry.created_at).num_days() as u32;
        age_days >= self.config.archive_threshold_days
    }

    /// Archive memory entries (compress + move to archive storage)
    pub async fn archive_entries(
        &mut self,
        entries: &[MemoryEntry],
    ) -> Result<ArchiveResult> {
        let mut archived_count = 0;
        let mut compressed_entries = Vec::new();
        let mut entries_by_type = HashMap::new();

        for entry in entries {
            if self.should_archive(entry) {
                // Compress entry
                let compressed = MemoryCompressor::compress(entry)?;
                let type_str = format!("{:?}", entry.memory_type);
                *entries_by_type.entry(type_str).or_insert(0) += 1;

                compressed_entries.push(compressed);
                archived_count += 1;
            }
        }

        let archive_size: usize = compressed_entries
            .iter()
            .map(|e| e.id.len() + e.context.len() + e.tags.len() + 100)
            .sum();

        self.stats.total_archived += archived_count;
        self.stats.archive_size_bytes += archive_size;
        self.stats.last_archival_timestamp = Some(Utc::now().timestamp());
        self.stats.entries_by_type = entries_by_type.clone();

        info!(
            "Archived {} entries ({} bytes)",
            archived_count, archive_size
        );

        Ok(ArchiveResult {
            archived_count: archived_count as u64,
            compressed_entries,
            archive_size_bytes: archive_size,
        })
    }

    /// Restore entries from archive (by entry ID)
    pub async fn restore_entry(&mut self, archived_entry: &CompressedMemoryEntry) -> Result<MemoryEntry> {
        let entry = MemoryCompressor::decompress(archived_entry)?;
        self.stats.total_restored += 1;
        debug!("Restored entry: {}", entry.id);
        Ok(entry)
    }

    /// Batch restore entries
    pub async fn restore_entries(
        &mut self,
        archived_entries: &[CompressedMemoryEntry],
    ) -> Result<Vec<MemoryEntry>> {
        let mut restored = Vec::new();
        for archived in archived_entries {
            match self.restore_entry(archived).await {
                Ok(entry) => restored.push(entry),
                Err(e) => warn!("Failed to restore entry: {}", e),
            }
        }
        Ok(restored)
    }

    /// Cleanup old archived entries (older than retention period)
    pub async fn cleanup_old_archived(
        &mut self,
        retention_days: u32,
        archived_entries: &[CompressedMemoryEntry],
    ) -> Result<CleanupResult> {
        let mut kept = Vec::new();
        let mut deleted_count = 0;
        let mut freed_bytes = 0;

        let cutoff_date = Utc::now() - Duration::days(retention_days as i64);

        for entry in archived_entries {
            let created_at = DateTime::<Utc>::from_timestamp(entry.created_at, 0)
                .unwrap_or_else(|| Utc::now());

            if created_at < cutoff_date {
                // Entry is older than retention period, delete it
                deleted_count += 1;
                freed_bytes += entry.id.len() + entry.description.len() + 100;
            } else {
                // Keep this entry
                kept.push(entry.clone());
            }
        }

        info!(
            "Cleaned up {} archived entries, freed {} bytes",
            deleted_count, freed_bytes
        );

        Ok(CleanupResult {
            deleted_count: deleted_count as u64,
            freed_bytes,
            kept_entries: kept,
        })
    }

    /// Get archive statistics
    pub fn get_stats(&self) -> ArchiveStats {
        self.stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = ArchiveStats::default();
    }
}

/// Result of archival operation
#[derive(Debug, Clone)]
pub struct ArchiveResult {
    pub archived_count: u64,
    pub compressed_entries: Vec<CompressedMemoryEntry>,
    pub archive_size_bytes: usize,
}

/// Result of cleanup operation
#[derive(Debug, Clone)]
pub struct CleanupResult {
    pub deleted_count: u64,
    pub freed_bytes: usize,
    pub kept_entries: Vec<CompressedMemoryEntry>,
}

/// Memory cleanup policy
#[derive(Debug, Clone)]
pub struct CleanupPolicy {
    /// Maximum total memory entries to keep
    pub max_total_entries: usize,
    /// Delete entries when count exceeds this
    pub cleanup_threshold: usize,
    /// Number of entries to keep after cleanup
    pub target_entries_after_cleanup: usize,
    /// Keep only entries with confidence >= this
    pub min_confidence: f32,
    /// Delete if not accessed in this many days
    pub unused_threshold_days: u32,
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            max_total_entries: 100000,
            cleanup_threshold: 95000,
            target_entries_after_cleanup: 80000,
            min_confidence: 0.0,
            unused_threshold_days: 90,
        }
    }
}

/// Policy-based memory cleanup manager
pub struct PolicyBasedCleanupManager {
    policy: CleanupPolicy,
}

impl PolicyBasedCleanupManager {
    /// Create new cleanup manager with policy
    pub fn new(policy: CleanupPolicy) -> Self {
        Self { policy }
    }

    /// Determine if cleanup is needed
    pub fn needs_cleanup(&self, current_count: usize) -> bool {
        current_count >= self.policy.cleanup_threshold
    }

    /// Apply cleanup policy to entries
    pub async fn apply_cleanup(
        &self,
        entries: &[MemoryEntry],
    ) -> Result<CleanupPolicyResult> {
        let mut to_delete = Vec::new();
        let mut to_keep = Vec::new();

        let cutoff_date = Utc::now() - Duration::days(self.policy.unused_threshold_days as i64);

        for entry in entries {
            let last_accessed = entry.updated_at;
            let is_unused = last_accessed < cutoff_date;
            let low_confidence = (entry.confidence as u8) < (self.policy.min_confidence * 100.0) as u8;

            if is_unused || low_confidence {
                to_delete.push(entry.clone());
            } else {
                to_keep.push(entry.clone());
            }
        }

        // If still too many, sort by confidence and delete the lowest
        if to_keep.len() > self.policy.target_entries_after_cleanup {
            let mut sorted = to_keep.clone();
            sorted.sort_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap());

            let excess = sorted.len() - self.policy.target_entries_after_cleanup;
            to_delete.extend_from_slice(&sorted[..excess]);
            to_keep = sorted[excess..].to_vec();
        }

        let deleted_bytes: usize = to_delete
            .iter()
            .map(|e| e.description.len() + e.id.len() + 100)
            .sum();

        info!(
            "Cleanup policy: deleted {} entries, freed {} bytes",
            to_delete.len(),
            deleted_bytes
        );

        Ok(CleanupPolicyResult {
            deleted_entries: to_delete,
            kept_entries: to_keep,
            freed_bytes: deleted_bytes,
        })
    }
}

/// Result of policy-based cleanup
#[derive(Debug, Clone)]
pub struct CleanupPolicyResult {
    pub deleted_entries: Vec<MemoryEntry>,
    pub kept_entries: Vec<MemoryEntry>,
    pub freed_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archival_config_default() {
        let config = ArchivalConfig::default();
        assert_eq!(config.archive_threshold_days, 30);
        assert_eq!(config.max_hot_entries, 10000);
        assert!(config.enable_auto_archival);
    }

    #[test]
    fn test_archive_stats_creation() {
        let stats = ArchiveStats::default();
        assert_eq!(stats.total_archived, 0);
        assert_eq!(stats.total_restored, 0);
    }

    #[test]
    fn test_cleanup_policy_default() {
        let policy = CleanupPolicy::default();
        assert_eq!(policy.max_total_entries, 100000);
        assert_eq!(policy.cleanup_threshold, 95000);
    }

    #[tokio::test]
    async fn test_archive_manager_creation() {
        let config = ArchivalConfig::default();
        let manager = MemoryArchiveManager::new(config);
        let stats = manager.get_stats();
        assert_eq!(stats.total_archived, 0);
    }

    #[test]
    fn test_should_archive_old_entry() {
        let config = ArchivalConfig {
            archive_threshold_days: 30,
            ..Default::default()
        };
        let manager = MemoryArchiveManager::new(config);

        let old_entry = MemoryEntry {
            id: "old-1".to_string(),
            specialist_id: "spec-1".to_string(),
            memory_type: MemoryType::Lesson,
            title: "test".to_string(),
            description: "test".to_string(),
            context: "test".to_string(),
            confidence: Confidence::Medium,
            relevance_score: 0.8,
            usage_count: 0,
            tags: vec![],
            related_memories: vec![],
            created_at: Utc::now() - Duration::days(35),
            updated_at: Utc::now(),
            source: MemorySource::Experience,
        };

        assert!(manager.should_archive(&old_entry));
    }

    #[test]
    fn test_should_not_archive_new_entry() {
        let config = ArchivalConfig {
            archive_threshold_days: 30,
            ..Default::default()
        };
        let manager = MemoryArchiveManager::new(config);

        let new_entry = MemoryEntry {
            id: "new-1".to_string(),
            specialist_id: "spec-1".to_string(),
            memory_type: MemoryType::Lesson,
            title: "test".to_string(),
            description: "test".to_string(),
            context: "test".to_string(),
            confidence: Confidence::Medium,
            relevance_score: 0.8,
            usage_count: 0,
            tags: vec![],
            related_memories: vec![],
            created_at: Utc::now() - Duration::days(5),
            updated_at: Utc::now(),
            source: MemorySource::Experience,
        };

        assert!(!manager.should_archive(&new_entry));
    }

    #[test]
    fn test_cleanup_policy_manager_creation() {
        let policy = CleanupPolicy::default();
        let manager = PolicyBasedCleanupManager::new(policy);
        assert!(!manager.needs_cleanup(50000));
        assert!(manager.needs_cleanup(95000));
    }
}
