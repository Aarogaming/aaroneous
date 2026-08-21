use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// High-dimensional representation of a piece of memory.
pub type Embedding = Vec<f32>;

/// Minimum similarity score [0..1] required to surface a memory during recall.
/// Below this, memories are considered too noisy for immediate context injection.
const MIN_RECALL_SCORE: f32 = 0.15;

/// A single discrete memory event stored in the vector bank.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedMemory {
    pub id: String,
    pub sovereign: String, // Which specialist/shard owns this memory
    pub content: String,
    pub category: String,
    pub timestamp: u64,
    pub embedding: Embedding,
}

/// Distributed Memory Store for the Federation.
///
/// Implements a lightweight RAG (Retrieval-Augmented Generation) system using
/// TF-IDF vectorization and cosine similarity. It provides sovereign-isolated
/// memory recall, allowing Merlin, Odin, and others to remember past actions.
pub struct EmbeddingStore {
    // Sovereign -> [Memories]
    store: HashMap<String, Vec<EmbeddedMemory>>,
    // TF-IDF state
    vocab: HashMap<String, usize>,
    idf: Vec<f32>,
    df: Vec<usize>,
    doc_count: usize,
    dim: usize,
}

impl EmbeddingStore {
    const MIN_RECALL_SCORE: f32 = MIN_RECALL_SCORE;

    pub fn new(dim: usize) -> Self {
        Self {
            store: HashMap::new(),
            vocab: HashMap::new(),
            idf: Vec::new(),
            df: Vec::new(),
            doc_count: 0,
            dim,
        }
    }

    /// Add a new text memory to the store for a specific sovereign.
    pub fn store_text(&mut self, id: &str, sovereign: &str, text: &str, category: &str) {
        self.update_vocab(text);
        let embedding = self.vectorize(text);

        let memory = EmbeddedMemory {
            id: id.to_string(),
            sovereign: sovereign.to_string(),
            content: text.to_string(),
            category: category.to_string(),
            timestamp: now_ms(),
            embedding,
        };

        self.store
            .entry(sovereign.to_string())
            .or_default()
            .push(memory);
    }

    /// Query the store for similar memories.
    ///
    /// If `sovereign_filter` is provided, only that specialist's memories are searched.
    /// If `category_filter` is provided, only that specific memory type is returned.
    pub fn query_text(
        &self,
        text: &str,
        top_k: usize,
        sovereign_filter: Option<&str>,
        category_filter: Option<&str>,
    ) -> Vec<SimilarMemory> {
        let query_vec = self.vectorize(text);
        let mut results = Vec::new();

        let sources = if let Some(s) = sovereign_filter {
            self.store.get(s).map(|v| vec![v]).unwrap_or_default()
        } else {
            self.store.values().collect::<Vec<_>>()
        };

        for memories in sources {
            for m in memories {
                if let Some(cat) = category_filter
                    && m.category != cat
                {
                    continue;
                }

                let score = cosine_similarity(&query_vec, &m.embedding);
                results.push(SimilarMemory {
                    memory: m.clone(),
                    score,
                });
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(top_k);
        results
    }

    /// Recall past experiences as a formatted string for LLM context injection.
    pub fn recall_for(&self, sovereign: &str, intent: &str, top_k: usize) -> String {
        let results = self.query_text(intent, top_k, Some(sovereign), None);
        // Filter below threshold — prevents low-relevance memories from polluting context
        let relevant: Vec<_> = results
            .iter()
            .filter(|r| r.score >= Self::MIN_RECALL_SCORE)
            .collect();

        if relevant.is_empty() {
            return String::new();
        }

        let mut buf = format!(
            "Relevant memories for '{}':\n",
            intent.chars().take(60).collect::<String>()
        );
        for (i, r) in relevant.iter().enumerate() {
            buf.push_str(&format!(
                "{}. [{}] {} (relevance: {:.2})\n",
                i + 1,
                r.memory.category,
                r.memory.content.chars().take(120).collect::<String>(),
                r.score
            ));
        }
        buf
    }

    /// Record a memory into the store.
    pub fn record_memory(&mut self, entry: crate::specialist_memory::MemoryEntry) {
        let id = entry.id;
        let sovereign = entry.specialist_id;
        let content = format!("{}: {}", entry.title, entry.description);
        let category = format!("{:?}", entry.memory_type);
        self.store_text(&id, &sovereign, &content, &category);
    }

    /// Access to internal memories for MCP pull/list
    pub fn memories(&self) -> &HashMap<String, Vec<EmbeddedMemory>> {
        &self.store
    }

    /// Total memory count across all sovereigns.
    pub fn total_count(&self) -> usize {
        self.store.values().map(|v| v.len()).sum()
    }

    /// Memory count for a specific sovereign.
    pub fn count_for(&self, sovereign: &str) -> usize {
        self.store.get(sovereign).map(|v| v.len()).unwrap_or(0)
    }

    // ── Vectorization (TF-IDF) ─────────────────────────────────────────────

    fn update_vocab(&mut self, text: &str) {
        self.doc_count += 1;
        let terms: std::collections::HashSet<String> = tokenize(text).into_iter().collect();
        for term in terms {
            let next_id = self.vocab.len();
            let idx = *self.vocab.entry(term).or_insert(next_id);
            if idx == next_id {
                self.df.push(1);
            } else {
                self.df[idx] += 1;
            }
        }
        // Real IDF: ln(N / (df + 1)) — +1 smoothing prevents division by zero.
        // Common words across many documents get low weight; rare domain-specific
        // terms ("security", "injection", "borrow") get high weight.
        let n = self.doc_count as f32;
        let vocab_size = self.vocab.len();
        self.idf.resize(vocab_size, 0.0);
        for i in 0..vocab_size {
            let df = self.df[i] as f32;
            self.idf[i] = (n / (df + 1.0)).ln().max(0.1);
        }
    }

    fn vectorize(&self, text: &str) -> Embedding {
        let dim = self.dim.min(self.vocab.len().max(self.dim));
        let mut vec = vec![0.0f32; dim];
        let terms = tokenize(text);
        let total = terms.len().max(1) as f32;

        // Term frequency
        let mut tf: HashMap<usize, f32> = HashMap::new();
        for term in &terms {
            if let Some(&idx) = self.vocab.get(term)
                && idx < dim
            {
                *tf.entry(idx).or_insert(0.0) += 1.0 / total;
            }
        }

        // TF-IDF weight
        for (idx, freq) in tf {
            let idf = self.idf.get(idx).copied().unwrap_or(1.0);
            vec[idx] = freq * idf;
        }

        // L2 normalize
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut vec {
                *x /= norm;
            }
        }
        vec
    }
}

// ── SimilarMemory ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SimilarMemory {
    pub memory: EmbeddedMemory,
    pub score: f32, // cosine similarity [0, 1]
}

