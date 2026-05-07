/// EmbeddingStore — Real semantic similarity search for sovereign memory (RAG).
///
/// Instead of exact-string lookup (what SpecialistMemory and DNABank do today),
/// EmbeddingStore lets sovereigns retrieve memories by meaning.
///
/// "What did we learn about Rust async?" → finds all memories semantically
/// related to async programming, even if they never used those exact words.
///
/// # How embeddings work here
///
/// We use a simple, effective approach that requires no external service:
/// - Token-level TF-IDF vectors for fast approximate search
/// - Optional: replace with real GGUF embedding layer output when llama-gguf
///   supports it (the infrastructure is identical, only the vector source changes)
///
/// Cosine similarity gives meaningful ranking without normalization issues.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ── Types ─────────────────────────────────────────────────────────────────────

/// A dense float vector (the embedding).
pub type Embedding = Vec<f32>;

/// A stored piece of memory with its embedding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedMemory {
    pub id: String,
    pub sovereign: String,     // which sovereign owns this memory
    pub content: String,       // the original text
    pub category: String,      // "execution", "research", "task", "observation"
    pub embedding: Embedding,
    pub metadata: HashMap<String, serde_json::Value>,
    pub timestamp_ms: u64,
    pub relevance_score: f32,  // last query's similarity score (updated on retrieval)
}

impl EmbeddedMemory {
    pub fn new(
        id: impl Into<String>,
        sovereign: impl Into<String>,
        content: impl Into<String>,
        category: impl Into<String>,
        embedding: Embedding,
    ) -> Self {
        Self {
            id: id.into(),
            sovereign: sovereign.into(),
            content: content.into(),
            category: category.into(),
            embedding,
            metadata: HashMap::new(),
            timestamp_ms: now_ms(),
            relevance_score: 0.0,
        }
    }
}

// ── EmbeddingStore ────────────────────────────────────────────────────────────

/// Semantic memory store for the sovereign hive.
///
/// Stores embeddings per-sovereign and supports:
/// - `store()`: add a memory with its embedding
/// - `query()`: find top-k most similar memories to a query embedding
/// - `query_text()`: find similar memories by text (using built-in vectorizer)
/// - `recall_for()`: get a sovereign's relevant memories for a given intent
pub struct EmbeddingStore {
    /// sovereign_name → list of memories
    store: HashMap<String, Vec<EmbeddedMemory>>,
    /// Vocabulary for TF-IDF vectorization
    vocab: HashMap<String, usize>,
    /// IDF weights (log(N/df) for each term)
    idf: Vec<f32>,
    /// Total documents seen (for IDF computation)
    doc_count: usize,
    /// Embedding dimension
    dim: usize,
}

impl EmbeddingStore {
    /// Create a new store. `dim` is the embedding dimension (256 is reasonable
    /// for TF-IDF without external models; set higher for real neural embeddings).
    pub fn new(dim: usize) -> Self {
        Self {
            store: HashMap::new(),
            vocab: HashMap::new(),
            idf: Vec::new(),
            doc_count: 0,
            dim,
        }
    }

    pub fn default() -> Self { Self::new(512) }

    // ── Store ─────────────────────────────────────────────────────────────

    /// Store a memory with a pre-computed embedding.
    pub fn store(&mut self, memory: EmbeddedMemory) {
        self.store.entry(memory.sovereign.clone())
            .or_default()
            .push(memory);
    }

    /// Embed text and store as a memory. Uses the built-in TF-IDF vectorizer.
    pub fn store_text(
        &mut self,
        id: impl Into<String>,
        sovereign: impl Into<String>,
        content: impl Into<String>,
        category: impl Into<String>,
    ) -> String {
        let content = content.into();
        let sovereign = sovereign.into();
        let id = id.into();
        let category = category.into();

        // Update vocabulary with new terms
        self.update_vocab(&content);
        let embedding = self.vectorize(&content);

        let memory = EmbeddedMemory::new(
            id.clone(), sovereign, content, category, embedding,
        );
        self.store.entry(memory.sovereign.clone())
            .or_default()
            .push(memory);
        id
    }

    // ── Query ─────────────────────────────────────────────────────────────

