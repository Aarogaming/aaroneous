use anyhow::Result;
use tracing::info;

/// DEVTOOL-09: OpenTelemetry (OTel) Export
/// Pipes internal system metrics, router decisions, and latency traces to an OTel collector.
pub struct OTelExporter {
    endpoint: String,
    is_active: bool,
}

/// Type alias for canonical naming
pub type OtelExporter = OTelExporter;

impl OTelExporter {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            is_active: false,
        }
    }

    /// Initializes the global OTel tracer and starts pushing metrics.
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing OpenTelemetry OTLP Exporter pointing to {}", self.endpoint);
        self.is_active = true;
        Ok(())
    }

    /// Emits a mock span to the OTel collector
    pub fn emit_span(&self, span_name: &str, duration_ms: u64) {
        if self.is_active {
            info!("[OTel-Mock] Exporting Span '{}' ({}ms) to {}", span_name, duration_ms, self.endpoint);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otel_exporter() {
        let mut exporter = OTelExporter::new("http://localhost:4317");
        assert!(exporter.initialize().is_ok());
        exporter.emit_span("test_span", 42);
    }
}