// src/gateway.rs

use anyhow::Result;
use async_trait::async_trait;
use runtime_monitor::Trigger;
use core::hypervisor::state::RingBuffer;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

// Import the LLMProvider trait from the hypervisor crate.
use a_run::llm::providers::*;

/// Wrapper around an LLMProvider that performs HTTP requests via reqwest and
/// emits a `Trigger` into the runtime monitor ring buffer after each successful call.
pub struct LlmGateway<P> {
    provider: P,
    client: reqwest::Client,
    // Shared ring buffer for telemetry triggers.
    trigger_buffer: Arc<Mutex<RingBuffer<Trigger, 256>>>,
}

impl<P> LlmGateway<P>
where
    P: LLMProvider,
{
    /// Construct a new gateway with the given provider.
    /// The runtime monitor ring buffer must be supplied by the caller.
    pub fn new(provider: P, trigger_buffer: Arc<Mutex<RingBuffer<Trigger, 256>>>) -> Self {
        let client = reqwest::Client::new();
        Self { provider, client, trigger_buffer }
    }

    /// Internal helper to emit a telemetry trigger.
    async fn emit_trigger(&self, tag: &str) {
        let mut trigger = Trigger::default();
        let bytes = tag.as_bytes();
        let len = bytes.len().min(trigger.context.len());
        trigger.context[..len].copy_from_slice(&bytes[..len]);
        trigger.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or_default();
        let mut buf = self.trigger_buffer.lock().await;
        let _ = buf.try_push(trigger);
        info!("Emitted telemetry trigger: {}", tag);
    }
}

#[async_trait]
impl<P> LlmGateway<P>
where
    P: LLMProvider + Send + Sync,
{
    /// Forward chat request to the underlying provider and emit telemetry.
    pub async fn chat(&self, system_prompt: &str, user_message: &str, domain: &str) -> Result<String> {
        let res = self.provider.chat(system_prompt, user_message, domain).await;
        match &res {
            Ok(_) => self.emit_trigger("chat_success").await,
            Err(e) => error!(error = %e, "chat failure"),
        }
        res
    }

    /// Forward embed request to the underlying provider and emit telemetry.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let res = self.provider.embed(text).await;
        match &res {
            Ok(_) => self.emit_trigger("embed_success").await,
            Err(e) => error!(error = %e, "chat failure"),
        }
        res
    }
}
