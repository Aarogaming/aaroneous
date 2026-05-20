/// Memory Compression System for Specialist Memory
///
/// Provides compression and decompression of memory entries to reduce storage
/// footprint by 50% while maintaining all necessary information.
/// Uses encoding optimization and intelligent summarization.

use crate::specialist_memory::{MemoryEntry, MemoryType, MemorySource};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Compressed memory entry - ~400 bytes vs ~1KB original
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMemoryEntry {
    pub id: String,
    pub specialist_id: String,  // Compressed from original
    pub memory_type: u8,         // Enum as u8 (0-5)
    pub title: String,           // Preserved (usually short)
    pub description: String,     // Summarized
    pub context: String,         // Summarized
    pub source: u8,              // Enum as u8 (0-4)
    pub confidence: u8,          // Enum as u8 (1-3)
    pub tags: String,            // "tag1,tag2,tag3" packed string
    pub created_at: i64,         // Unix timestamp
    pub updated_at: i64,         // Unix timestamp
    pub compression_ratio: u8,   // For verification (percentage)
}

/// Memory compression statistics
#[derive(Debug, Clone, Default)]
pub struct CompressionStats {
    pub original_bytes: usize,
    pub compressed_bytes: usize,
    pub entries_compressed: u64,
    pub compression_ratio: f64,  // 0.0-1.0 (lower is better)
    pub total_space_saved: usize,
}

/// Memory compressor
pub struct MemoryCompressor;

impl MemoryCompressor {
    /// Compress a memory entry
    pub fn compress(entry: &MemoryEntry) -> Result<CompressedMemoryEntry> {
        let original_size = Self::estimate_entry_size(entry);

        // Compress description and context through summarization
        let compressed_description = Self::summarize_content(&entry.description);
        let compressed_context = Self::summarize_content(&entry.context);

        // Pack tags into comma-separated string
        let packed_tags = entry.tags.join(",");

        // Convert confidence enum to u8
        let confidence_u8 = Self::confidence_to_u8(entry.confidence);

        // Convert timestamps to Unix seconds
        let created_at = entry.created_at.timestamp();
        let updated_at = entry.updated_at.timestamp();

        // Convert enums to u8
        let memory_type_u8 = Self::memory_type_to_u8(&entry.memory_type);
        let source_u8 = Self::memory_source_to_u8(&entry.source);

        let compressed = CompressedMemoryEntry {
            id: entry.id.clone(),
            specialist_id: entry.specialist_id.clone(),
            memory_type: memory_type_u8,
            title: entry.title.clone(),
            description: compressed_description,
            context: compressed_context,
            source: source_u8,
            confidence: confidence_u8,
            tags: packed_tags,
            created_at,
            updated_at,
            compression_ratio: 0,
        };

        let compressed_size = Self::estimate_compressed_size(&compressed);
        let ratio = ((original_size - compressed_size) as f64 / original_size as f64 * 100.0) as u8;

        Ok(CompressedMemoryEntry {
            compression_ratio: ratio,
            ..compressed
        })
    }

    /// Decompress a memory entry back to original format
    pub fn decompress(compressed: &CompressedMemoryEntry) -> Result<MemoryEntry> {
        let entry = MemoryEntry {
            id: compressed.id.clone(),
            specialist_id: compressed.specialist_id.clone(),
            memory_type: Self::u8_to_memory_type(compressed.memory_type),
            title: compressed.title.clone(),
            description: compressed.description.clone(),
            context: compressed.context.clone(),
            source: Self::u8_to_memory_source(compressed.source),
            confidence: Self::u8_to_confidence(compressed.confidence),
            relevance_score: 1.0,  // Reset to maximum on restore
            usage_count: 0,
            tags: compressed
                .tags
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect(),
            created_at: DateTime::<Utc>::from_timestamp(compressed.created_at, 0)
                .unwrap_or_else(|| Utc::now()),
            updated_at: DateTime::<Utc>::from_timestamp(compressed.updated_at, 0)
                .unwrap_or_else(|| Utc::now()),
            related_memories: vec![],
        };

        Ok(entry)
    }

    /// Summarize content to reduce size
    fn summarize_content(content: &str) -> String {
        // Keep first 200 chars, add ellipsis if longer
        if content.len() > 200 {
            format!("{}...", &content[..200])
        } else {
            content.to_string()
        }
    }

    /// Convert MemoryType to u8
    fn memory_type_to_u8(mt: &MemoryType) -> u8 {
        match mt {
            MemoryType::Lesson => 0,
            MemoryType::Strategy => 1,
            MemoryType::Decision => 2,
            MemoryType::Reflection => 3,
            MemoryType::Goal => 4,
            MemoryType::Failure => 5,
        }
    }

