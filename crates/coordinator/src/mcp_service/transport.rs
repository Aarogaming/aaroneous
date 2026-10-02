use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Transport trait - abstraction for different communication protocols
#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    /// Start the transport
    async fn start(&mut self) -> Result<(), String>;

    /// Stop the transport
    async fn stop(&mut self) -> Result<(), String>;

    /// Check if transport is running
    fn is_running(&self) -> bool;

    /// Get transport type name
    fn transport_type(&self) -> &str;
}

/// HTTP/REST transport
pub struct HttpTransport {
    _addr: SocketAddr,
    running: std::sync::atomic::AtomicBool,
}

impl HttpTransport {
    /// Create new HTTP transport
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            _addr: addr,
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl Transport for HttpTransport {
    async fn start(&mut self) -> Result<(), String> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn transport_type(&self) -> &str {
        "http"
    }
}

/// WebSocket transport
pub struct WebSocketTransport {
    _port: u16,
    running: std::sync::atomic::AtomicBool,
}

impl WebSocketTransport {
    /// Create new WebSocket transport
    pub fn new(port: u16) -> Self {
        Self {
            _port: port,
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl Transport for WebSocketTransport {
    async fn start(&mut self) -> Result<(), String> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn transport_type(&self) -> &str {
        "websocket"
    }
}

/// NATS transport
pub struct NatsTransport {
    _url: String,
    running: std::sync::atomic::AtomicBool,
}

impl NatsTransport {
    /// Create new NATS transport
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            _url: url.into(),
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl Transport for NatsTransport {
    async fn start(&mut self) -> Result<(), String> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn transport_type(&self) -> &str {
        "nats"
    }
}

/// MCP Protocol transport
pub struct McpTransport {
    _mode: McpMode,
    running: std::sync::atomic::AtomicBool,
}

/// MCP connection mode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum McpMode {
    /// Standard input/output
    Stdio,
    /// HTTP server
    Http { addr: SocketAddr },
    /// Server-sent events
    Sse { addr: SocketAddr },
}

impl McpTransport {
    /// Create new MCP transport
    pub fn new(mode: McpMode) -> Self {
        Self {
            _mode: mode,
            running: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl Transport for McpTransport {
    async fn start(&mut self) -> Result<(), String> {
        self.running
            .store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn transport_type(&self) -> &str {
        "mcp"
    }
}

/// Service health status
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Service status
    pub status: String,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Active transports
    pub active_transports: Vec<String>,
    /// Request count
    pub request_count: u64,
    /// Error count
    pub error_count: u64,
    /// Timestamp
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_transport() {
        let mut transport = HttpTransport::new(([127, 0, 0, 1], 8080).into());
        assert!(!transport.is_running());

        transport.start().await.unwrap();
        assert!(transport.is_running());
        assert_eq!(transport.transport_type(), "http");

        transport.stop().await.unwrap();
        assert!(!transport.is_running());
    }

    #[tokio::test]
    async fn test_websocket_transport() {
        let mut transport = WebSocketTransport::new(8443);
        transport.start().await.unwrap();
        assert_eq!(transport.transport_type(), "websocket");
        transport.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_nats_transport() {
        let mut transport = NatsTransport::new("nats://localhost:4222");
        transport.start().await.unwrap();
        assert_eq!(transport.transport_type(), "nats");
        transport.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_mcp_transport() {
        let mut transport = McpTransport::new(McpMode::Stdio);
        transport.start().await.unwrap();
        assert_eq!(transport.transport_type(), "mcp");
        transport.stop().await.unwrap();
    }
}
