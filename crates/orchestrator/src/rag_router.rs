use crate::memory_pipeline::EpisodicInsertionPipeline;
use crate::web_crawler::WebCrawler;
use anyhow::Result;
use std::sync::Arc;
use compute::episodic_memory::EpisodicMemoryFabric;

/// SEMANTIC-10: Dynamic RAG Pipeline
/// Automatically routes semantic queries between the Vector DB (Episodic Fabric),
/// local file system documents, and the headless Web Crawler.
pub struct DynamicRagPipeline {
    memory_fabric: Arc<EpisodicMemoryFabric>,
    web_crawler: WebCrawler,
}

impl DynamicRagPipeline {
    pub fn new(memory_fabric: Arc<EpisodicMemoryFabric>, insertion_pipeline: Arc<EpisodicInsertionPipeline>) -> Self {
        Self {
            memory_fabric,
            web_crawler: WebCrawler::new(insertion_pipeline),
        }
    }

    /// Primary query gateway. Determines the optimal data source for the query context.
    pub fn semantic_search(&self, query: &str) -> Result<Vec<String>> {
        let mut results = Vec::new();

        // 1. Check if the query is a URL (Route to Web Crawler)
        if query.starts_with("http://") || query.starts_with("https://") {
            self.web_crawler.ingest_url_background(query);
            results.push(format!("Dispatched headless crawler to ingest: {}", query));
            return Ok(results);
        }

        // 2. Otherwise route to the HNSW R^256 Episodic Memory Fabric
        // Dummy query vector for now until the local embedding model is wired
        let query_vector = [0.1f32; compute::episodic_memory::LATENT_VECTOR_DIM];
        
        // Return top 5 matches
        let matches = self.memory_fabric.recall_nearest(&query_vector, 5);
        for m in matches {
            results.push(format!("Found Memory Trajectory #{} (Distance: {:.3}, Similarity: {:.3})", m.id, m.distance, m.similarity));
        }

        Ok(results)
    }
}