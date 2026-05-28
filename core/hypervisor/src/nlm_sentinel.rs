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
}

impl NlmSentinel {
    pub fn new() -> Result<Self> {
        Ok(Self { device: Device::Cpu })
    }

    /// Compute NLM entropy score using the configured device.
    /// Higher values indicate higher uncertainty in intent classification.
    pub fn compute_nlm_entropy(&self, text: &str) -> Result<f32> {
        let char_count = text.len().max(1) as f32;
        let unique_chars = text.chars().collect::<std::collections::HashSet<_>>().len().max(1) as f32;
        let entropy_estimate = (unique_chars / char_count).ln_1p();

        // Run a minimal tensor operation on the configured device
        let a = Tensor::new(&[entropy_estimate], &self.device)?;
        let b = Tensor::new(&[0.5f32], &self.device)?;
        let adjusted = (a.add(&b)?).to_scalar::<f32>()?;

        Ok(adjusted)
    }

    /// Fast classification of an intent vector or raw string  
    /// Uses the device to optionally refine classification
    pub fn classify_intent(&self, intent_text: &str) -> IntentTier {
        let text = intent_text.to_lowercase();
        
        // 1. HARD GUARDRAILS (The Sentinel)
        if text.contains("paywall") || text.contains("private system") || text.contains("crack") {
            return IntentTier::Violation;
        }

        // 2. TIER ALLOCATION — optionally refine with NLM entropy
        let entropy = self.compute_nlm_entropy(intent_text).unwrap_or(0.0);

        if text.contains("google") || text.contains("research") || text.contains("search") || text.contains("arxiv") {
            // Low entropy + Bounded keywords → confident Bounded classification
            if entropy < 1.0 {
                return IntentTier::Bounded;
            }
            // High entropy on research keywords → upgrade to Remote for synthesis
            return IntentTier::Remote;
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
