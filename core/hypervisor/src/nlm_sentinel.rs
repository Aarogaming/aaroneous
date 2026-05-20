use candle_core::{Device, Tensor};
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum IntentTier {
    Local,      // SLM only, no network
    Bounded,    // Whitelisted APIs only
    Remote,     // High-level synthesis, cloud API
    Violation,  // Illegal/Unsafe
}

pub struct NlmSentinel {
    device: Device,
    // In a full implementation, we'd have a small linear model here
    // weights: Tensor,
}

impl NlmSentinel {
    pub fn new() -> Result<Self> {
        Ok(Self { device: Device::Cpu })
    }

    /// Fast classification of an intent vector or raw string
    pub fn classify_intent(&self, intent_text: &str) -> IntentTier {
        let text = intent_text.to_lowercase();
        
        // 1. HARD GUARDRAILS (The Sentinel)
        if text.contains("paywall") || text.contains("private system") || text.contains("crack") {
            return IntentTier::Violation;
        }

        // 2. TIER ALLOCATION
        if text.contains("google") || text.contains("research") || text.contains("search") || text.contains("arxiv") {
            return IntentTier::Bounded;
        }

        if text.contains("synthesize large") || text.contains("complex strategy") {
            return IntentTier::Remote;
        }

        // Default to local/private processing
        IntentTier::Local
    }

    /// Verifies if a given enzyme has the permission to execute the detected intent
    pub fn verify_permissions(&self, intent: IntentTier, enzyme_tier: u8) -> bool {
        let required = match intent {
            IntentTier::Local => 0,
            IntentTier::Bounded => 1,
            IntentTier::Remote => 2,
            IntentTier::Violation => 255,
        };
        
        enzyme_tier >= required
    }
}
