/// Specialist Memory - persistent memory entries for federation specialists.
///
/// Each specialist can store and retrieve memory entries that persist
/// across sessions and restarts. Enables learning from experience and
/// consultation during task execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Type of memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Factual knowledge learned from execution
    Factual,
    /// Procedural knowledge (how to do something)
    Procedural,
    /// Episodic memory (specific event)
    Episodic,
    /// Relational memory (about other specialists)
    Relational,
    /// Meta-cognitive (about own thinking)
    Metacognitive,
}

impl MemoryType {
    pub fn relevance_to_task(&self, task_type: &str) -> f32 {
        match self {
            MemoryType::Procedural => 0.95,  // Highly relevant for learning how
            MemoryType::Factual => 0.85,     // Good for understanding what
            MemoryType::Episodic => 0.70,    // Moderately relevant - past experience
            MemoryType::Relational => 0.60,  // Helpful for collaboration
            MemoryType::Metacognitive => 0.50, // Generally applicable
        }
    }
}

/// A single memory entry stored by a specialist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub specialist_id: String,
    pub title: String,
    pub description: String,
    pub memory_type: MemoryType,
    pub confidence: f32,          // 0.0-1.0: how confident in this memory
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
    pub tags: Vec<String>,        // For semantic search
}

impl MemoryEntry {
    pub fn new(
        id: String,
        specialist_id: String,
        title: String,
        description: String,
        memory_type: MemoryType,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            id,
            specialist_id,
            title,
            description,
            memory_type,
            confidence: 0.5,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            tags: Vec::new(),
        }
    }

    /// Update access tracking when memory is used
    pub fn record_access(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.last_accessed = now;
        self.access_count += 1;
    }

    /// Calculate relevance score for a query
    pub fn relevance_score(&self, query: &str, task_type: &str) -> f32 {
        let type_relevance = self.memory_type.relevance_to_task(task_type);
        
        // Check title and description for keyword matches
        let title_match = self.title.to_lowercase().contains(&query.to_lowercase());
        let desc_match = self.description.to_lowercase().contains(&query.to_lowercase());
        let tags_match = self.tags.iter().any(|t| t.to_lowercase().contains(&query.to_lowercase()));
        
        let keyword_score = if title_match { 0.9 } 
                          else if desc_match { 0.6 }
                          else if tags_match { 0.7 }
                          else { 0.0 };
        
        // Higher access count = more useful memory
        let recency_factor = (self.access_count as f32 / 100.0).min(1.0);
        
        // Weight: type (40%) + keywords (40%) + recency (20%)
        (type_relevance * 0.4) + (keyword_score * 0.4) + (recency_factor * 0.2)
    }
}

/// Query result when consulting specialist memory
#[derive(Debug, Clone, Serialize)]
pub struct MemoryQueryResult {
    pub entries: Vec<MemoryEntry>,
    pub total_score: f32,
    pub recommendation: String,
}

