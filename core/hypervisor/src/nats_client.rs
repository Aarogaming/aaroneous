/// NATS client wrapper for federation event publishing.
///
/// Provides a simple publisher and subscriber over the `async-nats` crate.
/// All operations are best-effort; connection failures are silently tolerated.

use std::sync::Arc;

use tokio::sync::Mutex;

/// Configuration for NATS client.
#[derive(Debug, Clone)]
pub struct NatsClientConfig {
    pub server_url: String,
    pub max_reconnects: usize,
    pub reconnect_delay_ms: u64,
}

impl Default for NatsClientConfig {
    fn default() -> Self {
        Self {
            server_url: "nats://localhost:4222".to_string(),
            max_reconnects: 5,
            reconnect_delay_ms: 1000,
        }
    }
}

/// A NATS publisher for broadcasting federation events.
/// Lazily connects on first publish if not already connected.
pub struct NatsPublisher {
    client: Arc<Mutex<Option<async_nats::Client>>>,
    server_url: String,
}

impl NatsPublisher {
    pub fn new(server_url: &str) -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
            server_url: server_url.to_string(),
        }
    }

    pub fn disconnected() -> Self {
        Self::new("nats://localhost:4222")
    }

    pub async fn publish(&self, subject: &str, data: &[u8]) -> anyhow::Result<()> {
        let subject = subject.to_string();
        let data = data.to_vec();
        let mut guard = self.client.lock().await;
        if guard.is_none() {
            *guard = async_nats::connect(&self.server_url).await.ok();
        }
        if let Some(ref nc) = *guard {
            nc.publish(subject, data.into()).await?;
        }
        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        self.client.lock().await.is_some()
    }
}

/// Shared NATS publisher handle.
pub type SharedNatsPublisher = Arc<NatsPublisher>;
