//! crates/platform_bridge/src/web_ingest.rs
//! Web Perception & Safe Ingestion Adapter.
//!
//! Bridges headless browser crawl streams and HTTP web endpoints directly into
//! the Universal SensoryFeedAdapter framework.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::adapters::{NormalizedObservation, SensoryFeedAdapter};

/// Compliance Policy Rule for Web Perception
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebComplianceConfig {
    pub blocked_domains: Vec<String>,
    pub max_payload_bytes: usize,
    pub allow_redirects: bool,
}

impl Default for WebComplianceConfig {
    fn default() -> Self {
        Self {
            blocked_domains: vec![
                "twitter.com".to_string(),
                "facebook.com".to_string(),
                "instagram.com".to_string(),
                "tiktok.com".to_string(),
            ],
            max_payload_bytes: 10 * 1024 * 1024, // 10 MB
            allow_redirects: true,
        }
    }
}

impl WebComplianceConfig {
    pub fn is_domain_permitted(&self, url_or_host: &str) -> bool {
        !self.blocked_domains.iter().any(|b| url_or_host.contains(b))
    }
}

/// Web Ingestion Sensory Adapter
pub struct WebIngestionAdapter {
    name: String,
    compliance: WebComplianceConfig,
    active_target_url: Option<String>,
    observation_sequence: u64,
}

impl WebIngestionAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            compliance: WebComplianceConfig::default(),
            active_target_url: None,
            observation_sequence: 0,
        }
    }

    pub fn with_compliance(name: impl Into<String>, compliance: WebComplianceConfig) -> Self {
        Self {
            name: name.into(),
            compliance,
            active_target_url: None,
            observation_sequence: 0,
        }
    }

    pub fn set_target_url(&mut self, url: impl Into<String>) -> Result<()> {
        let u = url.into();
        if !self.compliance.is_domain_permitted(&u) {
            anyhow::bail!("Web Ingestion Blocked: Domain violates compliance rules: {}", u);
        }
        self.active_target_url = Some(u);
        Ok(())
    }

    pub fn target_url(&self) -> Option<&str> {
        self.active_target_url.as_deref()
    }
}

impl SensoryFeedAdapter for WebIngestionAdapter {
    fn feed_name(&self) -> &str {
        &self.name
    }

    fn sample_observation(&mut self) -> Result<NormalizedObservation> {
        self.observation_sequence += 1;
        let target = self.active_target_url.clone().unwrap_or_else(|| "about:blank".to_string());

        // Emit normalized perception observation
        Ok(NormalizedObservation {
            source_id: self.name.clone(),
            timestamp_us: self.observation_sequence * 1_000_000,
            latent_feature_vector: vec![1.0, 0.0, 0.0, 0.0], // Initialized embedding representation
            metadata_tag: format!("url={},seq={}", target, self.observation_sequence),
        })
    }

    fn is_healthy(&self) -> bool {
        self.active_target_url.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_ingestion_compliance() {
        let config = WebComplianceConfig::default();
        assert!(!config.is_domain_permitted("https://facebook.com/feed"));
        assert!(config.is_domain_permitted("https://docs.rs/tokio"));
    }

    #[test]
    fn test_web_ingestion_adapter_lifecycle() {
        let mut adapter = WebIngestionAdapter::new("WebPerceiver-01");
        assert_eq!(adapter.feed_name(), "WebPerceiver-01");
        assert!(!adapter.is_healthy());

        // Target setting
        assert!(adapter.set_target_url("https://facebook.com").is_err());
        assert!(adapter.set_target_url("https://crates.io").is_ok());
        assert!(adapter.is_healthy());

        let obs = adapter.sample_observation().unwrap();
        assert_eq!(obs.source_id, "WebPerceiver-01");
        assert!(obs.metadata_tag.contains("crates.io"));
    }
}