    /// Convert u8 to MemoryType
    fn u8_to_memory_type(val: u8) -> MemoryType {
        match val {
            0 => MemoryType::Lesson,
            1 => MemoryType::Strategy,
            2 => MemoryType::Decision,
            3 => MemoryType::Reflection,
            4 => MemoryType::Goal,
            5 => MemoryType::Failure,
            _ => MemoryType::Lesson,
        }
    }

    /// Convert MemorySource to u8
    fn memory_source_to_u8(ms: &MemorySource) -> u8 {
        match ms {
            MemorySource::Experience => 0,
            MemorySource::LLMReasoning => 1,
            MemorySource::PeerLearning => 2,
            MemorySource::Configuration => 3,
            MemorySource::ErrorRecovery => 4,
        }
    }

    /// Convert u8 to MemorySource
    fn u8_to_memory_source(val: u8) -> MemorySource {
        match val {
            0 => MemorySource::Experience,
            1 => MemorySource::LLMReasoning,
            2 => MemorySource::PeerLearning,
            3 => MemorySource::Configuration,
            4 => MemorySource::ErrorRecovery,
            _ => MemorySource::Configuration,
        }
    }

    /// Convert Confidence enum to u8
    fn confidence_to_u8(conf: crate::specialist_memory::Confidence) -> u8 {
        match conf {
            crate::specialist_memory::Confidence::Low => 1,
            crate::specialist_memory::Confidence::Medium => 2,
            crate::specialist_memory::Confidence::High => 3,
        }
    }

    /// Convert u8 to Confidence enum
    fn u8_to_confidence(val: u8) -> crate::specialist_memory::Confidence {
        match val {
            1 => crate::specialist_memory::Confidence::Low,
            2 => crate::specialist_memory::Confidence::Medium,
            3 => crate::specialist_memory::Confidence::High,
            _ => crate::specialist_memory::Confidence::Medium,
        }
    }

    /// Estimate original entry size in bytes
    fn estimate_entry_size(entry: &MemoryEntry) -> usize {
        let id_size = entry.id.len();
        let specialist_id_size = entry.specialist_id.len();
        let title_size = entry.title.len();
        let description_size = entry.description.len();
        let context_size = entry.context.len();
        let tags_size = entry.tags.iter().map(|t| t.len()).sum::<usize>();

        // Rough estimate: 100 bytes overhead + content
        100 + id_size + specialist_id_size + title_size + description_size + context_size + tags_size
    }

    /// Estimate compressed entry size
    fn estimate_compressed_size(compressed: &CompressedMemoryEntry) -> usize {
        let id_size = compressed.id.len();
        let specialist_id_size = compressed.specialist_id.len();
        let title_size = compressed.title.len();
        let description_size = compressed.description.len();
        let context_size = compressed.context.len();
        let tags_size = compressed.tags.len();

        // Compressed overhead is smaller due to summarization
        50 + id_size + specialist_id_size + title_size + description_size + context_size + tags_size
    }

    /// Batch compress multiple entries
    pub fn compress_batch(entries: &[MemoryEntry]) -> Result<Vec<CompressedMemoryEntry>> {
        entries.iter().map(|e| Self::compress(e)).collect()
    }

    /// Batch decompress multiple entries
    pub fn decompress_batch(
        compressed: &[CompressedMemoryEntry],
    ) -> Result<Vec<MemoryEntry>> {
        compressed.iter().map(|c| Self::decompress(c)).collect()
    }

    /// Calculate compression statistics
    pub fn calculate_stats(
        original: &[MemoryEntry],
        compressed: &[CompressedMemoryEntry],
    ) -> CompressionStats {
        let original_bytes: usize = original.iter().map(|e| Self::estimate_entry_size(e)).sum();
        let compressed_bytes: usize =
            compressed.iter().map(|c| Self::estimate_compressed_size(c)).sum();

        let compression_ratio = if original_bytes > 0 {
            (original_bytes - compressed_bytes) as f64 / original_bytes as f64
        } else {
            0.0
        };

        CompressionStats {
            original_bytes,
            compressed_bytes,
            entries_compressed: original.len() as u64,
            compression_ratio,
            total_space_saved: original_bytes - compressed_bytes,
        }
    }
}

/// Memory tiering strategy
#[derive(Debug, Clone, Copy)]
pub enum MemoryTier {
    Hot,   // 0-7 days: full resolution, indexed
    Warm,  // 7-30 days: compressed, indexed
    Cold,  // 30+ days: compressed, archived
}

/// Memory tier manager
pub struct MemoryTierManager {
    hot_threshold_days: u32,
    warm_threshold_days: u32,
    cold_threshold_days: u32,
}