// ── Math ──────────────────────────────────────────────────────────────────────

/// Cosine similarity between two vectors. Returns 0.0 if either is zero.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Simple word tokenizer: lowercase, alphanumeric tokens, min length 3.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 3)
        .map(|s| s.to_lowercase())
        .collect()
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Persistence ───────────────────────────────────────────────────────────────

/// A serializable snapshot of the EmbeddingStore used for disk persistence.
///
/// The vocab, idf, and embeddings are all stored so the store can be fully
/// restored across restarts — no re-indexing needed.
#[derive(serde::Serialize, serde::Deserialize)]
struct EmbeddingStoreSnapshot {
    memories: std::collections::HashMap<String, Vec<EmbeddedMemory>>,
    vocab: std::collections::HashMap<String, usize>,
    idf: Vec<f32>,
    df: Vec<usize>,
    doc_count: usize,
    dim: usize,
    saved_at: u64,
    total_memories: usize,
}

impl EmbeddingStore {
    /// Save to a specific path (used by GenericSpecialist for per-sovereign memory).
    pub fn save_to_disk_at(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if self.total_count() == 0 {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let snapshot = EmbeddingStoreSnapshot {
            memories: self.store.clone(),
            vocab: self.vocab.clone(),
            idf: self.idf.clone(),
            df: self.df.clone(),
            doc_count: self.doc_count,
            dim: self.dim,
            saved_at: now_ms(),
            total_memories: self.total_count(),
        };
        std::fs::write(path, serde_json::to_string(&snapshot)?)?;
        Ok(())
    }

    /// Load from a specific path (used by GenericSpecialist for per-sovereign memory).
    pub fn load_from_disk_at(path: &std::path::Path, dim: usize) -> Self {
        if !path.exists() {
            return Self::new(dim);
        }
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(_) => return Self::new(dim),
        };
        let snapshot: EmbeddingStoreSnapshot = match serde_json::from_str(&data) {
            Ok(s) => s,
            Err(_) => return Self::new(dim),
        };
        Self {
            store: snapshot.memories,
            vocab: snapshot.vocab,
            idf: snapshot.idf,
            df: snapshot.df,
            doc_count: snapshot.doc_count,
            dim: snapshot.dim,
        }
    }

