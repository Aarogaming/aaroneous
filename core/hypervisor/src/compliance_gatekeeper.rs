use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct ComplianceGatekeeper;

impl ComplianceGatekeeper {
    /// Evaluates if a URL is safe to ingest based on robots.txt and local policy.
    pub async fn check_url(&self, url: &str) -> Result<bool> {
        println!("[Gatekeeper] Validating legal compliance for: {}", url);
        
        // 1. Robots.txt Simulation (Fast native check)
        if self.is_blacklisted(url) {
            return Ok(false);
        }

        // 2. Protocol Enforcement (HTTPS only)
        if !url.starts_with("https://") {
            return Ok(false);
        }

        Ok(true)
    }

    fn is_blacklisted(&self, url: &str) -> bool {
        let blacklist = vec![
            "linkedin.com",
            "facebook.com",
            "twitter.com",
            "instagram.com",
            "quora.com"
        ];
        blacklist.iter().any(|domain| url.contains(domain))
    }

    /// Scans extracted metadata for restrictive licenses (CC-BY-NC, etc.)
    pub fn evaluate_license(&self, head_content: &str) -> u8 {
        if head_content.contains("no-ai") || head_content.contains("no-robot") {
            return 2; // Private/Strict
        }
        if head_content.contains("CC BY-NC") {
            return 1; // Restricted
        }
        0 // Public
    }
}
