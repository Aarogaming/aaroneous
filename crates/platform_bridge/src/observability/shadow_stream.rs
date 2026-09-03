// crates/platform_bridge/src/observability/shadow_stream.rs
//! Non-Intrusive 3rd-Person Shadow Distillation Tap.
//!
//! Provides zero-latency stream mirroring of model and user message traffic
//! (Gemini, Copilot, local endpoints) without interrupting the client connection,
//! buffering correlated prompt-response pairs for `.si` cartridge distillation.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// An intercepted and mirrored conversation exchange.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowExchange {
    pub exchange_id: u64,
    pub timestamp_ms: u64,
    pub provider: String,
    pub prompt: String,
    pub response: String,
    pub verified_outcome: Option<bool>,
}

/// In-memory ring buffer holding mirrored exchanges for offline distillation.
pub struct ShadowDistillationTap {
    is_active: AtomicBool,
    exchange_counter: AtomicU64,
    buffer: RwLock<VecDeque<ShadowExchange>>,
    max_capacity: usize,
}

impl ShadowDistillationTap {
    /// Creates a new shadow tap with a maximum memory ring-buffer capacity.
    pub fn new(max_capacity: usize) -> Self {
        Self {
            is_active: AtomicBool::new(true),
            exchange_counter: AtomicU64::new(1),
            buffer: RwLock::new(VecDeque::with_capacity(max_capacity)),
            max_capacity,
        }
    }

    /// Default 1,024-exchange in-memory buffer.
    pub fn default_tap() -> Self {
        Self::new(1024)
    }

    /// Transparently intercepts a message exchange, storing a shadow copy while allowing flow to proceed.
    pub fn intercept_exchange(&self, provider: impl Into<String>, prompt: &str, response: &str) -> Result<u64> {
        if !self.is_active.load(Ordering::Acquire) {
            bail!("Shadow tap is disengaged");
        }

        let exchange_id = self.exchange_counter.fetch_add(1, Ordering::Relaxed);
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let exchange = ShadowExchange {
            exchange_id,
            timestamp_ms: ts,
            provider: provider.into(),
            prompt: prompt.to_string(),
            response: response.to_string(),
            verified_outcome: None,
        };

        let mut buf = self.buffer.write();
        if buf.len() >= self.max_capacity {
            buf.pop_front();
        }
        buf.push_back(exchange);

        Ok(exchange_id)
    }

    /// Stamps whether the exchange yielded a successful outcome (e.g., code compiled or action succeeded).
    pub fn stamp_verification(&self, exchange_id: u64, outcome: bool) -> bool {
        let mut buf = self.buffer.write();
        if let Some(item) = buf.iter_mut().find(|e| e.exchange_id == exchange_id) {
            item.verified_outcome = Some(outcome);
            true
        } else {
            false
        }
    }

    /// Extracts all verified successful exchanges for distillation into an `.si` habit stack.
    pub fn extract_verified_training_corpus(&self) -> Vec<ShadowExchange> {
        self.buffer
            .read()
            .iter()
            .filter(|e| e.verified_outcome == Some(true))
            .cloned()
            .collect()
    }

    /// Total count of buffered exchanges.
    pub fn buffered_count(&self) -> usize {
        self.buffer.read().len()
    }

    /// Disengages the shadow tap.
    pub fn disengage(&self) {
        self.is_active.store(false, Ordering::Release);
    }

    /// Engages the shadow tap.
    pub fn engage(&self) {
        self.is_active.store(true, Ordering::Release);
    }

    /// Whether the tap is actively observing.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Acquire)
    }
}

impl Default for ShadowDistillationTap {
    fn default() -> Self {
        Self::default_tap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_distillation_tap_lifecycle() {
        let tap = ShadowDistillationTap::new(10);
        assert!(tap.is_active());
        assert_eq!(tap.buffered_count(), 0);

        // 1. Intercept exchanges
        let id1 = tap
            .intercept_exchange("gemini-2.5", "Write a rust function", "fn add(a: i32, b: i32) -> i32 { a + b }")
            .unwrap();
        let id2 = tap
            .intercept_exchange("copilot", "Fix syntax error", "use std::sync::Arc;")
            .unwrap();

        assert_eq!(tap.buffered_count(), 2);

        // 2. Stamp verification (only id2 is verified successful)
        assert!(tap.stamp_verification(id2, true));
        assert!(tap.stamp_verification(id1, false));

        // 3. Extract verified training corpus
        let corpus = tap.extract_verified_training_corpus();
        assert_eq!(corpus.len(), 1);
        assert_eq!(corpus[0].exchange_id, id2);
        assert_eq!(corpus[0].provider, "copilot");

        // 4. Disengage
        tap.disengage();
        assert!(!tap.is_active());
        assert!(tap.intercept_exchange("claude", "prompt", "reply").is_err());
    }
}