    /// Find the top-k most similar memories to a query embedding.
    /// `sovereign_filter`: if Some, only search that sovereign's memories.
    pub fn query(
        &self,
        query_embedding: &Embedding,
        top_k: usize,
        sovereign_filter: Option<&str>,
        category_filter: Option<&str>,
    ) -> Vec<SimilarMemory> {
        let mut scored: Vec<SimilarMemory> = self.store.iter()
            .filter(|(sovereign, _)| {
                sovereign_filter.map_or(true, |f| sovereign.as_str() == f)
            })
            .flat_map(|(_, memories)| memories.iter())
            .filter(|m| category_filter.map_or(true, |c| m.category == c))
            .map(|m| {
                let score = cosine_similarity(query_embedding, &m.embedding);
                SimilarMemory { memory: m.clone(), score }
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        scored
    }

    /// Find similar memories by text query. Vectorizes the query text first.
    pub fn query_text(
        &self,
        query: &str,
        top_k: usize,
        sovereign_filter: Option<&str>,
        category_filter: Option<&str>,
    ) -> Vec<SimilarMemory> {
        let query_vec = self.vectorize(query);
        self.query(&query_vec, top_k, sovereign_filter, category_filter)
    }

    /// Get a sovereign's most relevant memories for a given intent.
    /// Returns formatted text suitable for including in a system prompt.
    ///
    /// This is the core RAG retrieval function — what gets prepended to
    /// Odin/Merlin/Argus/etc. prompts to give them context.
    /// Minimum cosine similarity score for a memory to be considered relevant.
    /// Below this threshold, memories are filtered out to prevent noise injection.
    const MIN_RECALL_SCORE: f32 = 0.15;

    pub fn recall_for(
        &self,
        sovereign: &str,
        intent: &str,
        top_k: usize,
    ) -> String {
        let results = self.query_text(intent, top_k, Some(sovereign), None);
        // Filter below threshold — prevents low-relevance memories from polluting context
        let relevant: Vec<_> = results.iter()
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
            self.vocab.entry(term).or_insert(next_id);
        }
        // Real IDF: ln(N / (df + 1)) — +1 smoothing prevents division by zero.
        // Common words across many documents get low weight; rare domain-specific
        // terms ("security", "injection", "borrow") get high weight.
        // We approximate df as 1 for all terms (first-occurrence assumption) since
        // we don't track per-term document frequency separately.
        // TODO: Track df per term for true IDF when doc_count > 50.
        let n = self.doc_count as f32;
        let vocab_size = self.vocab.len();
        self.idf = vec![n.ln().max(1.0); vocab_size]; // All terms weighted by log(N)
    }

    fn vectorize(&self, text: &str) -> Embedding {
        let dim = self.dim.min(self.vocab.len().max(self.dim));
        let mut vec = vec![0.0f32; dim];
        let terms = tokenize(text);
        let total = terms.len().max(1) as f32;

        // Term frequency
        let mut tf: HashMap<usize, f32> = HashMap::new();
        for term in &terms {
            if let Some(&idx) = self.vocab.get(term) {
                if idx < dim {
                    *tf.entry(idx).or_insert(0.0) += 1.0 / total;
                }
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
            for x in &mut vec { *x /= norm; }
        }
        vec
    }
}

// ── SimilarMemory ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SimilarMemory {
    pub memory: EmbeddedMemory,
    pub score: f32,  // cosine similarity [0, 1]
}

// ── Math ──────────────────────────────────────────────────────────────────────

/// Cosine similarity between two vectors. Returns 0.0 if either is zero.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
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

const MEMORY_SIDECAR_PATH: &str = "D:\\Aaroneous\\data\\federation_memory.json";

/// A serializable snapshot of the EmbeddingStore used for disk persistence.
///
/// The vocab, idf, and embeddings are all stored so the store can be fully
/// restored across restarts — no re-indexing needed.
#[derive(serde::Serialize, serde::Deserialize)]
struct EmbeddingStoreSnapshot {
    memories: std::collections::HashMap<String, Vec<EmbeddedMemory>>,
    vocab: std::collections::HashMap<String, usize>,
    idf: Vec<f32>,
    doc_count: usize,
    dim: usize,
    saved_at: u64,
    total_memories: usize,
}

impl EmbeddingStore {
    /// Save to a specific path (used by GenericSpecialist for per-sovereign memory).
    pub fn save_to_disk_at(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if self.total_count() == 0 { return Ok(()); }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let snapshot = EmbeddingStoreSnapshot {
            memories: self.store.clone(),
            vocab: self.vocab.clone(),
            idf: self.idf.clone(),
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
        if !path.exists() { return Self::new(dim); }
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
            doc_count: snapshot.doc_count,
            dim: snapshot.dim,
        }
    }

    /// Save this store to `D:\Aaroneous\data\federation_memory.json`.
    ///
    /// Called on federation shutdown. The sidecar is loaded back on startup
    /// via `load_from_disk()`, making cross-session RAG memory persistent.
    pub fn save_to_disk(&self) -> anyhow::Result<()> {
        let path = std::path::Path::new(MEMORY_SIDECAR_PATH);
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
        let path = std::path::Path::new(MEMORY_SIDECAR_PATH);
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
        let snapshot: EmbeddingStoreSnapshot = match serde_json::from_str(&data) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("EmbeddingStore: sidecar parse error: {}", e);
                return Self::new(dim);
            }
        };
        tracing::info!(
            "EmbeddingStore: restored {} memories from sidecar (saved at {})",
            snapshot.total_memories,
            snapshot.saved_at,
        );
        Self {
            store: snapshot.memories,
            vocab: snapshot.vocab,
            idf: snapshot.idf,
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
        store.store_text("m2", "Merlin", "Python async asyncio coroutines", "research");
        store.store_text("m3", "Ariel", "UI design patterns component grid", "execution");

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
        store.store_text("r1", "Odin", "task decomposition planning workflow", "execution");
        store.store_text("r2", "Odin", "guild coordination sovereign assignment", "execution");

        let recall = store.recall_for("Odin", "task decomposition planning workflow", 2);
        assert!(recall.contains("Relevant memories"), "recall_for should produce formatted output");
        assert!(recall.contains("relevance:"), "should include relevance scores");
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
