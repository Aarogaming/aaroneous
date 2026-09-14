use reqwest::Client;
use std::sync::Arc;
use tokio::sync::Semaphore;
use crate::memory_pipeline::EpisodicInsertionPipeline;

/// Error type for crawler backpressure and load shedding scenarios.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CrawlerBackpressureError {
    #[error("Crawler service saturated: concurrency limit reached")]
    ServiceSaturated,
}

/// SEMANTIC-08: Headless Web Crawler with Bounded Concurrency & Load Shedding
/// Fetches URLs in the background and silently embeds their textual content into Memory.
pub struct WebCrawler {
    pipeline: Arc<EpisodicInsertionPipeline>,
    client: Client,
    semaphore: Arc<Semaphore>,
}

impl WebCrawler {
    /// Creates a new `WebCrawler` with a default concurrency ceiling of 16 background tasks.
    pub fn new(pipeline: Arc<EpisodicInsertionPipeline>) -> Self {
        Self::with_capacity(pipeline, 16)
    }

    /// Creates a new `WebCrawler` with a specified maximum concurrent crawling capacity.
    pub fn with_capacity(pipeline: Arc<EpisodicInsertionPipeline>, max_concurrent_crawls: usize) -> Self {
        Self {
            pipeline,
            client: Client::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent_crawls)),
        }
    }

    /// Returns the number of currently available concurrency permits.
    pub fn capacity(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Returns `true` if all concurrency permits are actively leased.
    pub fn is_full(&self) -> bool {
        self.semaphore.available_permits() == 0
    }

    /// Attempts to ingest a URL in the background, enforcing bounded concurrency.
    /// If permits are exhausted, returns `CrawlerBackpressureError::ServiceSaturated`
    /// to shed excess load immediately without consuming worker threads.
    pub fn try_ingest_url_background(&self, url: impl Into<String>) -> Result<(), CrawlerBackpressureError> {
        let permit = self
            .semaphore
            .clone()
            .try_acquire_owned()
            .map_err(|_| CrawlerBackpressureError::ServiceSaturated)?;

        let url = url.into();
        let pipeline = self.pipeline.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let _permit = permit;
            if let Ok(response) = client.get(&url).send().await {
                if let Ok(html) = response.text().await {
                    // Primitive stripping of HTML tags for raw text extraction
                    let raw_text = html
                        .replace('<', " <")
                        .replace('>', "> ");
                    
                    let mut clean_text = String::new();
                    let mut in_tag = false;
                    for c in raw_text.chars() {
                        if c == '<' { in_tag = true; continue; }
                        if c == '>' { in_tag = false; continue; }
                        if !in_tag { clean_text.push(c); }
                    }

                    let _ = pipeline.embed_and_insert(&clean_text, &format!("#web_scrape {}", url));
                }
            }
        });

        Ok(())
    }

    /// Spawns an async task to fetch the URL, extract text, and insert it.
    /// Drops requests gracefully under backpressure for backward compatibility.
    pub fn ingest_url_background(&self, url: impl Into<String>) {
        let _ = self.try_ingest_url_background(url);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use compute::episodic_memory::EpisodicMemoryFabric;

    #[test]
    fn test_web_crawler_creation() {
        let fabric = Arc::new(EpisodicMemoryFabric::default());
        let pipeline = Arc::new(EpisodicInsertionPipeline::new(fabric));
        let crawler = WebCrawler::new(pipeline);
        assert_eq!(crawler.capacity(), 16);
        assert!(!crawler.is_full());
    }

    #[test]
    fn test_web_crawler_bounded_concurrency() {
        let fabric = Arc::new(EpisodicMemoryFabric::default());
        let pipeline = Arc::new(EpisodicInsertionPipeline::new(fabric));
        let crawler = WebCrawler::with_capacity(pipeline, 1);

        assert_eq!(crawler.capacity(), 1);
        assert!(!crawler.is_full());

        // Acquire the single permit
        let permit = crawler.semaphore.clone().try_acquire_owned();
        assert!(permit.is_ok());
        assert!(crawler.is_full());
        assert_eq!(crawler.capacity(), 0);

        // Next request must be shed with ServiceSaturated
        let result = crawler.try_ingest_url_background("http://localhost:9999/dummy");
        assert_eq!(result, Err(CrawlerBackpressureError::ServiceSaturated));

        drop(permit);
        assert!(!crawler.is_full());
        assert_eq!(crawler.capacity(), 1);
    }
}