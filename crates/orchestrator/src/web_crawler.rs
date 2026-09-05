use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use crate::memory_pipeline::EpisodicInsertionPipeline;

/// SEMANTIC-08: Headless Web Crawler
/// Fetches URLs in the background and silently embeds their textual content into Memory.
pub struct WebCrawler {
    pipeline: Arc<EpisodicInsertionPipeline>,
    client: Client,
}

impl WebCrawler {
    pub fn new(pipeline: Arc<EpisodicInsertionPipeline>) -> Self {
        Self {
            pipeline,
            client: Client::new(),
        }
    }

    /// Spawns an async task to fetch the URL, extract text, and insert it.
    pub fn ingest_url_background(&self, url: impl Into<String>) {
        let url = url.into();
        let pipeline = self.pipeline.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            if let Ok(response) = client.get(&url).send().await {
                if let Ok(html) = response.text().await {
                    // Extremely primitive stripping of HTML tags for raw text extraction
                    // In a production environment, use the scraper crate or readability.
                    let raw_text = html
                        .replace("<", " <")
                        .replace(">", "> ");
                    
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
    }
}