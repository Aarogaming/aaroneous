/// NATS client wrapper for federation event publishing.
///
/// Provides a simple publisher interface over the `nats` crate.

use std::sync::Arc;

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

/// A NATS client for publishing and subscribing.
pub struct NatsClient {
    connection: Option<nats::Connection>,
    config: NatsClientConfig,
}

impl NatsClient {
    pub fn new(config: NatsClientConfig) -> anyhow::Result<Self> {
        let connection = nats::connect(&config.server_url).ok();
        Ok(Self { connection, config })
    }

    pub fn publish(&self, subject: &str, data: &[u8]) -> anyhow::Result<()> {
        if let Some(ref nc) = self.connection {
            nc.publish(subject, data)?;
        }
        Ok(())
    }

    pub fn subscribe(&self, subject: &str) -> anyhow::Result<nats::Subscription> {
        if let Some(ref nc) = self.connection {
            Ok(nc.subscribe(subject)?)
        } else {
            anyhow::bail!("NATS not connected")
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

/// A NATS publisher for broadcasting federation events.
pub struct NatsPublisher {
    connection: Option<nats::Connection>,
}

impl NatsPublisher {
    /// Create a new publisher connected to the given NATS server.
    pub fn new(url: &str) -> anyhow::Result<Self> {
        let connection = nats::connect(url).ok();
        Ok(Self { connection })
    }

    /// Create a disconnected publisher (no-op for all publish calls).
    pub fn disconnected() -> Self {
        Self { connection: None }
    }

    /// Publish a message to the given subject.
    pub fn publish(&self, subject: &str, data: &[u8]) -> anyhow::Result<()> {
        if let Some(ref nc) = self.connection {
            nc.publish(subject, data)?;
        }
        Ok(())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

/// Shared NATS publisher handle.
pub type SharedNatsPublisher = Arc<NatsPublisher>;