    /// Save this store to the workspace data directory.
    ///
    /// Called on federation shutdown. The sidecar is loaded back on startup
    /// via `load_from_disk()`, making cross-session RAG memory persistent.
    pub fn save_to_disk(&self) -> anyhow::Result<()> {
        let path = crate::workspace::WorkspacePaths::workspace_root()
            .join("data")
            .join("federation_memory.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let total = self.total_count();
        if total == 0 {
            // Nothing to save — don't overwrite a good sidecar with an empty one
            return Ok(());
        }
        let snapshot = EmbeddingStoreSnapshot {
            memories: self.store.clone(),
            vocab: self.vocab.clone(),
            idf: self.idf.clone(),
            df: self.df.clone(),
            doc_count: self.doc_count,
            dim: self.dim,
            saved_at: now_ms(),
            total_memories: total,
        };
        let json = serde_json::to_string(&snapshot)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a previously saved store from disk.
    ///
    /// Called at federation startup. If the sidecar doesn't exist or is
    /// corrupt, a fresh empty store is returned gracefully.
    pub fn load_from_disk(dim: usize) -> Self {
        let path = crate::workspace::WorkspacePaths::workspace_root()
            .join("data")
            .join("federation_memory.json");
        if !path.exists() {
            return Self::new(dim);
        }
        let data = match std::fs::read_to_string(path) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("EmbeddingStore: failed to read sidecar: {}", e);
                return Self::new(dim);
            }
        };
        let mut snapshot: EmbeddingStoreSnapshot = match serde_json::from_str(&data) {
            Ok(s) => s,
            Err(e) => {
                // Try backward compatibility
                #[derive(serde::Deserialize)]
                struct LegacySnapshot {
                    memories: std::collections::HashMap<String, Vec<EmbeddedMemory>>,
                    vocab: std::collections::HashMap<String, usize>,
                    idf: Vec<f32>,
                    doc_count: usize,
                    dim: usize,
                    saved_at: u64,
                    total_memories: usize,
                }
                if let Ok(ls) = serde_json::from_str::<LegacySnapshot>(&data) {
                    EmbeddingStoreSnapshot {
                        memories: ls.memories,
                        vocab: ls.vocab.clone(),
                        idf: ls.idf,
                        df: vec![1; ls.vocab.len()],
                        doc_count: ls.doc_count,
                        dim: ls.dim,
                        saved_at: ls.saved_at,
                        total_memories: ls.total_memories,
                    }
                } else {
                    tracing::warn!("EmbeddingStore: sidecar parse error: {}", e);
                    return Self::new(dim);
                }
            }
        };

        if snapshot.df.len() < snapshot.vocab.len() {
            snapshot.df.resize(snapshot.vocab.len(), 1);
        }

        tracing::info!(
            "EmbeddingStore: restored {} memories from sidecar (saved at {})",
            snapshot.total_memories,
            snapshot.saved_at,
        );
        Self {
            store: snapshot.memories,
            vocab: snapshot.vocab,
            idf: snapshot.idf,
            df: snapshot.df,
            doc_count: snapshot.doc_count,
            dim: snapshot.dim,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let mut store = EmbeddingStore::new(256);

        store.store_text("m1", "Merlin", "Rust async runtime tokio", "research");
        store.store_text(
            "m2",
            "Merlin",
            "Python async asyncio coroutines",
            "research",
        );
        store.store_text(
            "m3",
            "Ariel",
            "UI design patterns component grid",
            "execution",
        );

        let results = store.query_text("Rust async programming", 2, Some("Merlin"), None);
        assert!(!results.is_empty(), "Should return at least one result");
        // m1 should score higher than m2 for a Rust query
        assert_eq!(results[0].memory.id, "m1");
    }

    #[test]
    fn test_sovereign_isolation() {
        let mut store = EmbeddingStore::new(256);
        store.store_text("a", "Merlin", "knowledge synthesis research", "research");
        store.store_text("b", "Argus", "security vulnerability scanning", "execution");

        let merlin_results = store.query_text("security scan", 5, Some("Merlin"), None);
        // Merlin has no security memory — should return her closest but not Argus's
        for r in &merlin_results {
            assert_eq!(r.memory.sovereign, "Merlin");
        }
    }

    #[test]
    fn test_recall_for_format() {
        let mut store = EmbeddingStore::new(256);
        store.store_text(
            "r1",
            "Odin",
            "task decomposition planning workflow",
            "execution",
        );
        store.store_text(
            "r2",
            "Odin",
            "guild coordination sovereign assignment",
            "execution",
        );

        let recall = store.recall_for("Odin", "task decomposition planning workflow", 2);
        assert!(
            recall.contains("Relevant memories"),
            "recall_for should produce formatted output"
        );
        assert!(
            recall.contains("relevance:"),
            "should include relevance scores"
        );
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);
    }
}
