/// Specialist Memory - persistent memory entries for federation specialists.
///
/// Each specialist can store and retrieve memory entries that persist
/// across sessions and restarts.

use serde::{Deserialize, Serialize};

/// Type of memory entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// A single memory entry stored by a specialist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub specialist_id: String,
    pub title: String,
    pub description: String,
    pub memory_type: MemoryType,
    pub confidence: f32,
    pub created_at: u64,
    pub last_accessed: u64,
    pub access_count: u32,
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
        }
    }
}