/// Specialist Memory Store - manages memory for a single specialist
#[derive(Clone)]
pub struct SpecialistMemoryStore {
    specialist_id: String,
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

impl SpecialistMemoryStore {
    /// Create new memory store for specialist
    pub fn new(specialist_id: String) -> Self {
        Self {
            specialist_id,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a memory entry
    pub fn store_memory(&self, mut entry: MemoryEntry) {
        entry.specialist_id = self.specialist_id.clone();
        let mut store = self.entries.write();
        store.insert(entry.id.clone(), entry);
    }

    /// Retrieve a specific memory by ID
    pub fn get_memory(&self, memory_id: &str) -> Option<MemoryEntry> {
        let mut store = self.entries.write();
        if let Some(mut entry) = store.get_mut(memory_id) {
            entry.record_access();
        }
        store.get(memory_id).cloned()
    }

    /// Query memory by keyword and task type
    pub fn query_memory(&self, query: &str, task_type: &str, limit: usize) -> MemoryQueryResult {
        let store = self.entries.read();
        
        let mut results: Vec<_> = store.values()
            .map(|entry| {
                let score = entry.relevance_score(query, task_type);
                (entry.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();
        
        // Sort by relevance score (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Take top N results
        let entries: Vec<MemoryEntry> = results.iter()
            .take(limit)
            .map(|(entry, _)| entry.clone())
            .collect();
        
        let total_score: f32 = results.iter()
            .take(limit)
            .map(|(_, score)| score)
            .sum();
        
        let avg_score = if !entries.is_empty() { 
            total_score / entries.len() as f32 
        } else { 
            0.0 
        };
        
        // Generate recommendation based on results
        let recommendation = if avg_score > 0.8 {
            "High confidence guidance available from past experience".to_string()
        } else if avg_score > 0.5 {
            "Moderate guidance available, proceed with caution".to_string()
        } else if avg_score > 0.0 {
            "Limited relevant experience, review carefully".to_string()
        } else {
            "No relevant memory found, use external expertise".to_string()
        };
        
        MemoryQueryResult {
            entries,
            total_score,
            recommendation,
        }
    }

    /// Get all memories of a specific type
    pub fn get_memories_by_type(&self, memory_type: MemoryType) -> Vec<MemoryEntry> {
        let store = self.entries.read();
        store.values()
            .filter(|e| e.memory_type == memory_type)
            .cloned()
            .collect()
    }

    /// Get most frequently accessed memories (most useful)
    pub fn get_frequently_used(&self, limit: usize) -> Vec<MemoryEntry> {
        let store = self.entries.read();
        let mut entries: Vec<_> = store.values().cloned().collect();
        entries.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        entries.into_iter().take(limit).collect()
    }

    /// Clear all memories
    pub fn clear_memories(&self) {
        self.entries.write().clear();
    }

    /// Get memory store statistics
    pub fn get_stats(&self) -> MemoryStats {
        let store = self.entries.read();
        let total_entries = store.len();
        let total_accesses: u32 = store.values().map(|e| e.access_count).sum();
        let avg_confidence: f32 = if total_entries > 0 {
            store.values().map(|e| e.confidence).sum::<f32>() / total_entries as f32
        } else {
            0.0
        };
        
        let type_counts = {
            let mut counts = HashMap::new();
            for entry in store.values() {
                *counts.entry(entry.memory_type.clone()).or_insert(0) += 1;
            }
            counts
        };
        
        MemoryStats {
            total_entries,
            total_accesses,
            avg_confidence,
            type_counts,
        }
    }
}

/// Statistics about specialist memory
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_accesses: u32,
    pub avg_confidence: f32,
    pub type_counts: HashMap<MemoryType, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "How to parse JSON".to_string(),
            "Use serde_json crate".to_string(),
            MemoryType::Procedural,
        );
        assert_eq!(entry.title, "How to parse JSON");
        assert_eq!(entry.memory_type, MemoryType::Procedural);
        assert_eq!(entry.access_count, 0);
    }

    #[test]
    fn test_memory_relevance_score() {
        let mut entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "JSON parsing".to_string(),
            "Using serde_json for deserialization".to_string(),
            MemoryType::Procedural,
        );
        entry.tags = vec!["json".to_string(), "parsing".to_string()];
        
        let score = entry.relevance_score("json", "parsing_task");
        assert!(score > 0.0);
    }

    #[test]
    fn test_memory_store_operations() {
        let store = SpecialistMemoryStore::new("specialist-1".to_string());
        
        let entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "Test memory".to_string(),
            "Test description".to_string(),
            MemoryType::Factual,
        );
        
        store.store_memory(entry.clone());
        let retrieved = store.get_memory("mem-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().title, "Test memory");
    }

    #[test]
    fn test_memory_query() {
        let store = SpecialistMemoryStore::new("specialist-1".to_string());
        
        let entry1 = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "How to debug errors".to_string(),
            "Use logging and assertions".to_string(),
            MemoryType::Procedural,
        );
        
        let entry2 = MemoryEntry::new(
            "mem-2".to_string(),
            "specialist-1".to_string(),
            "Optimization tips".to_string(),
            "Cache results when possible".to_string(),
            MemoryType::Factual,
        );
        
        store.store_memory(entry1);
        store.store_memory(entry2);
        
        let result = store.query_memory("debug", "debugging_task", 5);
        assert!(!result.entries.is_empty());
    }

    #[test]
    fn test_memory_stats() {
        let store = SpecialistMemoryStore::new("specialist-1".to_string());
        
        let entry = MemoryEntry::new(
            "mem-1".to_string(),
            "specialist-1".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            MemoryType::Procedural,
        );
        
        store.store_memory(entry);
        let stats = store.get_stats();
        assert_eq!(stats.total_entries, 1);
    }
}