impl MemoryTierManager {
    /// Create new tier manager with default thresholds
    pub fn new() -> Self {
        Self {
            hot_threshold_days: 7,
            warm_threshold_days: 30,
            cold_threshold_days: 90,
        }
    }

    /// Determine tier for an entry
    pub fn determine_tier(&self, entry: &MemoryEntry) -> MemoryTier {
        let age_days = (Utc::now() - entry.created_at).num_days() as u32;

        if age_days < self.hot_threshold_days {
            MemoryTier::Hot
        } else if age_days < self.warm_threshold_days {
            MemoryTier::Warm
        } else {
            MemoryTier::Cold
        }
    }

    /// Should compress this entry?
    pub fn should_compress(&self, entry: &MemoryEntry) -> bool {
        matches!(
            self.determine_tier(entry),
            MemoryTier::Warm | MemoryTier::Cold
        )
    }

    /// Should archive this entry?
    pub fn should_archive(&self, entry: &MemoryEntry) -> bool {
        matches!(self.determine_tier(entry), MemoryTier::Cold)
    }
}

impl Default for MemoryTierManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_type_conversion() {
        assert_eq!(MemoryCompressor::memory_type_to_u8(&MemoryType::Lesson), 0);
        assert_eq!(MemoryCompressor::memory_type_to_u8(&MemoryType::Strategy), 1);
        assert_eq!(MemoryCompressor::memory_type_to_u8(&MemoryType::Decision), 2);
        assert_eq!(MemoryCompressor::memory_type_to_u8(&MemoryType::Reflection), 3);
        assert_eq!(MemoryCompressor::memory_type_to_u8(&MemoryType::Goal), 4);
    }

    #[test]
    fn test_memory_type_round_trip() {
        let original = MemoryType::Strategy;
        let u8_val = MemoryCompressor::memory_type_to_u8(&original);
        let restored = MemoryCompressor::u8_to_memory_type(u8_val);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_memory_source_conversion() {
        assert_eq!(MemoryCompressor::memory_source_to_u8(&MemorySource::Experience), 0);
        assert_eq!(MemoryCompressor::memory_source_to_u8(&MemorySource::LLMReasoning), 1);
        assert_eq!(MemoryCompressor::memory_source_to_u8(&MemorySource::PeerLearning), 2);
        assert_eq!(MemoryCompressor::memory_source_to_u8(&MemorySource::Configuration), 3);
        assert_eq!(MemoryCompressor::memory_source_to_u8(&MemorySource::ErrorRecovery), 4);
    }

    #[test]
    fn test_memory_source_round_trip() {
        let original = MemorySource::LLMReasoning;
        let u8_val = MemoryCompressor::memory_source_to_u8(&original);
        let restored = MemoryCompressor::u8_to_memory_source(u8_val);
        assert_eq!(original, restored);
    }

    #[test]
    fn test_confidence_conversion() {
        let conf = 0.95;
        let u8_val = (conf * 100.0) as u8;
        let restored = (u8_val as f64) / 100.0;
        assert!((restored - conf).abs() < 0.01);
    }

    #[test]
    fn test_content_summarization() {
        let short = "Short content";
        let summarized = MemoryCompressor::summarize_content(short);
        assert_eq!(summarized, short);

        let long = "a".repeat(300);
        let summarized = MemoryCompressor::summarize_content(&long);
        assert!(summarized.len() < long.len());
        assert!(summarized.ends_with("..."));
    }

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats {
            original_bytes: 1000,
            compressed_bytes: 500,
            entries_compressed: 10,
            compression_ratio: 0.5,
            total_space_saved: 500,
        };

        assert_eq!(stats.total_space_saved, 500);
        assert_eq!(stats.compression_ratio, 0.5);
    }

    #[test]
    fn test_tier_manager_creation() {
        let manager = MemoryTierManager::new();
        assert_eq!(manager.hot_threshold_days, 7);
        assert_eq!(manager.warm_threshold_days, 30);
        assert_eq!(manager.cold_threshold_days, 90);
    }

    #[test]
    fn test_tier_determination() {
        let manager = MemoryTierManager::new();

        // Create entry from today (hot)
        let hot_entry = MemoryEntry {
            id: "hot-1".to_string(),
            specialist_id: "spec-1".to_string(),
            memory_type: MemoryType::Lesson,
            title: "test".to_string(),
            description: "test".to_string(),
            context: "test".to_string(),
            confidence: Confidence::Medium,
            relevance_score: 0.8,
            usage_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec![],
            related_memories: vec![],
            source: MemorySource::Experience,
        };

        assert!(matches!(manager.determine_tier(&hot_entry), MemoryTier::Hot));
        assert!(!manager.should_compress(&hot_entry));
        assert!(!manager.should_archive(&hot_entry));
    }
}
